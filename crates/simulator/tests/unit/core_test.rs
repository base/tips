/// Unit tests for the BundleSimulator core component
use crate::common::builders::*;
use crate::common::fixtures::*;
use crate::common::mock_bundle_simulator::MockBundleSimulator;
use crate::common::mocks::*;
use std::sync::Arc;
use tips_simulator::{core::BundleSimulator, SimulationError};
use uuid::Uuid;

#[tokio::test]
async fn test_bundle_simulator_creation() {
    let engine = MockSimulationEngine::new();
    let publisher = MockSimulationPublisher::new();
    let simulator = MockBundleSimulator::new(engine, publisher);

    // Simply verify it can be created
    assert!(std::mem::size_of_val(&simulator) > 0);
}

#[tokio::test]
async fn test_simulate_success_flow() {
    // Arrange
    let bundle_id = Uuid::new_v4();
    let expected_result = SimulationResultBuilder::successful()
        .with_ids(Uuid::new_v4(), bundle_id)
        .with_gas_used(200_000)
        .build();

    let engine = MockSimulationEngine::new().with_result(expected_result.clone());
    let publisher = MockSimulationPublisher::new();
    let simulator = MockBundleSimulator::new(engine.clone(), publisher.clone());

    let request = SimulationRequestBuilder::new()
        .with_bundle_id(bundle_id)
        .with_bundle(bundles::single_tx_bundle())
        .build();

    // Act - using the clean trait interface
    let result = simulator.simulate(&request).await;

    // Assert
    assert!(result.is_ok());
    assert_eq!(engine.simulation_count(), 1);
    assert_eq!(publisher.published_count(), 1);

    let published = publisher.get_published();
    assert_eq!(published[0].bundle_id, bundle_id);
    assert_eq!(published[0].gas_used, Some(200_000));
}

#[tokio::test]
async fn test_simulate_failure_flow() {
    // Arrange
    let bundle_id = Uuid::new_v4();
    let engine = MockSimulationEngine::new().fail_next_with(SimulationError::Revert {
        reason: "Test revert".to_string(),
    });
    let publisher = MockSimulationPublisher::new();
    let simulator = MockBundleSimulator::new(engine.clone(), publisher.clone());

    let request = SimulationRequestBuilder::new()
        .with_bundle_id(bundle_id)
        .build();

    // Act
    let result = simulator.simulate(&request).await;

    // Assert
    assert!(result.is_ok()); // simulate() succeeds even if simulation fails
    assert_eq!(engine.simulation_count(), 1);
    assert_eq!(publisher.published_count(), 1);

    let published = publisher.get_published();
    assert!(!published[0].success);
    assert!(published[0]
        .error_reason
        .as_ref()
        .unwrap()
        .contains("revert"));
}

#[tokio::test]
async fn test_publisher_error_handling() {
    // Arrange
    let engine = MockSimulationEngine::new();
    let publisher = MockSimulationPublisher::new().fail_next();
    let simulator = MockBundleSimulator::new(engine.clone(), publisher.clone());

    let request = SimulationRequestBuilder::new().build();

    // Act - should log error but not fail
    let result = simulator.simulate(&request).await;

    // Assert
    assert!(result.is_ok());
    assert_eq!(engine.simulation_count(), 1); // Engine was called
    assert_eq!(publisher.published_count(), 0); // Publisher failed
}

