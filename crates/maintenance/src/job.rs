use alloy_provider::Provider;
use alloy_rpc_types::eth::Block;
use anyhow::Result;
use op_alloy_network::Optimism;
use op_alloy_rpc_types::Transaction;
use tips_datastore::BundleDatastore;

pub struct BundleStore<T: BundleDatastore, P: Provider<Optimism>> {
    pub store: T,
    pub node: P,
}

impl<T: BundleDatastore, P: Provider<Optimism>> BundleStore<T, P> {
    pub fn new(store: T, node: P) -> Self {
        Self { store, node }
    }

    pub async fn on_new_block(&self, block: Block<Transaction>) -> Result<()> {
        // TODO: Bulk lookup, bundle by transaction hashes
        // TODO:

        Ok(())
    }
}
