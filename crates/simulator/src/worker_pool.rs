use crate::core::BundleSimulator;
use crate::types::SimulationRequest;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use reth_provider::StateProviderFactory;
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tracing::{debug, info, warn};

/// Simulation task
pub struct SimulationTask {
    pub request: SimulationRequest,
}

/// Generic simulation worker pool that can be shared across different simulators
pub struct SimulationWorkerPool<E, P, S>
where
    E: crate::engine::SimulationEngine,
    P: crate::publisher::SimulationResultPublisher,
    S: StateProviderFactory,
{
    /// Core bundle simulator
    simulator: Arc<BundleSimulator<E, P>>,
    /// State provider factory
    state_provider_factory: Arc<S>,
    /// Channel for sending simulation requests to workers
    simulation_tx: mpsc::Sender<SimulationTask>,
    /// Channel for receiving simulation requests in workers
    simulation_rx: Arc<tokio::sync::Mutex<mpsc::Receiver<SimulationTask>>>,
    /// Latest block number being processed (for cancellation)
    latest_block: Arc<AtomicU64>,
    /// Worker task handles
    worker_handles: JoinSet<()>,
    /// Maximum number of concurrent simulations
    max_concurrent: usize,
}

impl<E, P, S> SimulationWorkerPool<E, P, S>
where
    E: crate::engine::SimulationEngine + Clone + 'static,
    P: crate::publisher::SimulationResultPublisher + Clone + 'static,
    S: reth_provider::StateProviderFactory + Send + Sync + 'static,
{
    /// Create a new simulation worker pool
    pub fn new(
        simulator: Arc<BundleSimulator<E, P>>,
        state_provider_factory: Arc<S>,
        max_concurrent_simulations: usize,
    ) -> Self {
        let (simulation_tx, simulation_rx) = mpsc::channel(1000);
        
        Self {
            simulator,
            state_provider_factory,
            simulation_tx,
            simulation_rx: Arc::new(tokio::sync::Mutex::new(simulation_rx)),
            latest_block: Arc::new(AtomicU64::new(0)),
            worker_handles: JoinSet::new(),
            max_concurrent: max_concurrent_simulations,
        }
    }

    /// Start simulation worker tasks
    pub fn start(&mut self) {
        info!(num_workers = self.max_concurrent, "Starting simulation workers");
        
        for worker_id in 0..self.max_concurrent {
            let simulator = self.simulator.clone();
            let state_provider_factory = self.state_provider_factory.clone();
            let simulation_rx = self.simulation_rx.clone();
            let latest_block = self.latest_block.clone();
            
            self.worker_handles.spawn(async move {
                Self::simulation_worker(
                    worker_id,
                    simulator,
                    state_provider_factory,
                    simulation_rx,
                    latest_block,
                ).await
            });
        }
    }

    /// Queue a simulation task
    pub async fn queue_simulation(&self, task: SimulationTask) -> Result<(), mpsc::error::SendError<SimulationTask>> {
        self.simulation_tx.send(task).await
    }
    
    /// Update the latest block number being processed
    pub fn update_latest_block(&self, block_number: u64) {
        self.latest_block.store(block_number, Ordering::Release);
        debug!(block_number, "Updated latest block for cancellation");
    }


    /// Wait for all workers to complete
    pub async fn shutdown(mut self) {
        // Close the channel to signal workers to stop
        drop(self.simulation_tx);
        
        // Wait for workers to complete
        while let Some(result) = self.worker_handles.join_next().await {
            if let Err(e) = result {
                tracing::error!(error = %e, "Worker task failed");
            }
        }
    }

    /// Worker task that processes simulation requests
    async fn simulation_worker(
        worker_id: usize,
        simulator: Arc<BundleSimulator<E, P>>,
        state_provider_factory: Arc<S>,
        simulation_rx: Arc<tokio::sync::Mutex<mpsc::Receiver<SimulationTask>>>,
        latest_block: Arc<AtomicU64>,
    ) 
    where
        S: reth_provider::StateProviderFactory,
    {
        debug!(worker_id, "Simulation worker started");
        
        loop {
            // Get the next simulation task
            let task = {
                let mut rx = simulation_rx.lock().await;
                rx.recv().await
            };
            
            let Some(task) = task else {
                debug!(worker_id, "Simulation channel closed, worker shutting down");
                break;
            };
            
            // Check if this simulation is for an old block
            let current_latest = latest_block.load(Ordering::Acquire);
            if task.request.block_number < current_latest {
                warn!(
                    worker_id,
                    bundle_id = %task.request.bundle_id,
                    block_number = task.request.block_number,
                    latest_block = current_latest,
                    "Skipping simulation for outdated block"
                );
                continue;
            }
            
            // Execute the simulation
            match simulator.simulate(task.request.clone(), state_provider_factory.as_ref()).await {
                Ok(_) => {
                    debug!(
                        worker_id,
                        bundle_id = %task.request.bundle_id,
                        "Simulation completed successfully"
                    );
                }
                Err(e) => {
                    tracing::error!(
                        worker_id,
                        bundle_id = %task.request.bundle_id,
                        error = %e,
                        "Simulation failed"
                    );
                }
            }
        }
        
        debug!(worker_id, "Simulation worker stopped");
    }
}