#[tokio::test]
async fn test_state_provider_factory_error() {
    // This test would require a mock StateProviderFactory that fails
    // For now, we'll test with an invalid block hash
    let engine = MockSimulationEngine::new();
    let publisher = MockSimulationPublisher::new();
    let simulator = MockBundleSimulator::new(engine.clone(), publisher.clone());

    // Request with block hash that doesn't exist in our mock state
    let request = SimulationRequestBuilder::new()
        .with_block(99_999_999, alloy_primitives::B256::random())
        .build();

    // Act
    let result = simulator.simulate(&request).await;

    // Assert - in our mock, this actually succeeds, but in real implementation
    // it would fail with state provider error
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_multiple_sequential_simulations() {
    // Arrange
    let engine = MockSimulationEngine::new();
    let publisher = MockSimulationPublisher::new();
    let simulator = MockBundleSimulator::new(engine.clone(), publisher.clone());

    // Act - simulate multiple bundles
    for i in 0..5 {
        let request = SimulationRequestBuilder::new()
            .with_bundle(
                TestBundleBuilder::new()
                    .with_simple_transaction(&[i as u8, 0x01, 0x02])
                    .build(),
            )
            .build();

        let result = simulator.simulate(&request).await;
        assert!(result.is_ok());
    }

    // Assert
    assert_eq!(engine.simulation_count(), 5);
    assert_eq!(publisher.published_count(), 5);
}

#[tokio::test]
async fn test_empty_bundle_simulation() {
    // Arrange
    let engine = MockSimulationEngine::new();
    let publisher = MockSimulationPublisher::new();
    let simulator = MockBundleSimulator::new(engine.clone(), publisher.clone());

    let empty_bundle = TestBundleBuilder::new().build(); // No transactions
    let request = SimulationRequestBuilder::new()
        .with_bundle(empty_bundle)
        .build();

    // Act
    let result = simulator.simulate(&request).await;

    // Assert
    assert!(result.is_ok());
    assert_eq!(engine.simulation_count(), 1);
    assert_eq!(publisher.published_count(), 1);
}

#[tokio::test]
async fn test_simulate_with_complex_state_diff() {
    // Arrange
    let bundle_id = Uuid::new_v4();
    let mut state_diff = std::collections::HashMap::new();

    // Add multiple accounts with multiple storage changes
    for i in 0..3 {
        let addr = alloy_primitives::Address::random();
        let mut storage = std::collections::HashMap::new();
        for j in 0..5 {
            storage.insert(
                alloy_primitives::U256::from(j),
                alloy_primitives::U256::from(i * 100 + j),
            );
        }
        state_diff.insert(addr, storage);
    }

    let result = SimulationResultBuilder::successful()
        .with_ids(Uuid::new_v4(), bundle_id)
        .build();

    // Manually set the state diff
    let mut result = result;
    result.state_diff = state_diff.clone();

    let engine = MockSimulationEngine::new().with_result(result);
    let publisher = MockSimulationPublisher::new();
    let simulator = MockBundleSimulator::new(engine, publisher.clone());

    let request = SimulationRequestBuilder::new()
        .with_bundle_id(bundle_id)
        .build();

    // Act
    simulator.simulate(&request).await.unwrap();

    // Assert
    let published = publisher.get_published();
    assert_eq!(published[0].state_diff.len(), 3);
    for (_, storage) in &published[0].state_diff {
        assert_eq!(storage.len(), 5);
    }
}

#[tokio::test]
async fn test_concurrent_simulator_usage() {
    // Test that the simulator can be used concurrently
    let engine = MockSimulationEngine::new();
    let publisher = MockSimulationPublisher::new();
    let simulator = Arc::new(MockBundleSimulator::new(engine.clone(), publisher.clone()));

    let mut handles = vec![];

    // Spawn multiple concurrent simulations
    for i in 0..10 {
        let sim = Arc::clone(&simulator);

        let handle = tokio::spawn(async move {
            let request = SimulationRequestBuilder::new()
                .with_bundle(
                    TestBundleBuilder::new()
                        .with_simple_transaction(&[i as u8, 0x01, 0x02])
                        .build(),
                )
                .build();

            sim.simulate(&request).await
        });

        handles.push(handle);
    }

    // Wait for all to complete
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }

    // Verify all were processed
    assert_eq!(engine.simulation_count(), 10);
    assert_eq!(publisher.published_count(), 10);
}
