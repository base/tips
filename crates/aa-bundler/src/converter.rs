//! Convert UserOperations to EntryPoint transactions

use alloy_primitives::{Address, Bytes};
use alloy_signer_local::PrivateKeySigner;
use anyhow::Result;
use tracing::info;

use crate::types::{UserOperation, UserOperationMessage};

pub struct UserOperationConverter {
    bundler_signer: PrivateKeySigner,
    #[allow(dead_code)]
    chain_id: u64,
}

impl UserOperationConverter {
    pub fn new(bundler_private_key: &str, chain_id: u64) -> Result<Self> {
        let bundler_signer: PrivateKeySigner = bundler_private_key.parse()?;
        
        info!(
            bundler_address = %bundler_signer.address(),
            chain_id = chain_id,
            "UserOperation converter initialized"
        );

        Ok(Self {
            bundler_signer,
            chain_id,
        })
    }

    /// Convert a UserOperation into a transaction that calls EntryPoint.handleOps()
    ///
    /// This is a placeholder implementation. A full implementation would:
    /// 1. ABI encode the UserOperation struct according to EIP-4337
    /// 2. Build calldata for EntryPoint.handleOps([userOp], beneficiary)
    /// 3. Create transaction to entry point with this calldata
    /// 4. Sign with bundler key
    /// 5. Encode as Bytes
    pub fn convert_to_transaction(
        &self,
        user_op_message: &UserOperationMessage,
    ) -> Result<Bytes> {
        let user_op = &user_op_message.user_operation;
        let entry_point = user_op_message.entry_point;

        info!(
            sender = %user_op.sender(),
            entry_point = %entry_point,
            bundler = %self.bundler_signer.address(),
            version = user_op.version(),
            nonce = %user_op.nonce(),
            "Converting UserOperation to EntryPoint transaction"
        );

        match user_op {
            UserOperation::V06(op) => {
                info!(
                    sender = %op.sender,
                    nonce = %op.nonce,
                    call_gas_limit = %op.call_gas_limit,
                    verification_gas_limit = %op.verification_gas_limit,
                    max_fee_per_gas = %op.max_fee_per_gas,
                    "Successfully received v0.6 UserOperation"
                );
                // TODO: Implement v0.6 conversion
                // - ABI encode UserOperation struct
                // - Build handleOps([userOp], beneficiary) calldata
                // - Create and sign transaction
                
                // For now, return a placeholder empty bytes
                Ok(Bytes::new())
            }
            UserOperation::V07(op) => {
                info!(
                    sender = %op.sender,
                    nonce = %op.nonce,
                    call_gas_limit = %op.call_gas_limit,
                    verification_gas_limit = %op.verification_gas_limit,
                    max_fee_per_gas = %op.max_fee_per_gas,
                    "Successfully received v0.7 UserOperation"
                );
                // TODO: Implement v0.7 conversion
                // - Convert to PackedUserOperation
                // - ABI encode
                // - Build handleOps calldata
                // - Create and sign transaction
                
                // For now, return a placeholder empty bytes
                Ok(Bytes::new())
            }
        }
    }

    pub fn bundler_address(&self) -> Address {
        self.bundler_signer.address()
    }
}

