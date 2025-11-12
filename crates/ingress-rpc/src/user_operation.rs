//! EIP-4337 Account Abstraction User Operation types

use alloy_primitives::{Address, Bytes, B256, U256};
use serde::{Deserialize, Serialize};

/// User Operation as defined by EIP-4337 v0.6
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserOperationV06 {
    pub sender: Address,
    pub nonce: U256,
    pub init_code: Bytes,
    pub call_data: Bytes,
    pub call_gas_limit: U256,
    pub verification_gas_limit: U256,
    pub pre_verification_gas: U256,
    pub max_fee_per_gas: U256,
    pub max_priority_fee_per_gas: U256,
    pub paymaster_and_data: Bytes,
    pub signature: Bytes,
}

/// User Operation as defined by EIP-4337 v0.7+
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserOperationV07 {
    pub sender: Address,
    pub nonce: U256,
    pub factory: Address,
    pub factory_data: Bytes,
    pub call_data: Bytes,
    pub call_gas_limit: U256,
    pub verification_gas_limit: U256,
    pub pre_verification_gas: U256,
    pub max_fee_per_gas: U256,
    pub max_priority_fee_per_gas: U256,
    pub paymaster: Address,
    pub paymaster_verification_gas_limit: U256,
    pub paymaster_post_op_gas_limit: U256,
    pub paymaster_data: Bytes,
    pub signature: Bytes,
}

/// User Operation that can be either v0.6 or v0.7+
/// Automatically deserializes based on fields present
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UserOperation {
    V06(UserOperationV06),
    V07(UserOperationV07),
}

impl UserOperation {
    /// Get the sender address
    pub fn sender(&self) -> Address {
        match self {
            UserOperation::V06(op) => op.sender,
            UserOperation::V07(op) => op.sender,
        }
    }

    /// Get the nonce
    pub fn nonce(&self) -> U256 {
        match self {
            UserOperation::V06(op) => op.nonce,
            UserOperation::V07(op) => op.nonce,
        }
    }
}

