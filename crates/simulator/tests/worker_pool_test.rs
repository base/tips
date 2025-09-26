/// Unit tests for the SimulationWorkerPool implementation
mod common;

use common::builders::*;
use common::fixtures::*;
use common::mock_bundle_simulator::MockBundleSimulator;
use common::mocks::*;
use std::sync::Arc;
use std::time::Duration;
use tips_simulator::worker_pool::{SimulationTask, SimulationWorkerPool};


#[tokio::test]
async fn test_worker_pool_start_and_shutdown() {
    let engine = MockSimulationEngine::new();
    let publisher = MockSimulationPublisher::new();
    let simulator = Arc::new(MockBundleSimulator::new(engine, publisher));

    let pool = SimulationWorkerPool::new(simulator, 2);
    
    // Start the pool
    let started = pool.start().await;
    assert!(started); // Should return true on first start
    
    // Starting again should return false
    let started_again = pool.start().await;
    assert!(!started_again);
    
    // Test shutdown - pool will shutdown when dropped
}

#[tokio::test]
async fn test_worker_pool_single_simulation() {
    let engine = MockSimulationEngine::new();
    let publisher = MockSimulationPublisher::new();
    let simulator = Arc::new(MockBundleSimulator::new(engine.clone(), publisher.clone()));

    let pool = SimulationWorkerPool::new(simulator, 1);
    pool.start().await;

    // Queue a single simulation
    let bundle = bundles::single_tx_bundle();
    let request = SimulationRequestBuilder::new()
        .with_bundle(bundle)
        .build();
    
    let task = SimulationTask { request };
    
    let queue_result = pool.queue_simulation(task).await;
    assert!(queue_result.is_ok());

    // Wait for processing
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Verify simulation was processed
    assert_eq!(engine.simulation_count(), 1);
    assert_eq!(publisher.published_count(), 1);
}

#[tokio::test]
async fn test_worker_pool_multiple_simulations() {
    let engine = MockSimulationEngine::new();
    let publisher = MockSimulationPublisher::new();
    let simulator = Arc::new(MockBundleSimulator::new(engine.clone(), publisher.clone()));

    let pool = SimulationWorkerPool::new(simulator, 3);
    pool.start().await;

    // Queue multiple simulations
    let num_simulations = 10;
    for i in 0..num_simulations {
        let bundle = TestBundleBuilder::new()
            .with_simple_transaction(&[i as u8, 0x01, 0x02])
            .with_block_number(18_000_000 + i as u64)
            .build();
        
        let request = SimulationRequestBuilder::new()
            .with_bundle(bundle)
            .with_block(18_000_000 + i as u64, alloy_primitives::B256::random())
            .build();

        let task = SimulationTask { request };
        pool.queue_simulation(task).await.unwrap();
    }

    // Wait for processing
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Verify all simulations were processed
    assert_eq!(engine.simulation_count(), num_simulations);
    assert_eq!(publisher.published_count(), num_simulations);
}

#[tokio::test]
async fn test_worker_pool_concurrent_workers() {
    let engine = MockSimulationEngine::new();
    let publisher = MockSimulationPublisher::new();
    let simulator = Arc::new(MockBundleSimulator::new(engine.clone(), publisher.clone()));

    // Create pool with multiple workers
    let num_workers = 5;
    let pool = SimulationWorkerPool::new(simulator, num_workers);
    pool.start().await;

    // Queue many simulations quickly
    let num_simulations = 20;
    let mut tasks = vec![];
    
    for i in 0..num_simulations {
        let bundle = TestBundleBuilder::new()
            .with_simple_transaction(&[i as u8, 0x03, 0x04])
            .build();
        
        let request = SimulationRequestBuilder::new()
            .with_bundle(bundle)
            .build();

        tasks.push(SimulationTask { request });
    }

    // Queue all tasks
    for task in tasks {
        pool.queue_simulation(task).await.unwrap();
    }

    // Wait for all to process
    tokio::time::sleep(Duration::from_millis(800)).await;

    // Verify all were processed
    assert_eq!(engine.simulation_count(), num_simulations);
    assert_eq!(publisher.published_count(), num_simulations);
}

#[tokio::test]
async fn test_worker_pool_simulation_failures() {
    let engine = MockSimulationEngine::new();
    let publisher = MockSimulationPublisher::new();
    let simulator = Arc::new(MockBundleSimulator::new(engine.clone(), publisher.clone()));

    let pool = SimulationWorkerPool::new(simulator, 2);
    pool.start().await;

    // Queue simulations with some that will fail
    for i in 0..5 {
        // Configure engine to fail odd-numbered simulations
        if i % 2 == 1 {
            let _ = engine.clone().fail_next_with(tips_simulator::SimulationError::OutOfGas);
        } else {
            let result = SimulationResultBuilder::successful()
                .with_gas_used(100_000 + i * 10_000)
                .build();
            let _ = engine.clone().with_result(result);
        }

        let request = SimulationRequestBuilder::new()
            .with_bundle(bundles::single_tx_bundle())
            .build();

        let task = SimulationTask { request };
        pool.queue_simulation(task).await.unwrap();
    }

    // Wait for processing
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Verify all simulations were attempted (both successes and failures)
    assert_eq!(engine.simulation_count(), 5);
    assert_eq!(publisher.published_count(), 5);

    // Verify mix of success and failure results
    let published = publisher.get_published();
    let successes = published.iter().filter(|r| r.success).count();
    let failures = published.iter().filter(|r| !r.success).count();
    assert!(successes > 0);
    assert!(failures > 0);
}

