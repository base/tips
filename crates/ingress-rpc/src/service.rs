use alloy_consensus::{
    Transaction, Typed2718, constants::KECCAK_EMPTY, transaction::SignerRecoverable,
};
use alloy_primitives::{Address, B256, Bytes, U256, address};
use alloy_provider::{Provider, RootProvider, network::eip2718::Decodable2718};
use alloy_rpc_types_mev::{EthBundleHash, EthCancelBundle, EthSendBundle};
use anyhow::Result;
use jsonrpsee::{
    core::{RpcResult, async_trait},
    proc_macros::rpc,
    types::ErrorObject,
};
use op_alloy_consensus::OpTxEnvelope;
use op_alloy_network::{Optimism, eip2718::Encodable2718};
use op_revm::{OpSpecId, l1block::L1BlockInfo};
use reth_rpc_eth_types::EthApiError;
use tracing::{info, warn};

use crate::queue::QueuePublisher;

// from: https://github.com/alloy-rs/op-alloy/blob/main/crates/consensus/src/interop.rs#L9
// reference: https://github.com/paradigmxyz/reth/blob/bdc59799d0651133d8b191bbad62746cb5036595/crates/optimism/txpool/src/supervisor/access_list.rs#L39
const CROSS_L2_INBOX_ADDRESS: Address = address!("0x4200000000000000000000000000000000000022");

pub struct AccountInfo {
    pub balance: U256,
    pub nonce: u64,
    pub code_hash: B256,
}

pub trait AccountInfoLookup {
    async fn fetch_account_info(&self, address: Address) -> Result<AccountInfo>;
}

impl AccountInfoLookup for RootProvider<Optimism> {
    async fn fetch_account_info(&self, address: Address) -> Result<AccountInfo> {
        let account = self.get_account(address).await?;
        Ok(AccountInfo {
            balance: account.balance,
            nonce: account.nonce,
            code_hash: account.code_hash,
        })
    }
}

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
        let account = self.provider.fetch_account_info(sender).await?;

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

        // error if account is 7702 but tx is not 7702
        if account.code_hash != KECCAK_EMPTY && !envelope.is_eip7702() {
            return Err(anyhow::anyhow!(
                "Account is a 7702 account but transaction is not EIP-7702"
            ));
        }

        // error if tx nonce is not the latest
        // https://github.com/paradigmxyz/reth/blob/a047a055ab996f85a399f5cfb2fe15e350356546/crates/transaction-pool/src/validate/eth.rs#L611
        if envelope.nonce() < account.nonce {
            return Err(anyhow::anyhow!("Nonce is not the latest"));
        }

        // error if execution cost costs more than balance
        if envelope.value() > account.balance {
            return Err(anyhow::anyhow!("Insufficient funds"));
        }

        // op-checks to see if sender can cover L1 gas cost
        // from: https://github.com/paradigmxyz/reth/blob/6aa73f14808491aae77fc7c6eb4f0aa63bef7e6e/crates/optimism/txpool/src/validator.rs#L219
        let mut l1_block_info = L1BlockInfo::default();
        let tx = envelope.clone().try_into_pooled()?;
        let encoded = tx.encoded_2718();

        let cost_addition = l1_block_info.calculate_tx_l1_cost(&encoded, OpSpecId::ISTHMUS);
        let cost = tx.value().saturating_add(cost_addition);
        if cost > account.balance {
            return Err(anyhow::anyhow!("Insufficient funds to cover L1 gas cost"));
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

        self.validate_tx(&envelope)
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
