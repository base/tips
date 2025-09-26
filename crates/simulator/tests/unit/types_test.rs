/// Unit tests for simulator types
use crate::common::builders::*;
use crate::common::fixtures::*;
use alloy_primitives::{Address, B256, U256};
use std::collections::HashMap;
use tips_simulator::types::{SimulationError, SimulationRequest, SimulationResult};
use uuid::Uuid;

#[test]
fn test_simulation_result_success_creation() {
    let id = Uuid::new_v4();
    let bundle_id = Uuid::new_v4();
    let block_hash = B256::random();
    let gas_used = 150_000;
    let execution_time = 1500;

    let mut state_diff = HashMap::new();
    let addr = Address::random();
    let mut storage = HashMap::new();
    storage.insert(U256::from(0), U256::from(100));
    state_diff.insert(addr, storage);

    let result = SimulationResult::success(
        id,
        bundle_id,
        18_000_000,
        block_hash,
        gas_used,
        execution_time,
        state_diff.clone(),
    );

    assert_eq!(result.id, id);
    assert_eq!(result.bundle_id, bundle_id);
    assert_eq!(result.block_number, 18_000_000);
    assert_eq!(result.block_hash, block_hash);
    assert!(result.success);
    assert_eq!(result.gas_used, Some(gas_used));
    assert_eq!(result.execution_time_us, execution_time);
    assert_eq!(result.state_diff.len(), 1);
    assert!(result.error_reason.is_none());
}

#[test]
fn test_simulation_result_failure_creation() {
    let id = Uuid::new_v4();
    let bundle_id = Uuid::new_v4();
    let block_hash = B256::random();
    let execution_time = 500;
    let error = SimulationError::Revert {
        reason: "Test revert".to_string(),
    };

    let result = SimulationResult::failure(
        id,
        bundle_id,
        18_000_000,
        block_hash,
        execution_time,
        error.clone(),
    );

    assert_eq!(result.id, id);
    assert_eq!(result.bundle_id, bundle_id);
    assert!(!result.success);
    assert!(result.gas_used.is_none());
    assert!(result.state_diff.is_empty());
    assert_eq!(result.error_reason, Some(error.to_string()));
}

#[test]
fn test_simulation_error_display() {
    let test_cases = vec![
        (
            SimulationError::Revert {
                reason: "Invalid state".to_string(),
            },
            "Bundle reverted: Invalid state",
        ),
        (SimulationError::OutOfGas, "Bundle ran out of gas"),
        (
            SimulationError::InvalidNonce {
                tx_index: 2,
                expected: 5,
                actual: 3,
            },
            "Invalid nonce in tx 2: expected 5, got 3",
        ),
        (
            SimulationError::InsufficientBalance {
                tx_index: 1,
                required: U256::from(1000),
                available: U256::from(500),
            },
            "Insufficient balance in tx 1: required 1000, available 500",
        ),
        (
            SimulationError::StateAccessError {
                message: "RPC timeout".to_string(),
            },
            "State access error: RPC timeout",
        ),
        (SimulationError::Timeout, "Simulation timed out"),
        (
            SimulationError::Unknown {
                message: "Something went wrong".to_string(),
            },
            "Unknown error: Something went wrong",
        ),
    ];

    for (error, expected) in test_cases {
        assert_eq!(error.to_string(), expected);
    }
}

#[test]
fn test_simulation_request_fields() {
    let bundle_id = Uuid::new_v4();
    let bundle = bundles::single_tx_bundle();
    let block_number = 18_000_000;
    let block_hash = B256::random();

    let request = SimulationRequest {
        bundle_id,
        bundle: bundle.clone(),
        block_number,
        block_hash,
    };

    assert_eq!(request.bundle_id, bundle_id);
    assert_eq!(request.bundle.txs.len(), bundle.txs.len());
    assert_eq!(request.block_number, block_number);
    assert_eq!(request.block_hash, block_hash);
}

#[test]
fn test_simulation_result_builder() {
    // Test successful result
    let success_result = SimulationResultBuilder::successful()
        .with_gas_used(250_000)
        .with_execution_time_us(2000)
        .with_state_change(*addresses::ALICE, U256::from(0), U256::from(500))
        .build();

    assert!(success_result.success);
    assert_eq!(success_result.gas_used, Some(250_000));
    assert_eq!(success_result.execution_time_us, 2000);
    assert!(success_result.state_diff.contains_key(&*addresses::ALICE));

    // Test failed result with revert
    let revert_result = SimulationResultBuilder::failed()
        .with_revert("Insufficient funds".to_string())
        .build();

    assert!(!revert_result.success);
    assert!(revert_result.gas_used.is_none());
    assert!(revert_result
        .error_reason
        .as_ref()
        .unwrap()
        .contains("Insufficient funds"));

    // Test failed result with out of gas
    let oog_result = SimulationResultBuilder::failed().with_out_of_gas().build();

    assert!(!oog_result.success);
    assert!(oog_result
        .error_reason
        .as_ref()
        .unwrap()
        .contains("out of gas"));

    // Test invalid nonce
    let nonce_result = SimulationResultBuilder::failed()
        .with_invalid_nonce(0, 5, 3)
        .build();

    assert!(!nonce_result.success);
    assert!(nonce_result
        .error_reason
        .as_ref()
        .unwrap()
        .contains("Invalid nonce"));
}

#[test]
fn test_simulation_result_timestamp() {
    let result = SimulationResultBuilder::successful().build();

    // Check that timestamp is recent (within last minute)
    let now = chrono::Utc::now();
    let created_timestamp = result.created_at.timestamp();
    let now_timestamp = now.timestamp();
    let diff = now_timestamp - created_timestamp;
    assert!(diff < 60);
}

#[test]
fn test_large_state_diff() {
    let mut builder = SimulationResultBuilder::successful();

    // Add many state changes
    for i in 0..100 {
        let addr = Address::random();
        for j in 0..10 {
            builder = builder.with_state_change(addr, U256::from(j), U256::from(i * 1000 + j));
        }
    }

    let result = builder.build();
    assert_eq!(result.state_diff.len(), 100);

    // Verify each account has 10 storage slots
    for (_, storage) in &result.state_diff {
        assert_eq!(storage.len(), 10);
    }
}

#[test]
fn test_error_serialization() {
    // Verify that errors can be converted to strings and back
    let errors = vec![
        SimulationError::Revert {
            reason: "test".to_string(),
        },
        SimulationError::OutOfGas,
        SimulationError::InvalidNonce {
            tx_index: 1,
            expected: 2,
            actual: 3,
        },
        SimulationError::Timeout,
    ];

    for error in errors {
        let error_string = error.to_string();
        assert!(!error_string.is_empty());

        // Create a result with this error
        let result = SimulationResult::failure(
            Uuid::new_v4(),
            Uuid::new_v4(),
            18_000_000,
            B256::random(),
            1000,
            error,
        );

        assert_eq!(result.error_reason, Some(error_string));
    }
}

#[test]
fn test_simulation_result_gas_used_bounds() {
    // Test with maximum gas
    let max_gas_result = SimulationResultBuilder::successful()
        .with_gas_used(30_000_000) // 30M gas
        .build();

    assert_eq!(max_gas_result.gas_used, Some(30_000_000));

    // Test with zero gas (edge case)
    let zero_gas_result = SimulationResultBuilder::successful()
        .with_gas_used(0)
        .build();

    assert_eq!(zero_gas_result.gas_used, Some(0));
}
