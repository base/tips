use crate::auth::generate_jwt_token;
use alloy_primitives::B256;
use alloy_rpc_types_engine::{ExecutionPayloadV3, ForkchoiceUpdated, JwtSecret, PayloadAttributes};
use eyre::Result;
use jsonrpsee::{
    core::client::ClientT,
    http_client::{HttpClient, HttpClientBuilder},
    types::ErrorObjectOwned,
};
use op_alloy_rpc_types_engine::OpPayloadAttributes;
use serde_json::Value;
use std::{sync::Arc, time::Duration};
use tokio::sync::RwLock;
use tracing::{error, info, warn};

/// Information extracted from the last newPayload call, used to construct
/// synthetic payload attributes for shadow building when op-node sends FCU
/// without attributes (Boost Sync).
#[derive(Clone, Default)]
pub struct LastPayloadInfo {
    pub timestamp: u64,
    pub prev_randao: B256,
    pub fee_recipient: alloy_primitives::Address,
    pub gas_limit: u64,
    pub eip_1559_params: Option<alloy_primitives::B64>,
    pub parent_beacon_block_root: B256,
}

/// A pass-through proxy between op-node and the shadow builder that:
/// 1. Logs all Engine API requests and responses
/// 2. Injects synthetic payload attributes to trigger shadow building when
///    FCU arrives without attributes (non-sequencer Boost Sync scenario)
/// 3. Suppresses payload_id from responses when using injected attributes
///    so op-node doesn't know shadow building occurred
#[derive(Clone)]
pub struct ShadowBuilderProxy {
    pub builder_client: Arc<RwLock<HttpClient>>,
    builder_url: String,
    jwt_secret: JwtSecret,
    timeout_ms: u64,
    pub last_payload_info: Arc<RwLock<Option<LastPayloadInfo>>>,
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

    /// Handle engine_forkchoiceUpdatedV3 with synthetic attribute injection.
    ///
    /// When op-node sends FCU without payload attributes (params_count=1), this
    /// indicates a Boost Sync call to update chain state. In non-sequencer mode,
    /// we inject synthetic payload attributes based on the last received newPayload
    /// to trigger shadow building. The payload_id returned by the builder is
    /// suppressed before returning to op-node.
    ///
    /// When FCU has payload attributes (params_count=2), pass through unchanged.
    pub async fn handle_fcu(&self, params_vec: Vec<Value>) -> Result<Value, ErrorObjectOwned> {
        let has_payload_attrs = params_vec.len() == 2;

        info!(
            method = "engine_forkchoiceUpdatedV3",
            params_count = params_vec.len(),
            params = ?params_vec,
            "JSON-RPC request (original from op-node)"
        );

        let mut injected_attrs = false;
        let modified_params = if !has_payload_attrs {
            let last_info = self.last_payload_info.read().await;
            if let Some(info) = last_info.as_ref() {
                let timestamp = info.timestamp + 2;
                let synthetic_attrs = OpPayloadAttributes {
                    payload_attributes: PayloadAttributes {
                        timestamp,
                        prev_randao: info.prev_randao,
                        suggested_fee_recipient: info.fee_recipient,
                        withdrawals: Some(vec![]),
                        parent_beacon_block_root: Some(info.parent_beacon_block_root),
                    },
                    transactions: None,
                    no_tx_pool: Some(false),
                    gas_limit: Some(info.gas_limit),
                    eip_1559_params: info.eip_1559_params,
                    min_base_fee: None,
                };

                info!(
                    timestamp,
                    gas_limit = info.gas_limit,
                    "Injected synthetic payload attributes to trigger shadow building"
                );

                let mut new_params = params_vec.clone();
                new_params.push(serde_json::to_value(synthetic_attrs).unwrap());
                injected_attrs = true;
                new_params
            } else {
                info!("No last payload info available, passing FCU through unchanged");
                params_vec
            }
        } else {
            info!("FCU already has payload attributes, passing through unchanged");
            params_vec
        };

        if injected_attrs {
            info!(
                method = "engine_forkchoiceUpdatedV3",
                params_count = modified_params.len(),
                "JSON-RPC request (modified, sent to builder)"
            );
        }

        let client = self.builder_client.read().await;
        let mut response: ForkchoiceUpdated = client
            .request("engine_forkchoiceUpdatedV3", modified_params)
            .await
            .map_err(|e| {
                warn!(
                    method = "engine_forkchoiceUpdatedV3",
                    error = %e,
                    "JSON-RPC request failed"
                );
                ErrorObjectOwned::owned(-32603, e.to_string(), None::<()>)
            })?;
        drop(client);

        let builder_payload_id = response.payload_id;

        info!(
            method = "engine_forkchoiceUpdatedV3",
            payload_status = ?response.payload_status.status,
            payload_id = ?builder_payload_id,
            injected_attrs,
            "JSON-RPC response (from builder)"
        );

        if injected_attrs && builder_payload_id.is_some() {
            info!(
                payload_id = ?builder_payload_id,
                "Suppressing payload_id from injected attributes before returning to op-node"
            );
            response.payload_id = None;
        }

        let response_value = serde_json::to_value(response).unwrap();

        info!(
            method = "engine_forkchoiceUpdatedV3",
            response = ?response_value,
            "JSON-RPC response (returned to op-node)"
        );

        Ok(response_value)
    }

