use alloy_consensus::transaction::SignerRecoverable;
use alloy_primitives::{B256, Bytes};
use alloy_provider::{Provider, RootProvider, network::eip2718::Decodable2718};
use alloy_rpc_types_mev::{EthBundleHash, EthCancelBundle, EthSendBundle};
use jsonrpsee::{
    core::{RpcResult, async_trait},
    proc_macros::rpc,
};
use op_alloy_consensus::OpTxEnvelope;
use op_alloy_network::Optimism;
use reth_rpc_eth_types::EthApiError;
use std::time::{SystemTime, UNIX_EPOCH};
use tips_common::ValidationData;
use tokio::sync::{mpsc, oneshot};
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
    send_transaction_default_lifetime_seconds: u64,
    tx_sender: mpsc::UnboundedSender<ValidationData>,
}

impl<Queue> IngressService<Queue> {
    pub fn new(
        provider: RootProvider<Optimism>,
        dual_write_mempool: bool,
        queue: Queue,
        send_transaction_default_lifetime_seconds: u64,
        tx_sender: mpsc::UnboundedSender<ValidationData>,
    ) -> Self {
        Self {
            provider,
            dual_write_mempool,
            queue,
            send_transaction_default_lifetime_seconds,
            tx_sender,
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

        // Send transaction data to ExEx for validation
        // TODO: in the endgame version, this would push to a Redis cluster which the "Store" ExEx
        // would read from. Instead, we are implementing v1 of this by having the ingress-rpc
        // forward to the rpc-exex.
        let (response_tx, response_rx) = oneshot::channel();
        let validation_data = ValidationData {
            address: transaction.signer(),
            tx: transaction.clone(),
            data: data.clone(),
            response_tx,
        };

        if let Err(e) = self.tx_sender.send(validation_data) {
            warn!(message = "Failed to send transaction to ExEx", error = %e);
            // TODO: error on here?
        }

        // Wait for validation result from ExEx
        match response_rx.await {
            Ok(Ok(())) => {
                // Validation successful, continue processing
            }
            Ok(Err(validation_error)) => {
                // Validation failed, return the error
                return Err(validation_error);
            }
            Err(_) => {
                // Channel was dropped, ExEx is not responding
                warn!(message = "ExEx validation channel dropped");
                // TODO: error on here?
            }
        }

        let expiry_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + self.send_transaction_default_lifetime_seconds;

        let bundle = EthSendBundle {
            txs: vec![data.clone()],
            block_number: 0,
            min_timestamp: None,
            max_timestamp: Some(expiry_timestamp),
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
