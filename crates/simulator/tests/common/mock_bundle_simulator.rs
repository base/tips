use crate::common::mocks::{MockSimulationEngine, MockSimulationPublisher};
use async_trait::async_trait;
use eyre::Result;
/// Mock implementation of BundleSimulator for testing
use tips_simulator::core::BundleSimulator;
use tips_simulator::engine::SimulationEngine;
use tips_simulator::publisher::SimulationPublisher;
use tips_simulator::types::SimulationRequest;

/// Mock bundle simulator for testing - no Reth dependencies!
pub struct MockBundleSimulator {
    engine: MockSimulationEngine,
    publisher: MockSimulationPublisher,
}

impl MockBundleSimulator {
    pub fn new(engine: MockSimulationEngine, publisher: MockSimulationPublisher) -> Self {
        Self { engine, publisher }
    }


}

#[async_trait]
impl BundleSimulator for MockBundleSimulator {
    async fn simulate(&self, request: &SimulationRequest) -> Result<()> {
        // Run the simulation using the mock engine - no state provider needed!
        match self.engine.simulate_bundle(request).await {
            Ok(result) => {
                tracing::info!(
                    bundle_id = %request.bundle_id,
                    simulation_id = %result.id,
                    success = result.success,
                    "Simulation completed"
                );

                if let Err(e) = self.publisher.publish_result(result).await {
                    tracing::error!(
                        error = %e,
                        bundle_id = %request.bundle_id,
                        "Failed to publish simulation result"
                    );
                }
            }
            Err(e) => {
                tracing::error!(
                    error = %e,
                    bundle_id = %request.bundle_id,
                    "Simulation failed"
                );
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common;

    #[tokio::test]
    async fn test_mock_bundle_simulator() {
        let engine = MockSimulationEngine::new();
        let publisher = MockSimulationPublisher::new();
        let simulator = MockBundleSimulator::new(engine.clone(), publisher.clone());

        let bundle = common::create_test_bundle(1, 18_000_000);
        let request = common::create_test_request(bundle);

        // Use the clean trait interface
        let result = simulator.simulate(&request).await;

        assert!(result.is_ok());
        assert_eq!(engine.simulation_count(), 1);
        assert_eq!(publisher.published_count(), 1);
    }
}
