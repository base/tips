use alloy_consensus::{Transaction, Typed2718, constants::KECCAK_EMPTY, transaction::Recovered};
use alloy_primitives::{Address, B256, U256};
use alloy_provider::{Provider, RootProvider};
use anyhow::Result;
use async_trait::async_trait;
use jsonrpsee::{core::RpcResult, types::ErrorObject};
use op_alloy_consensus::OpTxEnvelope;
use op_alloy_consensus::interop::CROSS_L2_INBOX_ADDRESS;
use op_alloy_network::Optimism;
use op_revm::{OpSpecId, l1block::L1BlockInfo};
use reth_rpc_eth_types::{EthApiError, RpcInvalidTransactionError};

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

pub async fn validate_tx(
    account: AccountInfo,
    txn: &Recovered<OpTxEnvelope>,
    data: &[u8],
) -> RpcResult<B256> {
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
        let obj = ErrorObject::owned(11, "Insufficient funds for L1 gas", Some(2));
        return Err(EthApiError::InvalidTransaction(
            RpcInvalidTransactionError::other(obj)
        )
        .into_rpc_err());
    }
    Ok(txn.tx_hash())
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_consensus::SignableTransaction;
    use alloy_consensus::{TxEip1559, TxEip7702};
    use alloy_consensus::{Transaction, constants::KECCAK_EMPTY, transaction::SignerRecoverable};
    use alloy_primitives::{bytes, keccak256};
    use alloy_signer_local::PrivateKeySigner;
    use op_alloy_network::TxSignerSync;
    use revm_context_interface::transaction::{AccessList, AccessListItem};

    fn create_account(nonce: u64, balance: U256) -> AccountInfo {
        AccountInfo {
            balance,
            nonce,
            code_hash: KECCAK_EMPTY,
        }
    }

    fn create_7702_account() -> AccountInfo {
        AccountInfo {
            balance: U256::from(1000000000000000000u128),
            nonce: 0,
            code_hash: keccak256(bytes!("1234567890")),
        }
    }

    #[tokio::test]
    async fn test_valid_tx() {
        // Create a sample EIP-1559 transaction
        let signer = PrivateKeySigner::random();
        let mut tx = TxEip1559 {
            chain_id: 1,
            nonce: 0,
            gas_limit: 21000,
            max_fee_per_gas: 20000000000u128,
            max_priority_fee_per_gas: 1000000000u128,
            to: Address::random().into(),
            value: U256::from(10000000000000u128),
            access_list: Default::default(),
            input: bytes!("").clone(),
        };

        let account = create_account(0, U256::from(1000000000000000000u128));

        let data = tx.input().to_vec();
        let signature = signer.sign_transaction_sync(&mut tx).unwrap();
        let envelope = OpTxEnvelope::Eip1559(tx.into_signed(signature));
        let recovered_tx = envelope.try_into_recovered().unwrap();
        assert!(validate_tx(account, &recovered_tx, &data).await.is_ok());
    }

    #[tokio::test]
    async fn test_valid_7702_tx() {
        let signer = PrivateKeySigner::random();
        let mut tx = TxEip7702 {
            chain_id: 1,
            nonce: 0,
            gas_limit: 21000,
            max_fee_per_gas: 20000000000u128,
            max_priority_fee_per_gas: 1000000000u128,
            to: Address::random().into(),
            value: U256::from(10000000000000u128),
            authorization_list: Default::default(),
            access_list: Default::default(),
            input: bytes!("").clone(),
        };

        let account = create_7702_account();

        let data = tx.input().to_vec();
        let signature = signer.sign_transaction_sync(&mut tx).unwrap();
        let envelope = OpTxEnvelope::Eip7702(tx.into_signed(signature));
        let recovered_tx = envelope.try_into_recovered().unwrap();
        assert!(validate_tx(account, &recovered_tx, &data).await.is_ok());
    }

    #[tokio::test]
    async fn test_err_interop_tx() {
        let signer = PrivateKeySigner::random();

        let access_list = AccessList::from(vec![AccessListItem {
            address: CROSS_L2_INBOX_ADDRESS,
            storage_keys: vec![],
        }]);

        let mut tx = TxEip1559 {
            chain_id: 1,
            nonce: 0,
            gas_limit: 21000,
            max_fee_per_gas: 20000000000u128,
            max_priority_fee_per_gas: 1000000000u128,
            to: Address::random().into(),
            value: U256::from(10000000000000u128),
            access_list,
            input: bytes!("").clone(),
        };

        let account = create_account(0, U256::from(1000000000000000000u128));

        let data = tx.input().to_vec();
        let signature = signer.sign_transaction_sync(&mut tx).unwrap();
        let envelope = OpTxEnvelope::Eip1559(tx.into_signed(signature));
        let recovered_tx = envelope.try_into_recovered().unwrap();

        let obj = ErrorObject::owned(11, "Interop transactions are not supported", Some(2));
        assert_eq!(
            validate_tx(account, &recovered_tx, &data).await,
            Err(RpcInvalidTransactionError::other(obj).into_rpc_err())
        );
    }

    #[tokio::test]
    async fn test_err_tx_not_7702() {
        let signer = PrivateKeySigner::random();

        let mut tx = TxEip1559 {
            chain_id: 1,
            nonce: 0,
            gas_limit: 21000,
            max_fee_per_gas: 20000000000u128,
            max_priority_fee_per_gas: 1000000000u128,
            to: Address::random().into(),
            value: U256::from(10000000000000u128),
            access_list: Default::default(),
            input: bytes!("").clone(),
        };

        // account is 7702
        let account = create_7702_account();

        let data = tx.input().to_vec();
        let signature = signer.sign_transaction_sync(&mut tx).unwrap();
        let envelope = OpTxEnvelope::Eip1559(tx.into_signed(signature));
        let recovered_tx = envelope.try_into_recovered().unwrap();

        assert_eq!(validate_tx(account, &recovered_tx, &data).await, Err(EthApiError::InvalidTransaction(
            RpcInvalidTransactionError::AuthorizationListInvalidFields,
        )
        .into_rpc_err()));
    }

    #[tokio::test]
    async fn test_err_tx_nonce_too_low() {
        let signer = PrivateKeySigner::random();
        let mut tx = TxEip1559 {
            chain_id: 1,
            nonce: 0,
            gas_limit: 21000,
            max_fee_per_gas: 20000000000u128,
            max_priority_fee_per_gas: 1000000000u128,
            to: Address::random().into(),
            value: U256::from(10000000000000u128),
            access_list: Default::default(),
            input: bytes!("").clone(),
        };

        let account = create_account(1, U256::from(1000000000000000000u128));
        
        let nonce = account.nonce;
        let tx_nonce = tx.nonce();

        let data = tx.input().to_vec();
        let signature = signer.sign_transaction_sync(&mut tx).unwrap();
        let envelope = OpTxEnvelope::Eip1559(tx.into_signed(signature));
        let recovered_tx = envelope.try_into_recovered().unwrap();
        assert_eq!(validate_tx(account, &recovered_tx, &data).await, Err(EthApiError::InvalidTransaction(
            RpcInvalidTransactionError::NonceTooLow {
                tx: tx_nonce,
                state: nonce,
            })
            .into_rpc_err()));
    }

    #[tokio::test]
    async fn test_err_tx_insufficient_funds() {
        let signer = PrivateKeySigner::random();
        let mut tx = TxEip1559 {
            chain_id: 1,
            nonce: 0,
            gas_limit: 21000,
            max_fee_per_gas: 20000000000u128,
            max_priority_fee_per_gas: 10000000000000u128,
            to: Address::random().into(),
            value: U256::from(10000000000000u128),
            access_list: Default::default(),
            input: bytes!("").clone(),
        };

        let account = create_account(0, U256::from(1000000u128));

        let data = tx.input().to_vec();
        let signature = signer.sign_transaction_sync(&mut tx).unwrap();
        let envelope = OpTxEnvelope::Eip1559(tx.into_signed(signature));
        let recovered_tx = envelope.try_into_recovered().unwrap();
        assert_eq!(validate_tx(account, &recovered_tx, &data).await, Err(EthApiError::InvalidTransaction(
            RpcInvalidTransactionError::InsufficientFundsForTransfer,
        )
        .into_rpc_err()));
    }

    #[tokio::test]
    async fn test_err_tx_insufficient_funds_for_l1_gas() {
        let signer = PrivateKeySigner::random();
        let mut tx = TxEip1559 {
            chain_id: 1,
            nonce: 0,
            gas_limit: 21000,
            max_fee_per_gas: 20000000000u128,
            max_priority_fee_per_gas: 1000000000u128,
            to: Address::random().into(),
            value: U256::from(10000000000000u128),
            access_list: Default::default(),
            input: bytes!("").clone(),
        };

        let account = create_account(0, U256::from(1000000u128));

        let data = tx.input().to_vec();
        let signature = signer.sign_transaction_sync(&mut tx).unwrap();
        let envelope = OpTxEnvelope::Eip1559(tx.into_signed(signature));
        let recovered_tx = envelope.try_into_recovered().unwrap();
        assert_eq!(validate_tx(account, &recovered_tx, &data).await, Err(EthApiError::InvalidTransaction(
            RpcInvalidTransactionError::InsufficientFundsForTransfer,
        )
        .into_rpc_err()));
    }

}