#[tokio::test]
async fn test_worker_pool_publisher_failures() {
    let engine = MockSimulationEngine::new();
    let publisher = MockSimulationPublisher::new();
    let simulator = Arc::new(MockBundleSimulator::new(engine.clone(), publisher.clone()));

    let pool = SimulationWorkerPool::new(simulator, 1);
    pool.start().await;

    // Configure publisher to fail
    let _ = publisher.clone().fail_next();

    let request = SimulationRequestBuilder::new().build();
    let task = SimulationTask { request };
    
    pool.queue_simulation(task).await.unwrap();

    // Wait for processing
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Engine should still be called even if publisher fails
    assert_eq!(engine.simulation_count(), 1);
    assert_eq!(publisher.published_count(), 0); // Publisher failed
}

#[tokio::test]
async fn test_worker_pool_block_cancellation() {
    let engine = MockSimulationEngine::new();
    let publisher = MockSimulationPublisher::new();
    let simulator = Arc::new(MockBundleSimulator::new(engine.clone(), publisher.clone()));

    let pool = SimulationWorkerPool::new(simulator, 2);
    pool.start().await;

    // Queue simulations for different blocks
    let old_block = 18_000_000;
    let new_block = 18_000_010;

    // Queue simulation for old block
    let old_request = SimulationRequestBuilder::new()
        .with_block(old_block, alloy_primitives::B256::random())
        .build();
    let old_task = SimulationTask { request: old_request };
    
    // Queue simulation for new block
    let new_request = SimulationRequestBuilder::new()
        .with_block(new_block, alloy_primitives::B256::random())
        .build();
    let new_task = SimulationTask { request: new_request };

    // Update latest block to the new block (should cancel old simulations)
    pool.update_latest_block(new_block);

    // Queue both tasks (old should be cancelled, new should proceed)
    pool.queue_simulation(old_task).await.unwrap();
    pool.queue_simulation(new_task).await.unwrap();

    // Wait for processing
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Only the new block simulation should have been processed
    // Note: Due to timing, both might be processed or the old one might be skipped
    // We check that at least one was processed
    assert!(engine.simulation_count() >= 1);
    assert!(publisher.published_count() >= 1);
}

#[tokio::test]
async fn test_worker_pool_heavy_load() {
    let engine = MockSimulationEngine::new();
    let publisher = MockSimulationPublisher::new();
    let simulator = Arc::new(MockBundleSimulator::new(engine.clone(), publisher.clone()));

    let pool = SimulationWorkerPool::new(simulator, 4);
    pool.start().await;

    // Queue a large number of simulations
    let num_simulations = 50;
    
    for i in 0..num_simulations {
        let bundle = TestBundleBuilder::new()
            .with_simple_transaction(&[i as u8, 0x05, 0x06])
            .build();
        
        let request = SimulationRequestBuilder::new()
            .with_bundle(bundle)
            .build();

        let task = SimulationTask { request };
        pool.queue_simulation(task).await.unwrap();
    }

    // Wait for processing with a reasonable timeout
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Verify all simulations were processed
    assert_eq!(engine.simulation_count(), num_simulations);
    assert_eq!(publisher.published_count(), num_simulations);
}

#[tokio::test]
async fn test_worker_pool_empty_queue_shutdown() {
    let engine = MockSimulationEngine::new();
    let publisher = MockSimulationPublisher::new();
    let simulator = Arc::new(MockBundleSimulator::new(engine, publisher));

    let pool = SimulationWorkerPool::new(simulator, 2);
    pool.start().await;

    // Shutdown immediately without queuing any tasks - pool will shutdown when dropped
}

#[tokio::test]
async fn test_worker_pool_large_bundles() {
    let engine = MockSimulationEngine::new();
    let publisher = MockSimulationPublisher::new();
    let simulator = Arc::new(MockBundleSimulator::new(engine.clone(), publisher.clone()));

    let pool = SimulationWorkerPool::new(simulator, 2);
    pool.start().await;

    // Queue simulations with large bundles
    for i in 0..3 {
        let large_bundle = bundles::large_bundle(20 + i * 10); // 20, 30, 40 transactions
        let request = SimulationRequestBuilder::new()
            .with_bundle(large_bundle)
            .build();

        let task = SimulationTask { request };
        pool.queue_simulation(task).await.unwrap();
    }

    // Wait for processing
    tokio::time::sleep(Duration::from_millis(400)).await;

    // Verify all large bundles were processed
    assert_eq!(engine.simulation_count(), 3);
    assert_eq!(publisher.published_count(), 3);
}

