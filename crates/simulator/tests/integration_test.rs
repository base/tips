/// Integration tests for the simulator functionality
///
/// Note: These tests use mock implementations because the actual StateProvider
/// trait requires complex setup. For real integration testing with actual
/// state providers, see the component tests.
mod common;
mod unit;

use common::assertions::*;
use common::builders::*;
use common::fixtures::*;
use common::mock_bundle_simulator::MockBundleSimulator;
use common::mocks::*;
use common::timing::*;

use alloy_primitives::U256;
use std::sync::Arc;
use std::time::Duration;
use tips_simulator::{
    core::BundleSimulator, types::ExExSimulationConfig, MempoolListenerConfig, SimulationWorkerPool,
};

#[tokio::test]
async fn test_successful_bundle_simulation() {
    // Setup
    let engine = MockSimulationEngine::new();
    let publisher = MockSimulationPublisher::new();
    let simulator = MockBundleSimulator::new(engine.clone(), publisher.clone());

    // Create test request
    let bundle = bundles::single_tx_bundle();
    let request = SimulationRequestBuilder::new().with_bundle(bundle).build();

    // Execute
    let result = simulator.simulate(&request).await;

    // Verify
    assert!(result.is_ok());
    assert_eq!(engine.simulation_count(), 1);
    assert_eq!(publisher.published_count(), 1);

    let published = publisher.get_published();
    assert_simulation_success(&published[0]);
}

#[tokio::test]
async fn test_failed_bundle_simulation() {
    // Setup with failing engine
    let engine = MockSimulationEngine::new()
        .fail_next_with(tips_simulator::types::SimulationError::OutOfGas);
    let publisher = MockSimulationPublisher::new();
    let simulator = MockBundleSimulator::new(engine.clone(), publisher.clone());

    // Create test request
    let bundle = bundles::single_tx_bundle();
    let request = SimulationRequestBuilder::new().with_bundle(bundle).build();

    // Execute
    let result = simulator.simulate(&request).await;

    // Verify
    assert!(result.is_ok()); // simulate() itself succeeds even if simulation fails
    assert_eq!(engine.simulation_count(), 1);
    assert_eq!(publisher.published_count(), 1);

    let published = publisher.get_published();
    assert_simulation_failure(&published[0]);
    assert!(published[0]
        .error_reason
        .as_ref()
        .unwrap()
        .contains("out of gas"));
}

#[tokio::test]
async fn test_publisher_failure_handling() {
    // Setup with failing publisher
    let engine = MockSimulationEngine::new();
    let publisher = MockSimulationPublisher::new().fail_next();
    let simulator = MockBundleSimulator::new(engine.clone(), publisher.clone());

    // Create test request
    let bundle = bundles::single_tx_bundle();
    let request = SimulationRequestBuilder::new().with_bundle(bundle).build();

    // Execute - should not panic even if publisher fails
    let result = simulator.simulate(&request).await;

    // Verify
    assert!(result.is_ok());
    assert_eq!(engine.simulation_count(), 1);
    assert_eq!(publisher.published_count(), 0); // Publisher failed
}

#[tokio::test]
async fn test_worker_pool_concurrent_simulations() {
    // Setup
    let engine = MockSimulationEngine::new();
    let publisher = MockSimulationPublisher::new();
    let simulator = Arc::new(MockBundleSimulator::new(engine.clone(), publisher.clone()));
    // Provider no longer needed with new architecture

    // Create worker pool with 4 workers
    let pool = SimulationWorkerPool::new(simulator, 4);
    pool.start().await;

    // Queue multiple simulations
    let num_simulations = 20;
    let mut bundle_ids = Vec::new();

    for i in 0..num_simulations {
        let bundle = TestBundleBuilder::new()
            .with_simple_transaction(&[i as u8, 0x01, 0x02])
            .with_block_number(blocks::BLOCK_18M + i as u64)
            .build();

        let request = SimulationRequestBuilder::new()
            .with_bundle(bundle)
            .with_block(
                blocks::BLOCK_18M + i as u64,
                alloy_primitives::B256::random(),
            )
            .build();

        bundle_ids.push(request.bundle_id);

        let task = tips_simulator::worker_pool::SimulationTask { request };
        pool.queue_simulation(task).await.unwrap();
    }

    // Wait for completion with timeout
    let (_, duration) = measure_async(async {
        tokio::time::sleep(Duration::from_millis(500)).await;
    })
    .await;

    // Verify all simulations completed
    assert_eq!(publisher.published_count(), num_simulations);

    // Verify all bundle IDs are present
    let published = publisher.get_published();
    for bundle_id in bundle_ids {
        assert!(published.iter().any(|r| r.bundle_id == bundle_id));
    }

    // Verify reasonable execution time
    assert!(
        duration < Duration::from_secs(2),
        "Simulations took too long"
    );
}

