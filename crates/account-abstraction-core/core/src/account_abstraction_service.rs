use crate::types::{UserOperationRequest, UserOperationRequestValidationResult};
use alloy_provider::{Provider, RootProvider};
use jsonrpsee::core::RpcResult;
use op_alloy_network::Optimism;
use reth_rpc_eth_types::EthApiError;
use std::sync::Arc;
use tokio::time::{Duration, timeout};
pub trait AccountAbstractionService {
    async fn validate_user_operation(
        &self,
        user_operation: UserOperationRequest,
    ) -> RpcResult<UserOperationRequestValidationResult>;
}

#[derive(Debug, Clone)]
pub struct AccountAbstractionServiceImpl {
    simulation_provider: Arc<RootProvider<Optimism>>,
    validate_user_operation_timeout: u64,
}

impl AccountAbstractionService for AccountAbstractionServiceImpl {
    async fn validate_user_operation(
        &self,
        user_operation: UserOperationRequest,
    ) -> RpcResult<UserOperationRequestValidationResult> {
        // Steps: Reputation Service Validate
        // Steps: Base Node Validate User Operation
        let validation_result = self.base_node_validate_user_operation(user_operation).await;
        match validation_result {
            Ok(validation_result) => {
                return Ok(validation_result);
            }
            Err(e) => {
                return Err(e);
            }
        }
    }
}

impl AccountAbstractionServiceImpl {
    pub fn new(
        simulation_provider: Arc<RootProvider<Optimism>>,
        validate_user_operation_timeout: u64,
    ) -> Self {
        Self {
            simulation_provider,
            validate_user_operation_timeout,
        }
    }

    pub async fn base_node_validate_user_operation(
        &self,
        user_operation: UserOperationRequest,
    ) -> RpcResult<UserOperationRequestValidationResult> {
        let result = timeout(
            Duration::from_secs(self.validate_user_operation_timeout),
            self.simulation_provider
                .raw_request("base_validateUserOperation".into(), (user_operation,)),
        )
        .await;

        let validation_result: UserOperationRequestValidationResult = match result {
            Err(e) => {
                return Err(
                    EthApiError::InvalidParams("Timeout on requesting validation".into())
                        .into_rpc_err(),
                ); 
            }
            Ok(Err(e)) => {
                return Err(EthApiError::InvalidParams(e.to_string()).into_rpc_err()); // likewise, map RPC error to your error type
            }
            Ok(Ok(v)) => v,
        };

        Ok(validation_result)
    }
}
