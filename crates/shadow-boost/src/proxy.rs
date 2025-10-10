use crate::auth::generate_jwt_token;
use alloy_primitives::B256;
use alloy_rpc_types_engine::{
    ExecutionPayloadV3, ForkchoiceState, ForkchoiceUpdated, JwtSecret, PayloadId, PayloadStatus,
    PayloadStatusEnum,
};
use eyre::Result;
use jsonrpsee::{
    core::client::ClientT,
    http_client::{HttpClient, HttpClientBuilder},
    types::ErrorObjectOwned,
};
use op_alloy_rpc_types_engine::{OpExecutionPayloadEnvelopeV3, OpPayloadAttributes};
use std::{sync::Arc, time::Duration};
use tokio::sync::RwLock;
use tracing::{error, info, warn};

#[derive(Clone)]
pub struct ShadowBuilderProxy {
    pub builder_client: Arc<RwLock<HttpClient>>,
    builder_url: String,
    jwt_secret: JwtSecret,
    timeout_ms: u64,
    fetch_timeout_ms: u64,
}

impl ShadowBuilderProxy {
    fn create_client(
        builder_url: &str,
        jwt_secret: &JwtSecret,
        timeout_ms: u64,
    ) -> Result<HttpClient> {
        let token = generate_jwt_token(jwt_secret);
        let auth_value = format!("Bearer {}", token);

        let mut headers = http::HeaderMap::new();
        headers.insert(
            http::header::AUTHORIZATION,
            http::HeaderValue::from_str(&auth_value)
                .map_err(|e| eyre::eyre!("Invalid auth header: {}", e))?,
        );

        let client = HttpClientBuilder::new()
            .set_headers(headers)
            .request_timeout(Duration::from_millis(timeout_ms))
            .build(builder_url)?;

        Ok(client)
    }

    pub fn new(builder_url: &str, jwt_secret: JwtSecret, timeout_ms: u64) -> Result<Self> {
        let client = Self::create_client(builder_url, &jwt_secret, timeout_ms)?;

        let proxy = Self {
            builder_client: Arc::new(RwLock::new(client)),
            builder_url: builder_url.to_string(),
            jwt_secret,
            timeout_ms,
            fetch_timeout_ms: timeout_ms,
        };

        proxy.start_token_refresh_task();

        Ok(proxy)
    }

    fn start_token_refresh_task(&self) {
        let builder_client = self.builder_client.clone();
        let builder_url = self.builder_url.clone();
        let jwt_secret = self.jwt_secret;
        let timeout_ms = self.timeout_ms;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            interval.tick().await;

            loop {
                interval.tick().await;

                match Self::create_client(&builder_url, &jwt_secret, timeout_ms) {
                    Ok(new_client) => {
                        *builder_client.write().await = new_client;
                        info!("Refreshed JWT token for builder client");
                    }
                    Err(e) => {
                        error!(error = %e, "Failed to refresh JWT token");
                    }
                }
            }
        });
    }

    pub async fn handle_fcu(
        &self,
        fork_choice_state: ForkchoiceState,
        payload_attributes: Option<OpPayloadAttributes>,
    ) -> Result<ForkchoiceUpdated, ErrorObjectOwned> {
        info!(
            head_hash = %fork_choice_state.head_block_hash,
            has_attrs = payload_attributes.is_some(),
            "Received FCU from op-node"
        );

        let modified_attrs = payload_attributes.map(|mut attrs| {
            let original_no_tx_pool = attrs.no_tx_pool.unwrap_or(false);
            attrs.no_tx_pool = Some(false);

            info!(
                original_no_tx_pool,
                modified_no_tx_pool = false,
                "Rewriting FCU attributes to trigger building"
            );

            attrs
        });

        let client = self.builder_client.read().await;
        let response: ForkchoiceUpdated = ClientT::request(
            &*client,
            "engine_forkchoiceUpdatedV3",
            (fork_choice_state, modified_attrs),
        )
        .await
        .map_err(|e| {
            error!(error = %e, "Builder FCU failed");
            ErrorObjectOwned::owned(-32603, e.to_string(), None::<()>)
        })?;
        drop(client);

        if let Some(payload_id) = response.payload_id {
            info!(%payload_id, "Builder initiated block building");

            let builder_client = self.builder_client.clone();
            let timeout_ms = self.fetch_timeout_ms;
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(1000)).await;

                let fetch_result = tokio::time::timeout(
                    Duration::from_millis(timeout_ms),
                    Self::fetch_and_log_payload(builder_client, payload_id),
                )
                .await;

                match fetch_result {
                    Ok(Ok(_)) => info!(%payload_id, "Successfully fetched shadow block"),
                    Ok(Err(e)) => warn!(%payload_id, error = %e, "Failed to fetch shadow block"),
                    Err(_) => warn!(%payload_id, "Timeout fetching shadow block"),
                }
            });
        }

        Ok(ForkchoiceUpdated::new(PayloadStatus::new(
            PayloadStatusEnum::Valid,
            None,
        )))
    }

    async fn fetch_and_log_payload(
        builder_client: Arc<RwLock<HttpClient>>,
        payload_id: PayloadId,
    ) -> Result<()> {
        info!(%payload_id, "Fetching shadow block from builder");

        let client = builder_client.read().await;
        let payload: OpExecutionPayloadEnvelopeV3 =
            ClientT::request(&*client, "engine_getPayloadV3", (payload_id,)).await?;
        drop(client);

        let block_hash = payload
            .execution_payload
            .payload_inner
            .payload_inner
            .block_hash;
        let block_number = payload
            .execution_payload
            .payload_inner
            .payload_inner
            .block_number;
        let gas_used = payload
            .execution_payload
            .payload_inner
            .payload_inner
            .gas_used;
        let tx_count = payload
            .execution_payload
            .payload_inner
            .payload_inner
            .transactions
            .len();
        let block_value = payload.block_value;

        info!(
            %payload_id,
            %block_hash,
            block_number,
            gas_used,
            tx_count,
            %block_value,
            "Shadow block built successfully"
        );

        Ok(())
    }

    pub async fn handle_new_payload(
        &self,
        payload: ExecutionPayloadV3,
        versioned_hashes: Vec<B256>,
        parent_beacon_block_root: B256,
    ) -> Result<PayloadStatus, ErrorObjectOwned> {
        let block_hash = payload.payload_inner.payload_inner.block_hash;
        let block_number = payload.payload_inner.payload_inner.block_number;

        info!(
            %block_hash,
            block_number,
            "Forwarding newPayload to builder"
        );

        let builder_client = self.builder_client.clone();
        tokio::spawn(async move {
            let client = builder_client.read().await;
            let result: Result<PayloadStatus, _> = ClientT::request(
                &*client,
                "engine_newPayloadV3",
                (payload, versioned_hashes, parent_beacon_block_root),
            )
            .await;
            drop(client);

            match result {
                Ok(status) => info!("Builder accepted newPayload: {:?}", status.status),
                Err(e) => warn!(error = %e, "Builder rejected newPayload"),
            }
        });

        Ok(PayloadStatus::new(PayloadStatusEnum::Valid, None))
    }
}
