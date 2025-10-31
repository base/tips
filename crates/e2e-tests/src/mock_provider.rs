use alloy_consensus::constants::KECCAK_EMPTY;
use alloy_network::Network;
use alloy_primitives::{Address, U256};
use alloy_provider::Provider as AlloyProvider;
use async_trait::async_trait;
use jsonrpsee::core::RpcResult;
use op_revm::l1block::L1BlockInfo;
use tips_ingress_rpc::validation::{AccountInfo, AccountInfoLookup, L1BlockInfoLookup};

/// Mock provider that returns generous account balances and minimal L1 costs
#[derive(Clone, Debug)]
pub struct MockProvider {
    default_balance: U256,
    default_nonce: u64,
}

impl MockProvider {
    pub fn new() -> Self {
        Self {
            default_balance: U256::from(100_000_000_000_000_000_000u128), // 100 ETH
            default_nonce: 0,
        }
    }
}

impl Default for MockProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AccountInfoLookup for MockProvider {
    async fn fetch_account_info(&self, _address: Address) -> RpcResult<AccountInfo> {
        Ok(AccountInfo {
            balance: self.default_balance,
            nonce: self.default_nonce,
            code_hash: KECCAK_EMPTY,
        })
    }
}

#[async_trait]
impl L1BlockInfoLookup for MockProvider {
    async fn fetch_l1_block_info(&self) -> RpcResult<L1BlockInfo> {
        Ok(L1BlockInfo::default())
    }
}

// Stub implementation of AlloyProvider for MockProvider
// This is needed to satisfy the trait bound but should never be called
// since dual_write_mempool is false in tests to avoid the need for a real L1 node
impl<N: Network> AlloyProvider<N> for MockProvider {
    fn root(&self) -> &alloy_provider::RootProvider<N> {
        panic!(
            "MockProvider::root() should never be called - dual_write_mempool should be false in tests"
        )
    }
}
