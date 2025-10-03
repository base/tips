use crate::postgres::{BundleFilter, BundleWithMetadata, Simulation, StateDiff};
use alloy_primitives::TxHash;
use alloy_rpc_types_mev::EthSendBundle;
use anyhow::Result;
use uuid::Uuid;

/// Trait defining the interface for bundle datastore operations
#[async_trait::async_trait]
pub trait BundleDatastore: Send + Sync {
    /// Insert a new bundle into the datastore
    async fn insert_bundle(&self, bundle: EthSendBundle) -> Result<Uuid>;

    /// Fetch a bundle with metadata by its ID
    async fn get_bundle(&self, id: Uuid) -> Result<Option<BundleWithMetadata>>;

    /// Cancel a bundle by UUID
    async fn cancel_bundle(&self, id: Uuid) -> Result<()>;

    /// Select the candidate bundles to include in the next Flashblock
    async fn select_bundles(&self, filter: BundleFilter) -> Result<Vec<BundleWithMetadata>>;

    /// Find bundle ID by transaction hash
    async fn find_bundle_by_transaction_hash(&self, tx_hash: TxHash) -> Result<Option<Uuid>>;

    /// Remove a bundle by ID
    async fn remove_bundle(&self, id: Uuid) -> Result<()>;

    /// Insert a new simulation result
    async fn insert_simulation(
        &self,
        bundle_id: Uuid,
        block_number: u64,
        block_hash: String,
        execution_time_us: u64,
        gas_used: u64,
        state_diff: StateDiff,
    ) -> Result<Uuid>;

    /// Get a simulation by its ID
    async fn get_simulation(&self, id: Uuid) -> Result<Option<Simulation>>;

    /// Select bundles with their latest simulation
    /// Only bundles that have at least one simulation are returned
    async fn select_bundles_with_latest_simulation(
        &self,
        filter: BundleFilter,
    ) -> Result<Vec<(BundleWithMetadata, Simulation)>>;
}
