use crate::core::BundleSimulator;
use crate::types::SimulationRequest;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tracing::{debug, info, warn};

/// Simulation task
pub struct SimulationTask {
    pub request: SimulationRequest,
}

/// Generic simulation worker pool that can be shared across different simulators
pub struct SimulationWorkerPool<B>
where
    B: BundleSimulator,
{
    /// Core bundle simulator
    simulator: Arc<B>,
    /// Channel for sending simulation requests to workers
    simulation_tx: mpsc::Sender<SimulationTask>,
    /// Channel for receiving simulation requests in workers
    simulation_rx: Arc<tokio::sync::Mutex<mpsc::Receiver<SimulationTask>>>,
    /// Latest block number being processed (for cancellation)
    latest_block: AtomicU64,
    /// Worker task handles (wrapped in Mutex for interior mutability)
    worker_handles: tokio::sync::Mutex<JoinSet<()>>,
    /// Maximum number of concurrent simulations
    max_concurrent: usize,
}

impl<B> SimulationWorkerPool<B>
where
    B: BundleSimulator + 'static,
{
    /// Create a new simulation worker pool
    pub fn new(simulator: Arc<B>, max_concurrent_simulations: usize) -> Arc<Self> {
        let (simulation_tx, simulation_rx) = mpsc::channel(1000);

        Arc::new(Self {
            simulator,
            simulation_tx,
            simulation_rx: Arc::new(tokio::sync::Mutex::new(simulation_rx)),
            latest_block: AtomicU64::new(0),
            worker_handles: tokio::sync::Mutex::new(JoinSet::new()),
            max_concurrent: max_concurrent_simulations,
        })
    }

    /// Start simulation worker tasks
    /// Returns true if workers were started, false if already running
    pub async fn start(self: &Arc<Self>) -> bool {
        let mut handles = self.worker_handles.lock().await;

        if !handles.is_empty() {
            debug!("Simulation workers already started");
            return false;
        }
        info!(
            num_workers = self.max_concurrent,
            "Starting simulation workers"
        );

        for worker_id in 0..self.max_concurrent {
            let pool = Arc::clone(self);

            handles.spawn(async move { Self::simulation_worker(worker_id, pool).await });
        }
        true
    }

    /// Queue a simulation task
    pub async fn queue_simulation(
        &self,
        task: SimulationTask,
    ) -> Result<(), mpsc::error::SendError<SimulationTask>> {
        self.simulation_tx.send(task).await
    }

    /// Update the latest block number being processed
    pub fn update_latest_block(&self, block_number: u64) {
        self.latest_block.store(block_number, Ordering::Release);
        debug!(block_number, "Updated latest block for cancellation");
    }

    /// Wait for all workers to complete
    pub async fn shutdown(self) {
        // Close the channel to signal workers to stop
        drop(self.simulation_tx);

        // Wait for workers to complete
        let mut handles = self.worker_handles.lock().await;
        while let Some(result) = handles.join_next().await {
            if let Err(e) = result {
                tracing::error!(error = %e, "Worker task failed");
            }
        }
    }

    /// Worker task that processes simulation requests
    async fn simulation_worker(worker_id: usize, pool: Arc<Self>) {
        debug!(worker_id, "Simulation worker started");

        loop {
            // Get the next simulation task
            let task = {
                // FIXME: This lock looks like it prevents multiple workers from running in parallel.
                let mut rx = pool.simulation_rx.lock().await;
                rx.recv().await
            };

            let Some(task) = task else {
                debug!(worker_id, "Simulation channel closed, worker shutting down");
                break;
            };

            // Check if this simulation is for an old block
            let current_latest = pool.latest_block.load(Ordering::Acquire);
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
            match pool.simulator.simulate(&task.request).await {
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
