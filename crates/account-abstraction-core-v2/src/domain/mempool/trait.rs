use crate::domain::types::{UserOpHash, WrappedUserOperation};
use alloy_primitives::Address;
use std::cmp::Ordering;
use std::sync::Arc;

#[derive(Default)]
pub struct PoolConfig {
    pub minimum_max_fee_per_gas: u128,
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct OrderedPoolOperation {
    pub pool_operation: WrappedUserOperation,
    pub submission_id: u64,
}

impl OrderedPoolOperation {
    pub fn from_wrapped(operation: &WrappedUserOperation, submission_id: u64) -> Self {
        Self {
            pool_operation: operation.clone(),
            submission_id,
        }
    }

    pub fn sender(&self) -> Address {
        self.pool_operation.operation.sender()
    }
}

#[derive(Clone, Debug)]
pub struct ByMaxFeeAndSubmissionId(pub OrderedPoolOperation);

impl PartialEq for ByMaxFeeAndSubmissionId {
    fn eq(&self, other: &Self) -> bool {
        self.0.pool_operation.hash == other.0.pool_operation.hash
    }
}
impl Eq for ByMaxFeeAndSubmissionId {}

impl PartialOrd for ByMaxFeeAndSubmissionId {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ByMaxFeeAndSubmissionId {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .0
            .pool_operation
            .operation
            .max_priority_fee_per_gas()
            .cmp(&self.0.pool_operation.operation.max_priority_fee_per_gas())
            .then_with(|| self.0.submission_id.cmp(&other.0.submission_id))
    }
}

#[derive(Clone, Debug)]
pub struct ByNonce(pub OrderedPoolOperation);

impl PartialEq for ByNonce {
    fn eq(&self, other: &Self) -> bool {
        self.0.pool_operation.hash == other.0.pool_operation.hash
    }
}
impl Eq for ByNonce {}

impl PartialOrd for ByNonce {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ByNonce {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0
            .pool_operation
            .operation
            .nonce()
            .cmp(&other.0.pool_operation.operation.nonce())
            .then_with(|| self.0.submission_id.cmp(&other.0.submission_id))
            .then_with(|| self.0.pool_operation.hash.cmp(&other.0.pool_operation.hash))
    }
}

pub trait Mempool: Send + Sync {
    fn add_operation(
        &mut self,
        operation: &WrappedUserOperation,
    ) -> Result<Option<OrderedPoolOperation>, anyhow::Error>;
    fn get_top_operations(&self, n: usize) -> impl Iterator<Item = Arc<WrappedUserOperation>>;
    fn remove_operation(
        &mut self,
        operation_hash: &UserOpHash,
    ) -> Result<Option<WrappedUserOperation>, anyhow::Error>;
}
