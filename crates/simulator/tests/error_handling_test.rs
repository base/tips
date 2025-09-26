/// Unit tests for error handling scenarios
mod common;

use common::builders::*;
use common::fixtures::*;
use common::mock_bundle_simulator::MockBundleSimulator;
use common::mocks::*;
use tips_simulator::{core::BundleSimulator, SimulationError};

#[tokio::test]
async fn test_simulation_error_types() {
    // Test all error types get properly propagated
    let error_scenarios = vec![
        (
            SimulationError::Revert {
                reason: "Insufficient funds".to_string(),
            },
            "Insufficient funds",
        ),
        (SimulationError::OutOfGas, "out of gas"),
        (
            SimulationError::InvalidNonce {
                tx_index: 0,
                expected: 5,
                actual: 3,
            },
            "Invalid nonce",
        ),
        (
            SimulationError::InsufficientBalance {
                tx_index: 1,
                required: alloy_primitives::U256::from(1000),
                available: alloy_primitives::U256::from(500),
            },
            "Insufficient balance",
        ),
        (
            SimulationError::StateAccessError {
                message: "RPC timeout".to_string(),
            },
            "State access error",
        ),
        (SimulationError::Timeout, "timed out"),
        (
            SimulationError::Unknown {
                message: "Unexpected error".to_string(),
            },
            "Unexpected error",
        ),
    ];

    for (error, expected_msg) in error_scenarios {
        let engine = MockSimulationEngine::new().fail_next_with(error.clone());
        let publisher = MockSimulationPublisher::new();
        let simulator = MockBundleSimulator::new(engine, publisher.clone());

        let request = SimulationRequestBuilder::new().build();

        // Execute
        simulator.simulate(&request).await.unwrap();

        // Verify
        let published = publisher.get_published();
        assert_eq!(published.len(), 1);
        assert!(!published[0].success);
        assert!(published[0]
            .error_reason
            .as_ref()
            .unwrap()
            .contains(expected_msg));
    }
}

#[tokio::test]
async fn test_publisher_failure_recovery() {
    // Test that publisher failures don't crash the simulator
    let engine = MockSimulationEngine::new();
    let publisher = MockSimulationPublisher::new();
    let simulator = MockBundleSimulator::new(engine.clone(), publisher.clone());

    // First simulation succeeds
    let request1 = SimulationRequestBuilder::new().build();
    simulator.simulate(&request1).await.unwrap();
    assert_eq!(publisher.published_count(), 1);

    // Configure publisher to fail next
    let publisher2 = publisher.clone().fail_next();
    let simulator2 = MockBundleSimulator::new(engine.clone(), publisher2.clone());

    // Second simulation - publisher fails but simulator continues
    let request2 = SimulationRequestBuilder::new().build();
    simulator2.simulate(&request2).await.unwrap();
    assert_eq!(publisher2.published_count(), 1); // Still 1 from first simulation, second failed

    // Third simulation - publisher recovers
    let request3 = SimulationRequestBuilder::new().build();
    simulator2.simulate(&request3).await.unwrap();
    assert_eq!(publisher2.published_count(), 2); // Now 2: first succeeded, second failed, third succeeded
}

#[tokio::test]
async fn test_engine_failure_recovery() {
    // Test that engine failures are handled gracefully
    let engine = MockSimulationEngine::new();
    let publisher = MockSimulationPublisher::new();

    // Test simulation 1: Out of gas error
    let engine1 = engine.clone().fail_next_with(SimulationError::OutOfGas);
    let simulator1 = MockBundleSimulator::new(engine1, publisher.clone());
    let request1 = SimulationRequestBuilder::new()
        .with_bundle(
            TestBundleBuilder::new()
                .with_simple_transaction(&[1])
                .build(),
        )
        .build();
    simulator1.simulate(&request1).await.unwrap();

    // Test simulation 2: Success
    let engine2 = engine
        .clone()
        .with_result(SimulationResultBuilder::successful().build());
    let simulator2 = MockBundleSimulator::new(engine2, publisher.clone());
    let request2 = SimulationRequestBuilder::new()
        .with_bundle(
            TestBundleBuilder::new()
                .with_simple_transaction(&[2])
                .build(),
        )
        .build();
    simulator2.simulate(&request2).await.unwrap();

    // Test simulation 3: Revert error
    let engine3 = engine.clone().fail_next_with(SimulationError::Revert {
        reason: "Test revert".to_string(),
    });
    let simulator3 = MockBundleSimulator::new(engine3, publisher.clone());
    let request3 = SimulationRequestBuilder::new()
        .with_bundle(
            TestBundleBuilder::new()
                .with_simple_transaction(&[3])
                .build(),
        )
        .build();
    simulator3.simulate(&request3).await.unwrap();

    // Verify all were published despite failures
    let published = publisher.get_published();
    assert_eq!(published.len(), 3);

    // First should fail with out of gas
    assert!(!published[0].success);
    assert!(published[0]
        .error_reason
        .as_ref()
        .unwrap()
        .contains("Bundle ran out of gas"));

    // Second should succeed (from pre-configured result)
    assert!(published[1].success);

    // Third should fail with revert
    assert!(!published[2].success);
    assert!(published[2]
        .error_reason
        .as_ref()
        .unwrap()
        .contains("Bundle reverted"));
}

