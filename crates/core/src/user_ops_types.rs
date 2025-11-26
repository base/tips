use alloy_primitives::{Bytes, Uint};
use serde::{Deserialize, Serialize};

// https://reth.rs/docs/reth/rpc/types/struct.PackedUserOperation.html
#[derive(Default, Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PackedUserOperation {
    pub sender: String,

    pub nonce: Uint<256, 4>,

    pub factory: Option<String>,
    pub factory_data: Option<Bytes>,

    pub call_data: Bytes,

    pub call_gas_limit: Uint<256, 4>,

    pub verification_gas_limit: Uint<256, 4>,

    pub pre_verification_gas: Uint<256, 4>,

    pub max_fee_per_gas: Uint<256, 4>,

    pub max_priority_fee_per_gas: Uint<256, 4>,

    pub paymaster: Option<String>,

    pub paymaster_verification_gas_limit: Option<Uint<256, 4>>,

    pub paymaster_post_op_gas_limit: Option<Uint<256, 4>>,

    pub paymaster_data: Option<Bytes>,
    pub signature: Bytes,
}

// // https://reth.rs/docs/reth/srpc/types/struct.UserOperation.html
// crates/core/src/user_ops_types.rs

#[derive(Default, Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UserOperation {
    pub sender: Option<String>,

    pub nonce: Uint<256, 4>,

    pub factory: Option<String>,
    pub factory_data: Option<Bytes>,

    pub call_data: Bytes,

    pub call_gas_limit: Uint<256, 4>,

    pub verification_gas_limit: Uint<256, 4>,

    pub pre_verification_gas: Uint<256, 4>,

    pub max_fee_per_gas: Uint<256, 4>,

    pub max_priority_fee_per_gas: Uint<256, 4>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum UserOperationRequest {
    EntryPointV06(UserOperation),
    EntryPointV07(PackedUserOperation),
}


// Tests
#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    #[test]
    fn should_deserialize_user_operation() {
        const TEST_USER_OPERATION: &str = r#"
    {
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
        let user_operation: Result<UserOperation, serde_json::Error> =
            serde_json::from_str::<UserOperation>(TEST_USER_OPERATION);
            match user_operation {
                Ok(user_operation) => {
                    assert_eq!(user_operation.sender, Some("0x1111111111111111111111111111111111111111".to_string()));
                    assert_eq!(user_operation.nonce, Uint::from(0));
                    assert_eq!(user_operation.factory, None);
                    assert_eq!(user_operation.factory_data, None);
                    assert_eq!(user_operation.call_data, alloy_primitives::bytes!("0x"));
                    assert_eq!(user_operation.call_gas_limit, Uint::from(0x5208));
                    assert_eq!(user_operation.verification_gas_limit, Uint::from(0x100000));
                    assert_eq!(user_operation.pre_verification_gas, Uint::from(0x10000));
                }
                Err(e) => {
                    panic!("Error: {:?}", e);
                }
            }

       
    }

    #[test]
    fn should_throw_error_when_deserializing_invalid_user_operation() {
        const TEST_INVALID_USER_OPERATION: &str = r#"
        {
            "sender": "0x1111111111111111111111111111111111111111",
            "nonce": "0x0",
            "callGasLimit": "0x5208"
        }
    "#;
        let user_operation: Result<UserOperation, serde_json::Error> =
            serde_json::from_str::<UserOperation>(TEST_INVALID_USER_OPERATION);
        assert!(user_operation.is_err());
    }

    #[test]
    fn should_deserialize_user_operation_when_tag_is_entry_point_v06() {
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
        let user_operation: Result<UserOperation, serde_json::Error> =
            serde_json::from_str::<UserOperation>(TEST_USER_OPERATION);
        match user_operation {
            Ok(user_operation) => {
                assert_eq!(user_operation.sender, Some("0x1111111111111111111111111111111111111111".to_string()));
                assert_eq!(user_operation.nonce, Uint::from(0));
                assert_eq!(user_operation.factory, None);
                assert_eq!(user_operation.factory_data, None);
                assert_eq!(user_operation.call_data, alloy_primitives::bytes!("0x"));
                assert_eq!(user_operation.call_gas_limit, Uint::from(0x5208));
                assert_eq!(user_operation.verification_gas_limit, Uint::from(0x100000));
                assert_eq!(user_operation.pre_verification_gas, Uint::from(0x10000));
            }
            Err(e) => {
                panic!("Error: {:?}", e);
            }
        }
    }

    #[test]
    fn should_deserialize_user_operation_when_tag_is_entry_point_v07() {
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
        let user_operation: Result<UserOperation, serde_json::Error> =
            serde_json::from_str::<UserOperation>(TEST_PACKED_USER_OPERATION);
        match user_operation {
            Ok(user_operation) => {
                assert_eq!(user_operation.sender, Some("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".to_string()));
                assert_eq!(user_operation.nonce, Uint::from(1));
                assert_eq!(user_operation.factory, Some("0x2222222222222222222222222222222222222222".to_string()));
                assert_eq!(user_operation.factory_data, Some(alloy_primitives::bytes!("0xabcdef1234560000000000000000000000000000000000000000000000000000")));
                assert_eq!(user_operation.call_data, alloy_primitives::bytes!("0xb61d27f600000000000000000000000000000000000000000000000000000000000000c8"));
                assert_eq!(user_operation.call_gas_limit, Uint::from(0x2dc6c0));
                assert_eq!(user_operation.verification_gas_limit, Uint::from(0x1e8480));
                assert_eq!(user_operation.pre_verification_gas, Uint::from(0x186a0));
            }
            Err(e) => {
                panic!("Error: {:?}", e);
            }
        }

    }
}