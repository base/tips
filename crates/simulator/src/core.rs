use crate::engine::SimulationEngine;
use crate::publisher::SimulationResultPublisher;
use crate::types::SimulationRequest;
use eyre::Result;
use reth_provider::{StateProvider, StateProviderFactory};
use tracing::{error, info};

/// Core bundle simulator that provides shared simulation logic
/// Used by both mempool event simulators and ExEx event simulators
pub struct BundleSimulator<E, P> 
where
    E: SimulationEngine,
    P: SimulationResultPublisher,
{
    engine: E,
    publisher: P,
}

impl<E, P> BundleSimulator<E, P> 
where
    E: SimulationEngine,
    P: SimulationResultPublisher,
{
    pub fn new(engine: E, publisher: P) -> Self {
        Self {
            engine,
            publisher,
        }
    }
    
    /// Process a simulation request by creating state provider from factory
    /// Convenience method that handles state provider creation
    pub async fn simulate<F>(
        &self,
        request: SimulationRequest,
        state_provider_factory: &F,
    ) -> Result<()>
    where
        F: StateProviderFactory,
    {
        // Get state provider for the block
        // FIXME: We probably want to get the state provider once per block rather than once per
        // bundle for each block.
        let state_provider = state_provider_factory
            .state_by_block_hash(request.block_hash)
            .map_err(|e| eyre::eyre!("Failed to get state provider: {}", e))?;
        
        // Run the simulation
        match self.engine.simulate_bundle(request.clone(), &state_provider).await {
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
