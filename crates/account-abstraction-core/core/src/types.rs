use crate::userop::{
    compute_domain_separator, encode_packed_user_operation, encode_user_operation,
    to_typed_data_hash,
};
use alloy_primitives::{Address, B256, U256, keccak256};
use alloy_rpc_types::erc4337;
pub use alloy_rpc_types::erc4337::SendUserOperationResponse;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum VersionedUserOperation {
    EntryPointV06(erc4337::UserOperation),
    EntryPointV07(erc4337::PackedUserOperation),
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]

pub struct UserOperationRequest {
    pub user_operation: VersionedUserOperation,
    pub entry_point: Address,
}

impl UserOperationRequest {
    pub fn hash(&self) -> B256 {
        let chain_id = 0x2105; // TODO: LOGIC TO GET CHAIN ID
        let abiEncoded = self.encode_user_operation(&self.user_operation);
        let hashedUserOperation = keccak256(&abiEncoded);
        let domainSeparator = compute_domain_separator(chain_id, self.entry_point);
        to_typed_data_hash(domainSeparator, hashedUserOperation)
    }

    pub fn encode_user_operation(&self, op: &VersionedUserOperation) -> Vec<u8> {
        match op {
            VersionedUserOperation::EntryPointV06(user_op) => {
                encode_user_operation(user_op).to_vec()
            }
            VersionedUserOperation::EntryPointV07(user_op) => {
                let overrideInitCodeHash = B256::ZERO; // TODO: LOGIC TO GET OVERRIDE INIT CODE HASH
                encode_packed_user_operation(user_op, overrideInitCodeHash).to_vec()
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserOperationRequestValidationResult {
    pub expiration_timestamp: u64,
    pub hash: B256,
    pub gas_used: U256,
}

// Tests
#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use alloy_primitives::{Address, Bytes, Uint};
    #[test]
    fn should_throw_error_when_deserializing_invalid() {
        const TEST_INVALID_USER_OPERATION: &str = r#"
        {
        "type": "EntryPointV06",
        "sender": "0x1111111111111111111111111111111111111111",
        "nonce": "0x0",
        "callGasLimit": "0x5208"
        }
    "#;
        let user_operation: Result<VersionedUserOperation, serde_json::Error> =
            serde_json::from_str::<VersionedUserOperation>(TEST_INVALID_USER_OPERATION);
        assert!(user_operation.is_err());
    }

    #[test]
    fn should_deserialize_v06() {
        const TEST_USER_OPERATION: &str = r#"
        {
            "type": "EntryPointV06",
            "sender": "0x1111111111111111111111111111111111111111",
            "nonce": "0x0",
            "initCode": "0x",
            "callData": "0x",
            "callGasLimit": "0x5208",
            "verificationGasLimit": "0x100000",
            "preVerificationGas": "0x10000",
            "maxFeePerGas": "0x59682f10",
            "maxPriorityFeePerGas": "0x3b9aca00",
            "paymasterAndData": "0x",
            "signature": "0x01"
        }
    "#;
        let user_operation: Result<VersionedUserOperation, serde_json::Error> =
            serde_json::from_str::<VersionedUserOperation>(TEST_USER_OPERATION);
        if user_operation.is_err() {
            panic!("Error: {:?}", user_operation.err());
        }
        let user_operation = user_operation.unwrap();
        match user_operation {
            VersionedUserOperation::EntryPointV06(user_operation) => {
                assert_eq!(
                    user_operation.sender,
                    Address::from_str("0x1111111111111111111111111111111111111111").unwrap()
                );
                assert_eq!(user_operation.nonce, Uint::from(0));
                assert_eq!(user_operation.init_code, Bytes::from_str("0x").unwrap());
                assert_eq!(user_operation.call_data, Bytes::from_str("0x").unwrap());
                assert_eq!(user_operation.call_gas_limit, Uint::from(0x5208));
                assert_eq!(user_operation.verification_gas_limit, Uint::from(0x100000));
                assert_eq!(user_operation.pre_verification_gas, Uint::from(0x10000));
                assert_eq!(user_operation.max_fee_per_gas, Uint::from(0x59682f10));
                assert_eq!(
                    user_operation.max_priority_fee_per_gas,
                    Uint::from(0x3b9aca00)
                );
                assert_eq!(
                    user_operation.paymaster_and_data,
                    Bytes::from_str("0x").unwrap()
                );
                assert_eq!(user_operation.signature, Bytes::from_str("0x01").unwrap());
            }
            _ => {
                panic!("Expected EntryPointV06, got {:?}", user_operation);
            }
        }
    }

    #[test]
    fn should_deserialize_v07() {
        const TEST_PACKED_USER_OPERATION: &str = r#"
        {
        "type": "EntryPointV07",
        "sender": "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
        "nonce": "0x1",
        "factory": "0x2222222222222222222222222222222222222222",
        "factoryData": "0xabcdef1234560000000000000000000000000000000000000000000000000000",
        "callData": "0xb61d27f600000000000000000000000000000000000000000000000000000000000000c8",
        "callGasLimit": "0x2dc6c0",
        "verificationGasLimit": "0x1e8480",
        "preVerificationGas": "0x186a0",
        "maxFeePerGas": "0x77359400",
        "maxPriorityFeePerGas": "0x3b9aca00",
        "paymaster": "0x3333333333333333333333333333333333333333",
        "paymasterVerificationGasLimit": "0x186a0",
        "paymasterPostOpGasLimit": "0x27100",
        "paymasterData": "0xfafb00000000000000000000000000000000000000000000000000000000000064",
        "signature": "0xa3c5f1b90014e68abbbdc42e4b77b9accc0b7e1c5d0b5bcde1a47ba8faba00ff55c9a7de12e98b731766e35f6c51ab25c9b58cc0e7c4a33f25e75c51c6ad3c3a"
        }
    "#;
        let user_operation: Result<VersionedUserOperation, serde_json::Error> =
            serde_json::from_str::<VersionedUserOperation>(TEST_PACKED_USER_OPERATION);
        if user_operation.is_err() {
            panic!("Error: {:?}", user_operation.err());
        }
        let user_operation = user_operation.unwrap();
        match user_operation {
            VersionedUserOperation::EntryPointV07(user_operation) => {
                assert_eq!(
                    user_operation.sender,
                    Address::from_str("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48").unwrap()
                );
                assert_eq!(user_operation.nonce, Uint::from(1));
                assert_eq!(
                    user_operation.call_data,
                    alloy_primitives::bytes!(
                        "0xb61d27f600000000000000000000000000000000000000000000000000000000000000c8"
                    )
                );
                assert_eq!(user_operation.call_gas_limit, Uint::from(0x2dc6c0));
                assert_eq!(user_operation.verification_gas_limit, Uint::from(0x1e8480));
                assert_eq!(user_operation.pre_verification_gas, Uint::from(0x186a0));
            }
            _ => {
                panic!("Expected EntryPointV07, got {:?}", user_operation);
            }
        }
    }

    #[test]
    fn should_compute_user_op_hash() {
        let user_operation = VersionedUserOperation::EntryPointV07(PackedUserOperation {
            sender: Address::from_str("0x6A84A0cF27291eF29042305547a898D3861F05f3").unwrap(),
            nonce: U256::from(34478152104024148605889140946955407093471678057486667436843787709996345589760),
            init_code: Bytes::from_str("0x").unwrap(),
            call_data: Bytes::from_str("0x34fcd5be000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000020000000000000000000000000a6c9ba866992cfd7fd6460ba912bfa405ada9df00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000006000000000000000000000000000000000000000000000000000000000000000a4712ee07a000000000000000000000000000000000000000000000000000000003bad06880000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000006000000000000000000000000000000000000000000000000000000000000000204a6f6220313030303432303134302064656c697665727920616363657074656400000000000000000000000000000000000000000000000000000000").unwrap(),
            call_gas_limit: Uint::from(0x5208),
            verification_gas_limit: Uint::from(0x100000),
            pre_verification_gas: Uint::from(0x10000),
            max_fee_per_gas: Uint::from(0x59682f10),
            max_priority_fee_per_gas: Uint::from(0x3b9aca00),
            paymaster_and_data: Bytes::from_str("0x2cc0c7981d846b9f2a16276556f6e8cb52bfb63300000000000000000000000000007f720000000000000000000000000000000000000000000000006931c09b529f9957ccdded5c5cd49d33774020d10c3c6186d0ab497447f3d75f660d001c0c357f6f89ca71d4dc207384efe21b9e4b912c72144cc591935165ce9114c5f21b").unwrap(),
            signature: Bytes::from_str("0xff00b5d2030ee303b9098a70a89aac8a3cc267e38da8029751a48fc59193aa90de9540772784a825e1df2c012730c75a186f5b62c1eb702c6b60c9c536d5610382751c").unwrap(),
        });
    }
}
