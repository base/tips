use alloy_consensus::Typed2718;
use alloy_consensus::constants::KECCAK_EMPTY;
use alloy_consensus::{Transaction, transaction::SignerRecoverable};
use alloy_primitives::{Address, B256, Bytes, address};
use alloy_provider::network::eip2718::Decodable2718;
use alloy_provider::{Provider, RootProvider};
use alloy_rpc_types_mev::{EthBundleHash, EthCancelBundle, EthSendBundle};
use anyhow::Result;
use jsonrpsee::types::ErrorObject;
use jsonrpsee::{
    core::{RpcResult, async_trait},
    proc_macros::rpc,
};
use op_alloy_consensus::OpTxEnvelope;
use op_alloy_network::Optimism;
use reth_rpc_eth_types::EthApiError;
use tracing::{info, warn};

use crate::queue::QueuePublisher;

// from: https://github.com/alloy-rs/op-alloy/blob/main/crates/consensus/src/interop.rs#L9
// reference: https://github.com/paradigmxyz/reth/blob/bdc59799d0651133d8b191bbad62746cb5036595/crates/optimism/txpool/src/supervisor/access_list.rs#L39
const CROSS_L2_INBOX_ADDRESS: Address = address!("0x4200000000000000000000000000000000000022");

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

    async fn validate_tx(&self, envelope: &OpTxEnvelope) -> Result<()> {
        let sender = envelope.recover_signer().unwrap_or_default();
        let account = self.provider.get_account(sender).await.unwrap_or_default();

        // skip eip4844 transactions
        if envelope.is_eip4844() {
            return Err(anyhow::anyhow!("EIP-4844 transactions are not supported"));
        }

        // from: https://github.com/paradigmxyz/reth/blob/3b0d98f3464b504d96154b787a860b2488a61b3e/crates/optimism/txpool/src/supervisor/client.rs#L76-L84
        // it returns `None` if a tx is not cross chain, which is when `inbox_entries` is empty in the snippet above.
        // we can do something similar where if the inbox_entries is non-empty then it is a cross chain tx and it's something we don't support
        if let Some(access_list) = envelope.access_list() {
            let inbox_entries = access_list
                .iter()
                .filter(|entry| entry.address == CROSS_L2_INBOX_ADDRESS);
            if inbox_entries.count() > 0 {
                return Err(anyhow::anyhow!("Interop transactions are not supported"));
            }
        }

        // check account bytecode to see if the account is 7702 then is the tx 7702
        if account.code_hash != KECCAK_EMPTY && envelope.is_eip7702() {
            return Err(anyhow::anyhow!(
                "Account is a 7702 account but transaction is not EIP-7702"
            ));
        }

        // check if nonce is the latest
        if account.nonce < envelope.nonce() {
            return Err(anyhow::anyhow!("Nonce is not the latest"));
        }

        // check if execution cost <= balance
        if account.balance < envelope.value() {
            return Err(anyhow::anyhow!("Insufficient funds"));
        }
        Ok(())
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

        if let Err(validation_error) = self.validate_tx(&envelope).await {
            warn!(
                sender = %transaction.recover_signer().unwrap_or_default(),
                error = ?validation_error,
                "Transaction validation failed"
            );
            return Err(ErrorObject::owned(
                11,
                validation_error.to_string(),
                Some(2),
            ));
        }

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