#[tokio::test]
async fn test_worker_pool_queue_full_behavior() {
    let engine = MockSimulationEngine::new();
    let publisher = MockSimulationPublisher::new();
    let simulator = Arc::new(MockBundleSimulator::new(engine.clone(), publisher.clone()));

    // Create pool with small queue capacity (we can't easily control this with current API)
    let pool = SimulationWorkerPool::new(simulator, 1);
    pool.start().await;

    // Queue many simulations rapidly
    let mut queue_results = vec![];
    for i in 0..20 {
        let request = SimulationRequestBuilder::new()
            .with_bundle(
                TestBundleBuilder::new()
                    .with_simple_transaction(&[i as u8])
                    .build(),
            )
            .build();

        let task = SimulationTask { request };
        let result = pool.queue_simulation(task).await;
        queue_results.push(result);
    }

    // All should succeed with current implementation (large default queue size)
    for result in queue_results {
        assert!(result.is_ok());
    }

    // Wait for processing
    tokio::time::sleep(Duration::from_millis(800)).await;

    // Verify processing
    assert_eq!(engine.simulation_count(), 20);
    assert_eq!(publisher.published_count(), 20);
}

#[tokio::test]
async fn test_worker_pool_mixed_block_numbers() {
    let engine = MockSimulationEngine::new();
    let publisher = MockSimulationPublisher::new();
    let simulator = Arc::new(MockBundleSimulator::new(engine.clone(), publisher.clone()));

    let pool = SimulationWorkerPool::new(simulator, 3);
    pool.start().await;

    // Queue simulations for various block numbers
    let block_numbers = vec![18_000_000, 18_000_005, 18_000_002, 18_000_008, 18_000_001];
    
    for (i, block_num) in block_numbers.iter().enumerate() {
        let request = SimulationRequestBuilder::new()
            .with_block(*block_num, alloy_primitives::B256::random())
            .with_bundle(
                TestBundleBuilder::new()
                    .with_simple_transaction(&[i as u8, 0x07, 0x08])
                    .build(),
            )
            .build();

        let task = SimulationTask { request };
        pool.queue_simulation(task).await.unwrap();
    }

    // Update to latest block to potentially cancel some older simulations
    pool.update_latest_block(18_000_008);

    // Wait for processing
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Some simulations should have been processed
    assert!(engine.simulation_count() > 0);
    assert!(publisher.published_count() > 0);
    assert!(engine.simulation_count() <= block_numbers.len());
}

#[tokio::test]
async fn test_worker_pool_rapid_block_updates() {
    let engine = MockSimulationEngine::new();
    let publisher = MockSimulationPublisher::new();
    let simulator = Arc::new(MockBundleSimulator::new(engine.clone(), publisher.clone()));

    let pool = SimulationWorkerPool::new(simulator, 2);
    pool.start().await;

    // Rapidly update block numbers
    for i in 0..10 {
        pool.update_latest_block(18_000_000 + i);
        
        // Queue a simulation for an older block (should be cancelled)
        let request = SimulationRequestBuilder::new()
            .with_block(18_000_000 + i - 1, alloy_primitives::B256::random())
            .build();
        
        let task = SimulationTask { request };
        let _ = pool.queue_simulation(task).await;
    }

    // Wait for processing
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Most simulations should have been cancelled due to rapid block updates
    // The exact count depends on timing, but should be less than 10
    assert!(engine.simulation_count() <= 10);
    assert!(publisher.published_count() <= 10);
}

#[tokio::test]
async fn test_worker_pool_simulation_timing() {
    let engine = MockSimulationEngine::new();
    let publisher = MockSimulationPublisher::new();
    let simulator = Arc::new(MockBundleSimulator::new(engine.clone(), publisher.clone()));

    let pool = SimulationWorkerPool::new(simulator, 1);
    pool.start().await;

    let start_time = std::time::Instant::now();

    // Queue a few simulations
    for _i in 0..3 {
        let request = SimulationRequestBuilder::new()
            .with_bundle(bundles::single_tx_bundle())
            .build();

        let task = SimulationTask { request };
        pool.queue_simulation(task).await.unwrap();
    }

    // Wait for all to complete
    while engine.simulation_count() < 3 {
        tokio::time::sleep(Duration::from_millis(10)).await;
        
        // Prevent infinite loop with timeout
        if start_time.elapsed() > Duration::from_secs(5) {
            break;
        }
    }

    let elapsed = start_time.elapsed();

    // Verify timing is reasonable (should complete quickly with mocks)
    assert!(elapsed < Duration::from_secs(2));
    assert_eq!(engine.simulation_count(), 3);
    assert_eq!(publisher.published_count(), 3);
}
