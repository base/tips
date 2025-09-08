use alloy_primitives::{B256, Bytes};
use alloy_provider::{Provider, RootProvider};
use alloy_rpc_types_mev::{EthBundleHash, EthCancelBundle, EthSendBundle};
use jsonrpsee::{
    core::{RpcResult, async_trait},
    proc_macros::rpc,
    types::ErrorObjectOwned,
};
use tracing::warn;

#[rpc(server, namespace = "eth")]
pub trait IngressApi {
    /// `eth_sendBundle` can be used to send your bundles to the builder.
    #[method(name = "sendBundle")]
    async fn send_bundle(&self, bundle: EthSendBundle) -> RpcResult<EthBundleHash>;

    /// `eth_cancelBundle` is used to prevent a submitted bundle from being included on-chain.
    #[method(name = "cancelBundle")]
    async fn cancel_bundle(&self, request: EthCancelBundle) -> RpcResult<()>;

    /// Handler for: `eth_sendRawTransaction`
    #[method(name = "sendRawTransaction")]
    async fn send_raw_transaction(&self, tx: Bytes) -> RpcResult<B256>;
}

pub struct IngressService {
    provider: RootProvider,
}

impl IngressService {
    pub fn new(provider: RootProvider) -> Self {
        Self { provider }
    }
}

#[async_trait]
impl IngressApiServer for IngressService {
    async fn send_bundle(&self, _bundle: EthSendBundle) -> RpcResult<EthBundleHash> {
        warn!(
            message = "TODO: implement send_bundle",
            method = "send_bundle"
        );
        todo!("implement send_bundle")
    }

    async fn cancel_bundle(&self, _request: EthCancelBundle) -> RpcResult<()> {
        warn!(
            message = "TODO: implement cancel_bundle",
            method = "cancel_bundle"
        );
        todo!("implement cancel_bundle")
    }

    async fn send_raw_transaction(&self, tx: Bytes) -> RpcResult<B256> {
        let response = self
            .provider
            .send_raw_transaction(tx.iter().as_slice())
            .await;

        match response {
            Ok(r) => Ok(*r.tx_hash()),
            Err(e) => {
                warn!(
                    message = "Failed to send raw transaction to mempool",
                    method = "send_raw_transaction",
                    error = %e
                );
                Err(ErrorObjectOwned::owned(
                    -32000,
                    format!("Failed to send transaction: {}", e),
                    None::<()>,
                ))
            }
        }
    }
}
