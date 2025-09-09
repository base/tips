use alloy_rpc_types_mev::EthSendBundle;
use anyhow::Result;
use uuid::Uuid;

/// Trait defining the interface for bundle datastore operations
#[async_trait::async_trait]
pub trait BundleDatastore: Send + Sync {
    /// Insert a new bundle into the datastore
    async fn insert_bundle(&self, bundle: EthSendBundle) -> Result<Uuid>;

    /// Fetch a bundle by its ID
    async fn get_bundle(&self, id: Uuid) -> Result<Option<EthSendBundle>>;

    /// Cancel a bundle by UUID
    async fn cancel_bundle(&self, id: Uuid) -> Result<()>;

    /// Select the candidate bundles to include in the next Flashblock
    async fn select_bundles(&self) -> Result<Vec<EthSendBundle>>;
}
