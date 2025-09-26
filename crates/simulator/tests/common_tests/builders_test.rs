use crate::common::builders::*;
use alloy_primitives::{Address, B256, U256};
use uuid::Uuid;

#[test]
fn test_bundle_builder() {
    let bundle = TestBundleBuilder::new()
        .with_simple_transaction(&[0x01, 0x02])
        .with_simple_transaction(&[0x03, 0x04])
        .with_block_number(18_500_000)
        .with_timestamps(1000, 2000)
        .build();

    assert_eq!(bundle.txs.len(), 2);
    assert_eq!(bundle.block_number, 18_500_000);
    assert_eq!(bundle.min_timestamp, Some(1000));
    assert_eq!(bundle.max_timestamp, Some(2000));
}

#[test]
fn test_result_builder() {
    let bundle_id = Uuid::new_v4();
    let result = SimulationResultBuilder::successful()
        .with_ids(Uuid::new_v4(), bundle_id)
        .with_gas_used(200_000)
        .with_state_change(Address::random(), U256::from(1), U256::from(100))
        .build();

    assert!(result.success);
    assert_eq!(result.bundle_id, bundle_id);
    assert_eq!(result.gas_used, Some(200_000));
    assert!(!result.state_diff.is_empty());
}

#[test]
fn test_simulation_result_builder_comprehensive() {
    // Test successful result
    let success_result = SimulationResultBuilder::successful()
        .with_gas_used(250_000)
        .with_execution_time_us(2000)
        .with_state_change(Address::random(), U256::from(0), U256::from(500))
        .build();

    assert!(success_result.success);
    assert_eq!(success_result.gas_used, Some(250_000));
    assert_eq!(success_result.execution_time_us, 2000);
    assert!(!success_result.state_diff.is_empty());

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
fn test_scenario_builder() {
    let requests = ScenarioBuilder::new()
        .with_block(19_000_000, B256::random())
        .add_simple_bundle(2)
        .add_simple_bundle(3)
        .build_requests();

    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].block_number, 19_000_000);
    assert_eq!(requests[0].bundle.txs.len(), 2);
    assert_eq!(requests[1].bundle.txs.len(), 3);
}
