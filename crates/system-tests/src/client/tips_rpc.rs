use account_abstraction_core::domain::types::VersionedUserOperation;
use alloy_network::Network;
use alloy_primitives::{Address, Bytes, FixedBytes, TxHash};
use alloy_provider::{Provider, RootProvider};
use anyhow::Result;
use tips_core::{Bundle, BundleHash, CancelBundle};

/// Client for TIPS-specific RPC methods (eth_sendBundle, eth_cancelBundle)
///
/// Wraps a RootProvider to add TIPS functionality while preserving access
/// to standard Ethereum JSON-RPC methods via provider().
#[derive(Clone)]
pub struct TipsRpcClient<N: Network = alloy_network::Ethereum> {
    provider: RootProvider<N>,
}

impl<N: Network> TipsRpcClient<N> {
    pub fn new(provider: RootProvider<N>) -> Self {
        Self { provider }
    }

    pub async fn send_raw_transaction(&self, signed_tx: Bytes) -> Result<TxHash> {
        let tx_hex = format!("0x{}", hex::encode(&signed_tx));
        self.provider
            .raw_request("eth_sendRawTransaction".into(), [tx_hex])
            .await
            .map_err(Into::into)
    }

    pub async fn send_bundle(&self, bundle: Bundle) -> Result<BundleHash> {
        self.provider
            .raw_request("eth_sendBundle".into(), [bundle])
            .await
            .map_err(Into::into)
    }

    pub async fn send_backrun_bundle(&self, bundle: Bundle) -> Result<BundleHash> {
        self.provider
            .raw_request("eth_sendBackrunBundle".into(), [bundle])
            .await
            .map_err(Into::into)
    }

    pub async fn cancel_bundle(&self, request: CancelBundle) -> Result<bool> {
        self.provider
            .raw_request("eth_cancelBundle".into(), [request])
            .await
            .map_err(Into::into)
    }

    pub async fn send_user_operation(
        &self,
        user_operation: VersionedUserOperation,
        entry_point: Address,
    ) -> Result<FixedBytes<32>> {
        self.provider
            .raw_request(
                "eth_sendUserOperation".into(),
                (user_operation, entry_point),
            )
            .await
            .map_err(Into::into)
    }

    pub fn provider(&self) -> &RootProvider<N> {
        &self.provider
    }
}
