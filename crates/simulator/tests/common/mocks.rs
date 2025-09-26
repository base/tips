/// Reusable mock implementations for testing
use alloy_primitives::{Address, B256, U256};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tips_simulator::types::SimulationRequest;
use tips_simulator::{SimulationEngine, SimulationError, SimulationPublisher, SimulationResult};
use uuid::Uuid;

/// Mock simulation engine with configurable behavior
#[derive(Clone)]
pub struct MockSimulationEngine {
    /// Results to return for each simulation
    results: Arc<Mutex<Vec<SimulationResult>>>,
    /// Track all simulations for verification
    simulations: Arc<Mutex<Vec<SimulationRequest>>>,
    /// Whether to fail the next simulation
    fail_next: Arc<Mutex<bool>>,
    /// Custom error to return on failure
    error: Arc<Mutex<Option<SimulationError>>>,
}

impl MockSimulationEngine {
    pub fn new() -> Self {
        Self {
            results: Arc::new(Mutex::new(Vec::new())),
            simulations: Arc::new(Mutex::new(Vec::new())),
            fail_next: Arc::new(Mutex::new(false)),
            error: Arc::new(Mutex::new(None)),
        }
    }

    pub fn with_result(self, result: SimulationResult) -> Self {
        self.results.lock().unwrap().push(result);
        self
    }

    pub fn fail_next_with(self, error: SimulationError) -> Self {
        *self.fail_next.lock().unwrap() = true;
        *self.error.lock().unwrap() = Some(error);
        self
    }

    pub fn get_simulations(&self) -> Vec<SimulationRequest> {
        self.simulations.lock().unwrap().clone()
    }

    pub fn simulation_count(&self) -> usize {
        self.simulations.lock().unwrap().len()
    }
}

#[async_trait]
impl SimulationEngine for MockSimulationEngine {
    async fn simulate_bundle(&self, request: &SimulationRequest) -> eyre::Result<SimulationResult> {
        // Track the simulation
        self.simulations.lock().unwrap().push(request.clone());

        // Check if we should fail
        if *self.fail_next.lock().unwrap() {
            *self.fail_next.lock().unwrap() = false;
            let error = self
                .error
                .lock()
                .unwrap()
                .take()
                .unwrap_or(SimulationError::Unknown {
                    message: "Mock failure".to_string(),
                });

            return Ok(SimulationResult::failure(
                Uuid::new_v4(),
                request.bundle_id,
                request.block_number,
                request.block_hash,
                1000,
                error,
            ));
        }

        // Return pre-configured result or create a default success
        let mut results = self.results.lock().unwrap();
        if let Some(result) = results.pop() {
            Ok(result)
        } else {
            let mut state_diff = HashMap::new();
            let address = Address::random();
            let mut storage = HashMap::new();
            storage.insert(U256::from(1), U256::from(100));
            state_diff.insert(address, storage);

            Ok(SimulationResult::success(
                Uuid::new_v4(),
                request.bundle_id,
                request.block_number,
                request.block_hash,
                150_000,
                1500,
                state_diff,
            ))
        }
    }
}

/// Mock simulation publisher that records published results
#[derive(Clone)]
pub struct MockSimulationPublisher {
    published: Arc<Mutex<Vec<SimulationResult>>>,
    fail_next: Arc<Mutex<bool>>,
}

impl MockSimulationPublisher {
    pub fn new() -> Self {
        Self {
            published: Arc::new(Mutex::new(Vec::new())),
            fail_next: Arc::new(Mutex::new(false)),
        }
    }

    pub fn fail_next(self) -> Self {
        *self.fail_next.lock().unwrap() = true;
        self
    }

    pub fn get_published(&self) -> Vec<SimulationResult> {
        self.published.lock().unwrap().clone()
    }

    pub fn published_count(&self) -> usize {
        self.published.lock().unwrap().len()
    }

    pub fn clear_published(&self) {
        self.published.lock().unwrap().clear();
    }
}

#[async_trait]
impl SimulationPublisher for MockSimulationPublisher {
    async fn publish_result(&self, result: SimulationResult) -> eyre::Result<()> {
        if *self.fail_next.lock().unwrap() {
            *self.fail_next.lock().unwrap() = false;
            return Err(eyre::eyre!("Mock publisher failure"));
        }

        self.published.lock().unwrap().push(result);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common;

    #[tokio::test]
    async fn test_mock_simulation_engine() {
        let engine = MockSimulationEngine::new();
        let _request = common::create_test_request(common::create_test_bundle(1, 18_000_000));

        // Verify the engine is initialized correctly
        assert_eq!(engine.simulation_count(), 0);
    }

    #[tokio::test]
    async fn test_mock_publisher() {
        let publisher = MockSimulationPublisher::new();
        let result = common::create_success_result(Uuid::new_v4(), 100_000);

        publisher.publish_result(result.clone()).await.unwrap();
        assert_eq!(publisher.published_count(), 1);

        let published = publisher.get_published();
        assert_eq!(published[0].id, result.id);
    }
}
