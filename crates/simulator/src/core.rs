use crate::engine::SimulationEngine;
use crate::publisher::SimulationPublisher;
use crate::types::SimulationRequest;
use async_trait::async_trait;
use eyre::Result;
use tracing::{error, info};

/// Clean trait for bundle simulation without exposing Reth's complex types
#[async_trait]
pub trait BundleSimulator: Send + Sync {
    /// Simulate a bundle execution
    async fn simulate(&self, request: &SimulationRequest) -> Result<()>;
}

/// Production bundle simulator for Reth
/// This is the Reth-specific implementation
pub struct RethBundleSimulator<E, P>
where
    E: SimulationEngine,
    P: SimulationPublisher,
{
    engine: E,
    publisher: P,
}

impl<E, P> RethBundleSimulator<E, P>
where
    E: SimulationEngine,
    P: SimulationPublisher,
{
    pub fn new(engine: E, publisher: P) -> Self {
        Self { 
            engine, 
            publisher,
        }
    }
}

#[async_trait]
impl<E, P> BundleSimulator for RethBundleSimulator<E, P>
where
    E: SimulationEngine + 'static,
    P: SimulationPublisher + 'static,
{
    async fn simulate(&self, request: &SimulationRequest) -> Result<()> {
        // Run the simulation - engine will get its own state provider
        match self.engine.simulate_bundle(request).await {
            Ok(result) => {
                info!(
                    bundle_id = %request.bundle_id,
                    simulation_id = %result.id,
                    success = result.success,
                    "Simulation completed"
                );

                if let Err(e) = self.publisher.publish_result(result).await {
                    error!(
                        error = %e,
                        bundle_id = %request.bundle_id,
                        "Failed to publish simulation result"
                    );
                }
            }
            Err(e) => {
                error!(
                    error = %e,
                    bundle_id = %request.bundle_id,
                    "Simulation failed"
                );
            }
        }

        Ok(())
    }
}
