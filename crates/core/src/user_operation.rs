//! EIP-4337 Account Abstraction User Operation types
use alloy_primitives::{Address, Bytes, U256, B256, keccak256};
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

    /// Calculate the user operation hash (for use as Kafka key and tracking)
    /// This is a simplified hash - for production, use the full EIP-4337 hash algorithm
    pub fn user_op_hash(&self, entry_point: &Address, chain_id: u64) -> B256 {
        let mut data = Vec::new();
        
        // Include chain ID and entry point
        data.extend_from_slice(&chain_id.to_be_bytes());
        data.extend_from_slice(entry_point.as_slice());
        
        match self {
            UserOperation::V06(op) => {
                data.extend_from_slice(op.sender.as_slice());
                data.extend_from_slice(&op.nonce.to_be_bytes::<32>());
                data.extend_from_slice(&keccak256(&op.init_code).0);
                data.extend_from_slice(&keccak256(&op.call_data).0);
                data.extend_from_slice(&op.call_gas_limit.to_be_bytes::<32>());
                data.extend_from_slice(&op.verification_gas_limit.to_be_bytes::<32>());
                data.extend_from_slice(&op.pre_verification_gas.to_be_bytes::<32>());
                data.extend_from_slice(&op.max_fee_per_gas.to_be_bytes::<32>());
                data.extend_from_slice(&op.max_priority_fee_per_gas.to_be_bytes::<32>());
                data.extend_from_slice(&keccak256(&op.paymaster_and_data).0);
            }
            UserOperation::V07(op) => {
                data.extend_from_slice(op.sender.as_slice());
                data.extend_from_slice(&op.nonce.to_be_bytes::<32>());
                data.extend_from_slice(op.factory.as_slice());
                data.extend_from_slice(&keccak256(&op.factory_data).0);
                data.extend_from_slice(&keccak256(&op.call_data).0);
                data.extend_from_slice(&op.call_gas_limit.to_be_bytes::<32>());
                data.extend_from_slice(&op.verification_gas_limit.to_be_bytes::<32>());
                data.extend_from_slice(&op.pre_verification_gas.to_be_bytes::<32>());
                data.extend_from_slice(&op.max_fee_per_gas.to_be_bytes::<32>());
                data.extend_from_slice(&op.max_priority_fee_per_gas.to_be_bytes::<32>());
                data.extend_from_slice(op.paymaster.as_slice());
                data.extend_from_slice(&op.paymaster_verification_gas_limit.to_be_bytes::<32>());
                data.extend_from_slice(&op.paymaster_post_op_gas_limit.to_be_bytes::<32>());
                data.extend_from_slice(&keccak256(&op.paymaster_data).0);
            }
        }
        
        keccak256(&data)
    }
}

/// Wrapper for UserOperation with metadata for Kafka queue
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserOperationWithMetadata {
    pub user_operation: UserOperation,
    pub entry_point: Address,
    pub user_op_hash: B256,
    pub received_at: u64, // Unix timestamp
    pub chain_id: u64,
}

impl UserOperationWithMetadata {
    pub fn new(
        user_operation: UserOperation,
        entry_point: Address,
        chain_id: u64,
    ) -> Self {
        let user_op_hash = user_operation.user_op_hash(&entry_point, chain_id);
        let received_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();

        Self {
            user_operation,
            entry_point,
            user_op_hash,
            received_at,
            chain_id,
        }
    }
}