    /// Handle engine_newPayloadV4 and capture payload info for synthetic attributes.
    ///
    /// Extracts key information from the payload (timestamp, gas_limit, prevRandao,
    /// feeRecipient, EIP-1559 params, parent beacon block root) and stores it for
    /// use in constructing synthetic payload attributes when future FCU calls arrive
    /// without attributes.
    ///
    /// The request and response are passed through unchanged to/from the builder.
    pub async fn handle_new_payload(
        &self,
        params_vec: Vec<Value>,
    ) -> Result<Value, ErrorObjectOwned> {
        info!(
            method = "engine_newPayloadV4",
            params_count = params_vec.len(),
            params = ?params_vec,
            "JSON-RPC request"
        );

        if params_vec.len() >= 3 {
            if let Ok(payload) = serde_json::from_value::<ExecutionPayloadV3>(params_vec[0].clone())
            {
                let parent_beacon_block_root = if let Some(root_val) = params_vec.get(2) {
                    serde_json::from_value(root_val.clone()).ok()
                } else {
                    None
                };

                if let Some(parent_beacon_block_root) = parent_beacon_block_root {
                    let timestamp = payload.payload_inner.payload_inner.timestamp;
                    let prev_randao = payload.payload_inner.payload_inner.prev_randao;
                    let fee_recipient = payload.payload_inner.payload_inner.fee_recipient;
                    let gas_limit = payload.payload_inner.payload_inner.gas_limit;
                    let extra_data = &payload.payload_inner.payload_inner.extra_data;

                    let eip_1559_params = if extra_data.len() >= 9 {
                        Some(alloy_primitives::B64::from_slice(&extra_data[1..9]))
                    } else {
                        Some(alloy_primitives::B64::from_slice(&[
                            0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x08,
                        ]))
                    };

                    *self.last_payload_info.write().await = Some(LastPayloadInfo {
                        timestamp,
                        prev_randao,
                        fee_recipient,
                        gas_limit,
                        eip_1559_params,
                        parent_beacon_block_root,
                    });

                    info!(
                        timestamp,
                        gas_limit, "Captured payload info for future synthetic attributes"
                    );
                }
            }
        }

        let client = self.builder_client.read().await;
        let result: Value = client
            .request("engine_newPayloadV4", params_vec)
            .await
            .map_err(|e| {
                warn!(
                    method = "engine_newPayloadV4",
                    error = %e,
                    "JSON-RPC request failed"
                );
                ErrorObjectOwned::owned(-32603, e.to_string(), None::<()>)
            })?;

        info!(
            method = "engine_newPayloadV4",
            response = ?result,
            "JSON-RPC response"
        );

        Ok(result)
    }
}
