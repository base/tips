use crate::validation::{AccountInfoLookup, validate_tx};
use alloy_consensus::transaction::SignerRecoverable;
use alloy_primitives::{B256, Bytes};
use alloy_provider::{Provider, RootProvider, network::eip2718::Decodable2718};
use alloy_rpc_types_mev::{EthBundleHash, EthCancelBundle, EthSendBundle};
use jsonrpsee::{
    core::{RpcResult, async_trait},
    proc_macros::rpc,
    types::ErrorObject,
};
use op_alloy_consensus::OpTxEnvelope;
use op_alloy_network::Optimism;
use op_revm::l1block::L1BlockInfo;
use reth_rpc_eth_types::EthApiError;
use tracing::{info, warn};

use crate::queue::QueuePublisher;

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

pub struct IngressService<Queue> {
    provider: RootProvider<Optimism>,
    dual_write_mempool: bool,
    queue: Queue,
}

impl<Queue> IngressService<Queue> {
    pub fn new(provider: RootProvider<Optimism>, dual_write_mempool: bool, queue: Queue) -> Self {
        Self {
            provider,
            dual_write_mempool,
            queue,
        }
    }
}

#[async_trait]
impl<Queue> IngressApiServer for IngressService<Queue>
where
    Queue: QueuePublisher + Sync + Send + 'static,
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

    async fn send_raw_transaction(&self, data: Bytes) -> RpcResult<B256> {
        if data.is_empty() {
            return Err(EthApiError::EmptyRawTransactionData.into_rpc_err());
        }

        let envelope = OpTxEnvelope::decode_2718_exact(data.iter().as_slice())
            .map_err(|_| EthApiError::FailedToDecodeSignedTransaction.into_rpc_err())?;

        let transaction = envelope
            .clone()
            .try_into_recovered()
            .map_err(|_| EthApiError::FailedToDecodeSignedTransaction.into_rpc_err())?;

        // TODO: implement provider to fetch l1 block info
        let mut l1_block_info = L1BlockInfo::default();

        let block_number = self
            .provider
            .get_block_number()
            .await
            .map_err(|e| ErrorObject::owned(11, e.to_string(), Some(2)))?;
        let block = self
            .provider
            .get_block_by_number(block_number.into())
            .await
            .map_err(|e| ErrorObject::owned(11, e.to_string(), Some(2)))?;
        if let Some(block) = block {
            let body = block
                .into_full_block(block.transactions.into_transactions_vec())
                .into_block_body_unchecked();
            let l1_block_info = reth_optimism_evm::extract_l1_info(&body);
        }


        let account = self
            .provider
            .fetch_account_info(transaction.signer())
            .await
            .map_err(|e| ErrorObject::owned(11, e.to_string(), Some(2)))?;
        validate_tx(account, &transaction, &data, &mut l1_block_info)
            .await
            .map_err(|e| ErrorObject::owned(11, e.to_string(), Some(2)))?;

        // TODO: parallelize DB and mempool setup

        let bundle = EthSendBundle {
            txs: vec![data.clone()],
            block_number: 0,
            min_timestamp: None,
            max_timestamp: None,
            reverting_tx_hashes: vec![transaction.tx_hash()],
            ..Default::default()
        };

        // queue the bundle
        let sender = transaction.signer();
        if let Err(e) = self.queue.publish(&bundle, sender).await {
            warn!(message = "Failed to publish Queue::enqueue_bundle", sender = %sender, error = %e);
        }

        info!(message="queued singleton bundle", txn_hash=%transaction.tx_hash());

        if self.dual_write_mempool {
            let response = self
                .provider
                .send_raw_transaction(data.iter().as_slice())
                .await;

            match response {
                Ok(_) => {
                    info!(message = "sent transaction to the mempool", hash=%transaction.tx_hash());
                }
                Err(e) => {
                    warn!(
                        message = "Failed to send raw transaction to mempool",
                        error = %e
                    );
                }
            }
        }

        Ok(transaction.tx_hash())
    }
}
