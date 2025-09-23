use alloy_rpc_types_mev::EthSendBundle;
use anyhow::Result;
use async_trait::async_trait;
use tips_datastore::BundleDatastore;
use uuid::Uuid;

#[async_trait]
pub trait Writer: Send + Sync {
    async fn write_bundle(&self, bundle: EthSendBundle) -> Result<Uuid>;
}

pub struct DatastoreWriter<Store> {
    datastore: Store,
}

impl<Store> DatastoreWriter<Store> {
    pub fn new(datastore: Store) -> Self {
        Self { datastore }
    }
}

#[async_trait]
impl<Store> Writer for DatastoreWriter<Store>
where
    Store: BundleDatastore + Send + Sync + 'static,
{
    async fn write_bundle(&self, bundle: EthSendBundle) -> Result<Uuid> {
        self.datastore.insert_bundle(bundle).await
    }
}
