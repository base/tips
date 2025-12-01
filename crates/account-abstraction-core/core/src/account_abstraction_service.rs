use crate::user_ops_types::UserOperationRequest;
use alloy_provider::{RootProvider};
use std::fmt::Error;
use op_alloy_network::Optimism;
use std::sync::Arc;

pub trait AccountAbstractionService {
    fn validate_user_operation(&self, user_operation: UserOperationRequest) -> Result<bool, Error>;
}

#[derive(Debug, Clone)]
pub struct AccountAbstractionServiceImpl<> {
    simulation_provider: Arc<RootProvider<Optimism>>,
    provider: Arc<RootProvider<Optimism>>
}

impl AccountAbstractionService for AccountAbstractionServiceImpl {
    fn validate_user_operation(&self, _user_operation: UserOperationRequest) -> Result<bool, Error> {
        todo!("validate_user_operation");
    }
}

impl AccountAbstractionServiceImpl {
    pub fn new(simulation_provider: Arc<RootProvider<Optimism>>, provider: Arc<RootProvider<Optimism>>) -> Self {
        Self { simulation_provider, provider }
    }
}