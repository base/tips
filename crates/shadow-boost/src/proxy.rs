use crate::auth::generate_jwt_token;
use alloy_primitives::B256;
use alloy_rpc_types_engine::{
    ExecutionPayloadV3, ForkchoiceState, ForkchoiceUpdated, JwtSecret, PayloadAttributes,
    PayloadId, PayloadStatus, PayloadStatusEnum,
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

#[derive(Clone, Default)]
struct LastPayloadInfo {
    timestamp: u64,
    prev_randao: B256,
    fee_recipient: alloy_primitives::Address,
    gas_limit: u64,
    eip_1559_params: Option<alloy_primitives::B64>,
    last_block_hash: B256,
}

#[derive(Clone)]
pub struct ShadowBuilderProxy {
    pub builder_client: Arc<RwLock<HttpClient>>,
    builder_url: String,
    jwt_secret: JwtSecret,
    timeout_ms: u64,
    fetch_timeout_ms: u64,
    last_payload_info: Arc<RwLock<Option<LastPayloadInfo>>>,
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
            last_payload_info: Arc::new(RwLock::new(None)),
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
        let has_original_attrs = payload_attributes.is_some();

        info!(
            head_hash = %fork_choice_state.head_block_hash,
            safe_hash = %fork_choice_state.safe_block_hash,
            finalized_hash = %fork_choice_state.finalized_block_hash,
            has_attrs = has_original_attrs,
            "Received FCU from op-node"
        );

        let injected_attrs = if !has_original_attrs {
            info!("No payload attributes provided - injecting synthetic attributes to trigger shadow building");
            true
        } else {
            info!("FCU has payload attributes - will rewrite no_tx_pool to trigger building");
            false
        };

        let modified_attrs = match payload_attributes {
            Some(mut attrs) => {
                let original_no_tx_pool = attrs.no_tx_pool.unwrap_or(false);
                attrs.no_tx_pool = Some(false);
                info!(
                    timestamp = attrs.payload_attributes.timestamp,
                    original_no_tx_pool,
                    modified_no_tx_pool = false,
                    "Rewrote no_tx_pool in existing payload attributes"
                );
                Some(attrs)
            }
            None => {
                let last_info = self.last_payload_info.read().await;
                if let Some(info) = last_info.as_ref() {
                    use std::time::{SystemTime, UNIX_EPOCH};
                    let current_timestamp = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs();

                    let timestamp = current_timestamp.max(info.timestamp + 2);

                    info!(
                        timestamp,
                        gas_limit = info.gas_limit,
                        ?info.eip_1559_params,
                        "Created synthetic payload attributes from last newPayload"
                    );

                    Some(OpPayloadAttributes {
                        payload_attributes: PayloadAttributes {
                            timestamp,
                            prev_randao: info.prev_randao,
                            suggested_fee_recipient: info.fee_recipient,
                            withdrawals: Some(vec![]),
                            parent_beacon_block_root: Some(B256::ZERO),
                        },
                        transactions: None,
                        no_tx_pool: Some(false),
                        gas_limit: Some(info.gas_limit),
                        eip_1559_params: info.eip_1559_params,
                        min_base_fee: None,
                    })
                } else {
                    warn!("No payload attributes and no previous newPayload - cannot build shadow block yet");
                    None
                }
            }
        };

        info!("Sending FCU with modified attributes to shadow builder");

        let client = self.builder_client.read().await;
        let response: ForkchoiceUpdated = ClientT::request(
            &*client,
            "engine_forkchoiceUpdatedV3",
            (fork_choice_state, modified_attrs),
        )
        .await
        .map_err(|e| {
            error!(error = %e, "Shadow builder FCU failed");
            ErrorObjectOwned::owned(-32603, e.to_string(), None::<()>)
        })?;
        drop(client);

        if let Some(payload_id) = response.payload_id {
            info!(
                %payload_id,
                injected_attrs,
                "Shadow builder initiated block building - spawning fetch task"
            );

            let builder_client = self.builder_client.clone();
            let timeout_ms = self.fetch_timeout_ms;
            tokio::spawn(async move {
                info!(%payload_id, "Waiting 1s before fetching shadow block");
                tokio::time::sleep(Duration::from_millis(1000)).await;

                let fetch_result = tokio::time::timeout(
                    Duration::from_millis(timeout_ms),
                    Self::fetch_and_log_payload(builder_client, payload_id),
                )
                .await;

                match fetch_result {
                    Ok(Ok(_)) => info!(%payload_id, "Successfully fetched and logged shadow block"),
                    Ok(Err(e)) => warn!(%payload_id, error = %e, "Failed to fetch shadow block"),
                    Err(_) => warn!(%payload_id, timeout_ms, "Timeout fetching shadow block"),
                }
            });
        } else {
            warn!(
                injected_attrs,
                "Shadow builder FCU returned Valid but no payload_id - block building may not have started"
            );
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
        let tx_count = payload.payload_inner.payload_inner.transactions.len();
        let gas_used = payload.payload_inner.payload_inner.gas_used;
        let gas_limit = payload.payload_inner.payload_inner.gas_limit;
        let timestamp = payload.payload_inner.payload_inner.timestamp;
        let prev_randao = payload.payload_inner.payload_inner.prev_randao;
        let fee_recipient = payload.payload_inner.payload_inner.fee_recipient;
        let extra_data = &payload.payload_inner.payload_inner.extra_data;

        info!(
            %block_hash,
            block_number,
            tx_count,
            gas_used,
            "Received newPayload from op-node - storing payload info and forwarding to shadow builder"
        );

        let eip_1559_params = if extra_data.len() >= 9 {
            let params_bytes = &extra_data[1..9];
            Some(alloy_primitives::B64::from_slice(params_bytes))
        } else {
            Some(alloy_primitives::B64::from_slice(&[
                0x00, 0x00, 0x00, 0x08,
                0x00, 0x00, 0x00, 0x08,
            ]))
        };

        let is_duplicate = {
            let last_info = self.last_payload_info.read().await;
            last_info.as_ref().map_or(false, |info| info.last_block_hash == block_hash)
        };

        if is_duplicate {
            info!(
                %block_hash,
                block_number,
                "Skipping duplicate newPayload (already forwarded to shadow builder)"
            );
            return Ok(PayloadStatus::new(PayloadStatusEnum::Valid, None));
        }

        *self.last_payload_info.write().await = Some(LastPayloadInfo {
            timestamp,
            prev_randao,
            fee_recipient,
            gas_limit,
            eip_1559_params,
            last_block_hash: block_hash,
        });

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
                Ok(status) => info!(
                    %block_hash,
                    block_number,
                    status = ?status.status,
                    "Shadow builder accepted newPayload"
                ),
                Err(e) => error!(
                    %block_hash,
                    block_number,
                    error = %e,
                    "Shadow builder rejected newPayload"
                ),
            }
        });

        info!(
            %block_hash,
            block_number,
            "Returning Valid status to op-node immediately"
        );

        Ok(PayloadStatus::new(PayloadStatusEnum::Valid, None))
    }
}