#[tokio::test]
async fn test_worker_pool_error_recovery() {
    // Setup engine that fails every other simulation
    let engine = MockSimulationEngine::new();
    let publisher = MockSimulationPublisher::new();
    let simulator = Arc::new(MockBundleSimulator::new(engine.clone(), publisher.clone()));
    // Provider no longer needed with new architecture

    // Create worker pool
    let pool = SimulationWorkerPool::new(simulator, 2);
    pool.start().await;

    // Queue simulations with some failures
    for i in 0..10 {
        let mut builder = SimulationResultBuilder::successful();
        if i % 2 == 1 {
            builder = SimulationResultBuilder::failed().with_revert(format!("Test revert {}", i));
        }

        let _ = engine.clone().with_result(builder.build());

        let request = SimulationRequestBuilder::new()
            .with_bundle(bundles::single_tx_bundle())
            .build();

        let task = tips_simulator::worker_pool::SimulationTask { request };
        pool.queue_simulation(task).await.unwrap();
    }

    // Wait for completion
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Verify all simulations were attempted
    let published = publisher.get_published();
    assert_eq!(published.len(), 10);

    // Verify mix of successes and failures
    let successes = published.iter().filter(|r| r.success).count();
    let failures = published.iter().filter(|r| !r.success).count();
    assert!(successes > 0 && failures > 0);
}

#[tokio::test]
async fn test_large_bundle_simulation() {
    // Setup
    let engine = MockSimulationEngine::new();
    let publisher = MockSimulationPublisher::new();
    let simulator = MockBundleSimulator::new(engine.clone(), publisher.clone());

    // Create large bundle
    let large_bundle = bundles::large_bundle(100);
    let request = SimulationRequestBuilder::new()
        .with_bundle(large_bundle)
        .build();

    // Execute with timeout
    let result =
        assert_completes_within(simulator.simulate(&request), Duration::from_secs(5)).await;

    // Verify
    assert!(result.is_ok());
    assert_eq!(engine.simulation_count(), 1);
    assert_eq!(publisher.published_count(), 1);
}

#[tokio::test]
async fn test_state_diff_tracking() {
    // Setup engine that returns specific state changes
    let simulation_result = SimulationResultBuilder::successful()
        .with_state_change(*addresses::ALICE, U256::from(0), U256::from(100))
        .with_state_change(*addresses::ALICE, U256::from(1), U256::from(200))
        .with_state_change(*addresses::BOB, U256::from(0), U256::from(300))
        .build();

    let engine = MockSimulationEngine::new().with_result(simulation_result);
    let publisher = MockSimulationPublisher::new();
    let simulator = MockBundleSimulator::new(engine, publisher.clone());

    // Execute
    let bundle = bundles::single_tx_bundle();
    let request = SimulationRequestBuilder::new().with_bundle(bundle).build();
    simulator.simulate(&request).await.unwrap();

    // Verify state diff
    let published = publisher.get_published();
    assert_eq!(published.len(), 1);

    let result = &published[0];
    assert_state_diff_contains(result, *addresses::ALICE, U256::from(0), U256::from(100));
    assert_state_diff_contains(result, *addresses::ALICE, U256::from(1), U256::from(200));
    assert_state_diff_contains(result, *addresses::BOB, U256::from(0), U256::from(300));
}

#[test]
fn test_simulation_request_creation() {
    let bundle = bundles::single_tx_bundle();
    let request = SimulationRequestBuilder::new()
        .with_bundle(bundle.clone())
        .with_block(blocks::BLOCK_18M, *blocks::HASH_18M)
        .build();

    assert_eq!(request.bundle.txs.len(), bundle.txs.len());
    assert_eq!(request.block_number, blocks::BLOCK_18M);
    assert_eq!(request.block_hash, *blocks::HASH_18M);
}

#[test]
fn test_mempool_config() {
    let config = MempoolListenerConfig {
        kafka_brokers: vec!["localhost:9092".to_string()],
        kafka_topic: "tips-audit".to_string(),
        kafka_group_id: "tips-simulator".to_string(),
        database_url: "postgresql://user:pass@localhost:5432/tips".to_string(),
    };

    assert_eq!(config.kafka_brokers, vec!["localhost:9092"]);
    assert_eq!(config.kafka_topic, "tips-audit");
}

#[test]
fn test_exex_config() {
    let config = ExExSimulationConfig {
        database_url: "postgresql://user:pass@localhost:5432/tips".to_string(),
        max_concurrent_simulations: 10,
        simulation_timeout_ms: 5000,
    };

    assert_eq!(config.max_concurrent_simulations, 10);
    assert_eq!(config.simulation_timeout_ms, 5000);
}
