use crate::domain::types::{UserOpHash, WrappedUserOperation};
use std::sync::Arc;
use std::fmt::Debug;
#[derive(Default, Debug)]
pub struct PoolConfig {
    pub minimum_max_fee_per_gas: u128,
}

pub trait Mempool: Send + Sync + Debug {
    fn add_operation(&mut self, operation: &WrappedUserOperation) -> Result<(), anyhow::Error>;

    fn get_top_operations(&self, n: usize) -> Vec<Arc<WrappedUserOperation>>;

    fn remove_operation(
        &mut self,
        operation_hash: &UserOpHash,
    ) -> Result<Option<WrappedUserOperation>, anyhow::Error>;
}
