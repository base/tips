// Tests for the concrete BundleSimulator implementation
mod common;

use common::builders::*;
use common::fixtures::*;
use common::mocks::*;
use tips_simulator::{core::BundleSimulator, core::BundleSimulatorImpl, SimulationError};
use uuid::Uuid;

#[tokio::test]
async fn test_bundle_simulator_impl_successful_flow() {
    // Setup - exercise the concrete BundleSimulator implementation
    let bundle_id = Uuid::new_v4();
    let expected_result = SimulationResultBuilder::successful()
        .with_ids(Uuid::new_v4(), bundle_id)
        .with_gas_used(250_000)
        .with_execution_time_us(2500)
        .build();

    let engine = MockSimulationEngine::new().with_result(expected_result.clone());
    let publisher = MockSimulationPublisher::new();
    let simulator = BundleSimulatorImpl::new(engine.clone(), publisher.clone());

    let request = SimulationRequestBuilder::new()
        .with_bundle_id(bundle_id)
        .with_bundle(bundles::single_tx_bundle())
        .build();

    // Act - test the actual BundleSimulator trait implementation
    let result = simulator.simulate(&request).await;

    // Assert
    assert!(result.is_ok());
    assert_eq!(engine.simulation_count(), 1);
    assert_eq!(publisher.published_count(), 1);

    let published = publisher.get_published();
    assert_eq!(published[0].bundle_id, bundle_id);
    assert_eq!(published[0].gas_used, Some(250_000));
    assert!(published[0].success);
}

#[tokio::test]
async fn test_bundle_simulator_impl_engine_failure() {
    // Test that the concrete BundleSimulator handles engine failures
    let bundle_id = Uuid::new_v4();
    let engine = MockSimulationEngine::new().fail_next_with(SimulationError::OutOfGas);
    let publisher = MockSimulationPublisher::new();
    let simulator = BundleSimulatorImpl::new(engine.clone(), publisher.clone());

    let request = SimulationRequestBuilder::new()
        .with_bundle_id(bundle_id)
        .build();

    // Act
    let result = simulator.simulate(&request).await;

    // Assert - simulate() should succeed even if the engine simulation fails
    assert!(result.is_ok());
    assert_eq!(engine.simulation_count(), 1);
    assert_eq!(publisher.published_count(), 1);

    let published = publisher.get_published();
    assert!(!published[0].success);
    assert!(published[0].error_reason.is_some());
}

#[tokio::test]
async fn test_bundle_simulator_impl_publisher_failure() {
    // Test that the concrete BundleSimulator handles publisher failures gracefully
    let engine = MockSimulationEngine::new();
    let publisher = MockSimulationPublisher::new().fail_next();
    let simulator = BundleSimulatorImpl::new(engine.clone(), publisher.clone());

    let request = SimulationRequestBuilder::new().build();

    // Act - should complete without error even if publisher fails
    let result = simulator.simulate(&request).await;

    // Assert
    assert!(result.is_ok());
    assert_eq!(engine.simulation_count(), 1);
    assert_eq!(publisher.published_count(), 0); // Publisher failed
}

#[tokio::test]
async fn test_bundle_simulator_impl_multiple_simulations() {
    // Test the concrete BundleSimulator with multiple sequential simulations
    let engine = MockSimulationEngine::new();
    let publisher = MockSimulationPublisher::new();
    let simulator = BundleSimulatorImpl::new(engine.clone(), publisher.clone());

    // Run multiple simulations with different bundle IDs
    for i in 0..3 {
        let bundle_id = Uuid::new_v4();
        let result = SimulationResultBuilder::successful()
            .with_ids(Uuid::new_v4(), bundle_id)
            .with_gas_used(100_000 + i * 50_000)
            .build();

        let _ = engine.clone().with_result(result);

        let request = SimulationRequestBuilder::new()
            .with_bundle_id(bundle_id)
            .with_bundle(
                TestBundleBuilder::new()
                    .with_simple_transaction(&[i as u8, 0x01, 0x02])
                    .build(),
            )
            .build();

        let sim_result = simulator.simulate(&request).await;
        assert!(sim_result.is_ok());
    }

    // Verify all simulations were processed
    assert_eq!(engine.simulation_count(), 3);
    assert_eq!(publisher.published_count(), 3);

    let published = publisher.get_published();
    assert_eq!(published.len(), 3);
    for (i, result) in published.iter().enumerate() {
        assert!(result.success);
        assert_eq!(result.gas_used, Some(100_000 + i as u64 * 50_000));
    }
}

#[tokio::test]
async fn test_bundle_simulator_impl_various_error_types() {
    // Test the concrete BundleSimulator with different types of simulation errors
    let errors = vec![
        SimulationError::Revert {
            reason: "Contract reverted".to_string(),
        },
        SimulationError::InvalidNonce {
            tx_index: 1,
            expected: 10,
            actual: 5,
        },
        SimulationError::InsufficientBalance {
            tx_index: 0,
            required: alloy_primitives::U256::from(1000000),
            available: alloy_primitives::U256::from(500000),
        },
        SimulationError::StateAccessError {
            message: "RPC timeout".to_string(),
        },
        SimulationError::Timeout,
    ];

    for (_i, error) in errors.into_iter().enumerate() {
        let engine = MockSimulationEngine::new().fail_next_with(error.clone());
        let publisher = MockSimulationPublisher::new();
        let simulator = BundleSimulatorImpl::new(engine.clone(), publisher.clone());

        let request = SimulationRequestBuilder::new()
            .with_bundle_id(Uuid::new_v4())
            .build();

        let result = simulator.simulate(&request).await;
        assert!(result.is_ok());

        let published = publisher.get_published();
        assert_eq!(published.len(), 1);
        assert!(!published[0].success);

        let error_msg = published[0].error_reason.as_ref().unwrap();
        match error {
            SimulationError::Revert { .. } => assert!(error_msg.contains("reverted")),
            SimulationError::InvalidNonce { .. } => assert!(error_msg.contains("nonce")),
            SimulationError::InsufficientBalance { .. } => assert!(error_msg.contains("balance")),
            SimulationError::StateAccessError { .. } => assert!(error_msg.contains("State access")),
            SimulationError::Timeout => assert!(error_msg.contains("timed out")),
            _ => {}
        }
    }
}
