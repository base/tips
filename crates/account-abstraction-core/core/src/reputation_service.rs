use crate::mempool;
use alloy_primitives::Address;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Reputation status for an entity
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ReputationStatus {
    /// Entity is not throttled or banned
    Ok,
    /// Entity is throttled
    Throttled,
    /// Entity is banned
    Banned,
}

pub trait ReputationService {
    fn get_reputation(&self, entity: &Address) -> ReputationStatus;
}

pub struct ReputationServiceImpl {
    mempool: Arc<RwLock<mempool::MempoolImpl>>,
}

impl ReputationServiceImpl {
    pub async fn new(mempool: Arc<RwLock<mempool::MempoolImpl>>) -> Self {
        Self { mempool }
    }
}

impl ReputationService for ReputationServiceImpl {
    fn get_reputation(&self, _entity: &Address) -> ReputationStatus {
        ReputationStatus::Ok
    }
}
