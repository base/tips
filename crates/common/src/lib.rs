use alloy_primitives::{Address, TxHash};
use alloy_rpc_types_mev::EthSendBundle;
use sqlx::types::chrono::{DateTime, Utc};

#[derive(Debug, Clone, sqlx::Type)]
#[sqlx(type_name = "bundle_state", rename_all = "PascalCase")]
pub enum BundleState {
    Ready,
    IncludedByBuilder,
}

/// Extended bundle data that includes the original bundle plus extracted metadata
#[derive(Debug, Clone)]
pub struct BundleWithMetadata {
    pub bundle: EthSendBundle,
    pub txn_hashes: Vec<TxHash>,
    pub senders: Vec<Address>,
    pub min_base_fee: i64,
    pub state: BundleState,
    pub state_changed_at: DateTime<Utc>,
}
