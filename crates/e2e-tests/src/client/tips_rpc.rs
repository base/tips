use alloy_primitives::{Bytes, TxHash};
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use tips_core::{Bundle, BundleHash};
use uuid::Uuid;

#[derive(Clone)]
pub struct TipsRpcClient {
    client: reqwest::Client,
    url: String,
}

#[derive(Debug, Serialize)]
struct JsonRpcRequest<T> {
    jsonrpc: String,
    method: String,
    params: T,
    id: u64,
}

#[derive(Debug, Deserialize)]
struct JsonRpcResponse<T> {
    #[allow(dead_code)]
    jsonrpc: String,
    #[allow(dead_code)]
    id: u64,
    #[serde(flatten)]
    result: JsonRpcResult<T>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum JsonRpcResult<T> {
    Success { result: T },
    Error { error: JsonRpcError },
}

#[derive(Debug, Deserialize)]
struct JsonRpcError {
    code: i64,
    message: String,
    #[serde(default)]
    data: Option<serde_json::Value>,
}

impl TipsRpcClient {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            url: url.into(),
        }
    }

    async fn call<P: Serialize, R: for<'de> Deserialize<'de>>(
        &self,
        method: &str,
        params: P,
    ) -> Result<R> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
            id: 1,
        };

        let response = self.client.post(&self.url).json(&request).send().await?;

        let rpc_response: JsonRpcResponse<R> = response.json().await?;

        match rpc_response.result {
            JsonRpcResult::Success { result } => Ok(result),
            JsonRpcResult::Error { error } => {
                bail!(
                    "RPC error {}: {} (data: {:?})",
                    error.code,
                    error.message,
                    error.data
                )
            }
        }
    }

    pub async fn send_raw_transaction(&self, signed_tx: Bytes) -> Result<TxHash> {
        let tx_hex = format!("0x{}", hex::encode(&signed_tx));
        self.call("eth_sendRawTransaction", vec![tx_hex]).await
    }

    pub async fn send_bundle(&self, bundle: Bundle) -> Result<BundleHash> {
        self.call("eth_sendBundle", vec![bundle]).await
    }

    pub async fn cancel_bundle(&self, uuid: Uuid) -> Result<bool> {
        let params = serde_json::json!({
            "bundleId": uuid.to_string()
        });
        self.call("eth_cancelBundle", vec![params]).await
    }
}
