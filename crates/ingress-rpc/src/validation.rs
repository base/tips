use alloy_consensus::{Transaction, Typed2718, constants::KECCAK_EMPTY, transaction::Recovered};
use alloy_primitives::{Address, B256, U256, address};
use alloy_provider::{Provider, RootProvider};
use anyhow::Result;
use jsonrpsee::{core::RpcResult, types::ErrorObject};
use op_alloy_consensus::OpTxEnvelope;
use op_alloy_network::Optimism;
use op_revm::{OpSpecId, l1block::L1BlockInfo};
use reth_rpc_eth_types::{EthApiError, RpcInvalidTransactionError};
use async_trait::async_trait;

// from: https://github.com/alloy-rs/op-alloy/blob/main/crates/consensus/src/interop.rs#L9
// reference: https://github.com/paradigmxyz/reth/blob/bdc59799d0651133d8b191bbad62746cb5036595/crates/optimism/txpool/src/supervisor/access_list.rs#L39
const CROSS_L2_INBOX_ADDRESS: Address = address!("0x4200000000000000000000000000000000000022");

pub struct AccountInfo {
    pub balance: U256,
    pub nonce: u64,
    pub code_hash: B256,
}

#[async_trait]
pub trait AccountInfoLookup: Send + Sync {
    async fn fetch_account_info(&self, address: Address) -> Result<AccountInfo>;
}

#[async_trait]
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

#[async_trait]
pub trait TxValidation: Send + Sync {
    async fn validate_tx(&self, txn: &Recovered<OpTxEnvelope>, data: &[u8]) -> RpcResult<B256>;
}

#[async_trait]
impl TxValidation for RootProvider<Optimism> {
    async fn validate_tx(&self, txn: &Recovered<OpTxEnvelope>, data: &[u8]) -> RpcResult<B256> {
        let account = self.fetch_account_info(txn.signer()).await.map_err(|e| {
            let obj = ErrorObject::owned(11, e.to_string(), Some(2));
            RpcInvalidTransactionError::other(obj).into_rpc_err()
        })?;

        // skip eip4844 transactions
        if txn.is_eip4844() {
            let obj = ErrorObject::owned(11, "EIP-4844 transactions are not supported", Some(2));
            return Err(RpcInvalidTransactionError::other(obj).into_rpc_err());
        }

        // from: https://github.com/paradigmxyz/reth/blob/3b0d98f3464b504d96154b787a860b2488a61b3e/crates/optimism/txpool/src/supervisor/client.rs#L76-L84
        // it returns `None` if a tx is not cross chain, which is when `inbox_entries` is empty in the snippet above.
        // we can do something similar where if the inbox_entries is non-empty then it is a cross chain tx and it's something we don't support
        if let Some(access_list) = txn.access_list() {
            let inbox_entries = access_list
                .iter()
                .filter(|entry| entry.address == CROSS_L2_INBOX_ADDRESS);
            if inbox_entries.count() > 0 {
                let obj = ErrorObject::owned(11, "Interop transactions are not supported", Some(2));
                return Err(RpcInvalidTransactionError::other(obj).into_rpc_err());
            }
        }

        // error if account is 7702 but tx is not 7702
        if account.code_hash != KECCAK_EMPTY && !txn.is_eip7702() {
            return Err(EthApiError::InvalidTransaction(
                RpcInvalidTransactionError::AuthorizationListInvalidFields,
            )
            .into_rpc_err());
        }

        // error if tx nonce is not the latest
        // https://github.com/paradigmxyz/reth/blob/a047a055ab996f85a399f5cfb2fe15e350356546/crates/transaction-pool/src/validate/eth.rs#L611
        if txn.nonce() < account.nonce {
            return Err(
                EthApiError::InvalidTransaction(RpcInvalidTransactionError::NonceTooLow {
                    tx: txn.nonce(),
                    state: account.nonce,
                })
                .into_rpc_err(),
            );
        }

        // For EIP-1559 transactions: `max_fee_per_gas * gas_limit + tx_value`.
        // ref: https://github.com/paradigmxyz/reth/blob/main/crates/transaction-pool/src/traits.rs#L1186
        let max_fee = txn
            .max_fee_per_gas()
            .saturating_mul(txn.gas_limit() as u128);
        let txn_cost = txn.value().saturating_add(U256::from(max_fee));

        // error if execution cost costs more than balance
        if txn_cost > account.balance {
            return Err(EthApiError::InvalidTransaction(
                RpcInvalidTransactionError::InsufficientFundsForTransfer,
            )
            .into_rpc_err());
        }

        // op-checks to see if sender can cover L1 gas cost
        // from: https://github.com/paradigmxyz/reth/blob/6aa73f14808491aae77fc7c6eb4f0aa63bef7e6e/crates/optimism/txpool/src/validator.rs#L219
        let mut l1_block_info = L1BlockInfo::default();
        let l1_cost_addition = l1_block_info.calculate_tx_l1_cost(data, OpSpecId::ISTHMUS);
        let l1_cost = txn_cost.saturating_add(l1_cost_addition);
        if l1_cost > account.balance {
            return Err(EthApiError::InvalidTransaction(
                RpcInvalidTransactionError::InsufficientFundsForTransfer,
            )
            .into_rpc_err());
        }
        Ok(txn.tx_hash())
    }
}
