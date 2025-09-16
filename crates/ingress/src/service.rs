use alloy_primitives::{B256, Bytes};
use alloy_provider::network::eip2718::Decodable2718;
use alloy_provider::{Provider, RootProvider};
use alloy_rpc_types_mev::{EthBundleHash, EthCancelBundle, EthSendBundle};
use jsonrpsee::types::ErrorObject;
use jsonrpsee::{
    core::{RpcResult, async_trait},
    proc_macros::rpc,
};
use op_alloy_consensus::OpTxEnvelope;
use tips_datastore::BundleDatastore;
use tracing::{info, warn};

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

pub struct IngressService<Store> {
    provider: RootProvider,
    datastore: Store,
    dual_write_mempool: bool,
}

impl<Store> IngressService<Store> {
    pub fn new(provider: RootProvider, datastore: Store, dual_write_mempool: bool) -> Self {
        Self {
            provider,
            datastore,
            dual_write_mempool,
        }
    }
}

#[async_trait]
impl<Store> IngressApiServer for IngressService<Store>
where
    Store: BundleDatastore + Sync + Send + 'static,
{
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
        let envelope = OpTxEnvelope::decode_2718_exact(tx.iter().as_slice())
            .map_err(|_e| ErrorObject::owned(10, "todo", Some(1)))?;

        // TODO: Validation and simulation

        // TODO: parallelize DB and mempool setup
        let bundle = EthSendBundle {
            txs: vec![tx.clone()],
            block_number: 0,
            min_timestamp: None,
            max_timestamp: None,
            reverting_tx_hashes: vec![envelope.tx_hash()],
            ..Default::default()
        };

        let result = self
            .datastore
            .insert_bundle(bundle)
            .await
            .map_err(|_e| ErrorObject::owned(11, "todo", Some(2)))?;

        info!(message="inserted singleton bundle", uuid=%result, txn_hash=%envelope.tx_hash());

        if self.dual_write_mempool {
            // If we also want to dual write to the mempool
            let response = self
                .provider
                .send_raw_transaction(tx.iter().as_slice())
                .await;

            match response {
                Ok(_) => {
                    info!(message = "sent transaction to the mempool", hash=%envelope.tx_hash());
                }
                Err(e) => {
                    warn!(
                        message = "Failed to send raw transaction to mempool",
                        error = %e
                    );
                }
            }
        }

        Ok(envelope.tx_hash())
    }
}