#[tokio::test]
async fn test_invalid_bundle_handling() {
    // Test handling of various invalid bundle scenarios
    let engine = MockSimulationEngine::new();
    let publisher = MockSimulationPublisher::new();
    let simulator = MockBundleSimulator::new(engine.clone(), publisher.clone());

    // Empty bundle
    let empty_bundle = TestBundleBuilder::new().build();
    let request = SimulationRequestBuilder::new()
        .with_bundle(empty_bundle)
        .build();

    simulator.simulate(&request).await.unwrap();
    assert_eq!(publisher.published_count(), 1);

    // Bundle with invalid block number (future block)
    let future_bundle = TestBundleBuilder::new()
        .with_simple_transaction(&[0x01])
        .with_block_number(99_999_999)
        .build();
    let future_request = SimulationRequestBuilder::new()
        .with_bundle(future_bundle)
        .with_block(99_999_999, alloy_primitives::B256::random())
        .build();

    simulator.simulate(&future_request).await.unwrap();
    assert_eq!(publisher.published_count(), 2);
}

#[tokio::test]
async fn test_concurrent_error_handling() {
    // Test error handling under concurrent load
    use std::sync::Arc;

    let engine = MockSimulationEngine::new();
    let publisher = MockSimulationPublisher::new();
    let simulator = Arc::new(MockBundleSimulator::new(engine.clone(), publisher.clone()));

    // Provider factory no longer needed with new architecture

    let mut handles = vec![];

    // Spawn multiple tasks, some will fail
    for i in 0..10 {
        let sim = Arc::clone(&simulator);
        // Provider factory no longer needed
        let eng = engine.clone();

        let handle = tokio::spawn(async move {
            // Every third simulation fails
            if i % 3 == 0 {
                let _ = eng.fail_next_with(SimulationError::Timeout);
            }

            let request = SimulationRequestBuilder::new()
                .with_bundle(
                    TestBundleBuilder::new()
                        .with_simple_transaction(&[i as u8])
                        .build(),
                )
                .build();

            sim.simulate(&request).await
        });

        handles.push(handle);
    }

    // Wait for all to complete
    let mut results = vec![];
    for handle in handles {
        results.push(handle.await.unwrap());
    }

    // All should complete without panicking
    assert!(results.iter().all(|r: &eyre::Result<()>| r.is_ok()));
    assert_eq!(publisher.published_count(), 10);

    // Verify mix of successes and failures
    let published = publisher.get_published();
    let failures = published.iter().filter(|r| !r.success).count();
    assert!(failures > 0);
}

#[test]
fn test_error_display_formatting() {
    // Verify error messages are properly formatted
    let errors = vec![
        (
            SimulationError::Revert {
                reason: "ERC20: transfer amount exceeds balance".to_string(),
            },
            "Bundle reverted: ERC20: transfer amount exceeds balance",
        ),
        (
            SimulationError::InvalidNonce {
                tx_index: 0,
                expected: 10,
                actual: 5,
            },
            "Invalid nonce in tx 0: expected 10, got 5",
        ),
        (
            SimulationError::InsufficientBalance {
                tx_index: 2,
                required: alloy_primitives::U256::from(1_000_000),
                available: alloy_primitives::U256::from(500_000),
            },
            "Insufficient balance in tx 2: required 1000000, available 500000",
        ),
    ];

    for (error, expected) in errors {
        assert_eq!(error.to_string(), expected);
    }
}

#[tokio::test]
async fn test_timeout_simulation() {
    // Test timeout error handling
    let engine = MockSimulationEngine::new().fail_next_with(SimulationError::Timeout);
    let publisher = MockSimulationPublisher::new();
    let simulator = MockBundleSimulator::new(engine, publisher.clone());

    let large_bundle = bundles::large_bundle(1000); // Very large bundle
    let request = SimulationRequestBuilder::new()
        .with_bundle(large_bundle)
        .build();

    // Execute
    let result = simulator.simulate(&request).await;

    // Should complete successfully even with timeout
    assert!(result.is_ok());

    let published = publisher.get_published();
    assert_eq!(published.len(), 1);
    assert!(!published[0].success);
    assert_eq!(
        published[0].error_reason,
        Some("Simulation timed out".to_string())
    );
}
