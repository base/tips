use crate::engine::{create_simulation_engine, SimulationEngine};
use crate::listener::{create_mempool_listener, MempoolEventListener};
use crate::publisher::{create_database_publisher, SimulationResultPublisher};
use crate::state::create_rpc_state_provider;
use crate::types::{SimulationConfig, SimulationRequest};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{error, info};
use uuid;

/// Main service that orchestrates all simulation components
pub struct SimulatorService {
    config: SimulationConfig,
    listener: Box<dyn MempoolEventListener>,
    engine: Box<dyn SimulationEngine>,
    publisher: Box<dyn SimulationResultPublisher>,
    simulation_queue: Option<mpsc::Receiver<SimulationRequest>>,
    listener_handle: Option<JoinHandle<Result<()>>>,
    simulation_handle: Option<JoinHandle<Result<()>>>,
}

impl SimulatorService {
    /// Create a new simulator service with the given configuration
    pub async fn new(config: SimulationConfig) -> Result<Self> {
        info!("Initializing simulator service");

        // Create state provider
        let state_provider = Arc::new(create_rpc_state_provider(&config.reth_http_url)?);
        info!(reth_url = %config.reth_http_url, "State provider initialized");

        // Create simulation engine
        let engine = Box::new(create_simulation_engine(
            state_provider,
            config.simulation_timeout_ms,
        ));
        info!(
            timeout_ms = config.simulation_timeout_ms,
            "Simulation engine initialized"
        );

        // Create mempool listener
        let listener = Box::new(create_mempool_listener(&config)?);
        info!(
            topic = %config.kafka_topic,
            brokers = ?config.kafka_brokers,
            "Mempool listener initialized"
        );

        // Create database connection and publisher
        let datastore = Arc::new(
            tips_datastore::PostgresDatastore::connect(config.database_url.clone()).await?
        );
        
        // Run database migrations
        datastore.run_migrations().await?;
        info!("Database migrations completed");

        let publisher = Box::new(create_database_publisher(datastore));
        info!("Result publisher initialized");

        Ok(Self {
            config,
            listener,
            engine,
            publisher,
            simulation_queue: None,
            listener_handle: None,
            simulation_handle: None,
        })
    }

    /// Start the simulator service
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting simulator service");

        // Create channel for simulation requests
        let (sender, receiver) = mpsc::channel::<SimulationRequest>(1000);
        self.simulation_queue = Some(receiver);

        // Start mempool listener
        let listener_handle = {
            let mut listener = std::mem::replace(
                &mut self.listener,
                Box::new(NoOpListener),
            );
            let sender_clone = sender.clone();
            
            tokio::spawn(async move {
                listener.start(sender_clone).await
            })
        };

        // Start simulation worker
        let simulation_handle = {
            let engine = std::mem::replace(
                &mut self.engine,
                Box::new(NoOpEngine),
            );
            let publisher = std::mem::replace(
                &mut self.publisher,
                Box::new(NoOpPublisher),
            );
            let mut queue = self.simulation_queue.take().unwrap();
            let max_concurrent = self.config.max_concurrent_simulations;

            tokio::spawn(async move {
                Self::simulation_worker(
                    &mut queue,
                    engine.as_ref(),
                    publisher.as_ref(),
                    max_concurrent,
                ).await
            })
        };

        self.listener_handle = Some(listener_handle);
        self.simulation_handle = Some(simulation_handle);

        info!(
            max_concurrent = self.config.max_concurrent_simulations,
            "Simulator service started successfully"
        );

        Ok(())
    }

    /// Stop the simulator service
    pub async fn stop(&mut self) -> Result<()> {
        info!("Stopping simulator service");

        // Stop listener
        if let Some(handle) = self.listener_handle.take() {
            handle.abort();
        }

        // Stop simulation worker
        if let Some(handle) = self.simulation_handle.take() {
            handle.abort();
        }

        info!("Simulator service stopped");
        Ok(())
    }

    /// Wait for the service to complete
    pub async fn wait(&mut self) -> Result<()> {
        if let Some(listener_handle) = &mut self.listener_handle {
            if let Err(e) = listener_handle.await {
                error!(error = %e, "Listener task failed");
            }
        }

        if let Some(simulation_handle) = &mut self.simulation_handle {
            if let Err(e) = simulation_handle.await {
                error!(error = %e, "Simulation worker task failed");
            }
        }

        Ok(())
    }

    /// Main simulation worker that processes simulation requests
    async fn simulation_worker(
        queue: &mut mpsc::Receiver<SimulationRequest>,
        engine: &dyn SimulationEngine,
        publisher: &dyn SimulationResultPublisher,
        max_concurrent: usize,
    ) -> Result<()> {
        info!(max_concurrent = max_concurrent, "Starting simulation worker");

        // Use a semaphore to limit concurrent simulations
        let semaphore = Arc::new(tokio::sync::Semaphore::new(max_concurrent));
        
        while let Some(request) = queue.recv().await {
            let semaphore_clone = semaphore.clone();
            let request_clone = request.clone();

            // Spawn a task for this simulation
            tokio::spawn(async move {
                let _permit = match semaphore_clone.acquire().await {
                    Ok(permit) => permit,
                    Err(_) => {
                        error!("Failed to acquire semaphore permit");
                        return;
                    }
                };

                info!(
                    bundle_id = %request_clone.bundle_id,
                    block_number = request_clone.block_number,
                    num_transactions = request_clone.bundle.txs.len(),
                    "Processing simulation request"
                );

                // Perform the simulation
                match engine.simulate_bundle(request_clone.clone()).await {
                    Ok(result) => {
                        info!(
                            bundle_id = %request_clone.bundle_id,
                            simulation_id = %result.id,
                            success = result.success,
                            gas_used = ?result.gas_used,
                            execution_time_us = result.execution_time_us,
                            "Simulation completed"
                        );

                        // Publish the result
                        if let Err(e) = publisher.publish_result(result).await {
                            error!(
                                error = %e,
                                bundle_id = %request_clone.bundle_id,
                                "Failed to publish simulation result"
                            );
                        }
                    }
                    Err(e) => {
                        error!(
                            error = %e,
                            bundle_id = %request_clone.bundle_id,
                            "Simulation failed with error"
                        );
                    }
                }
            });
        }

        info!("Simulation worker shutting down");
        Ok(())
    }
}

// Placeholder implementations for move semantics
struct NoOpListener;

#[async_trait::async_trait]
impl MempoolEventListener for NoOpListener {
    async fn start(&mut self, _sender: mpsc::Sender<SimulationRequest>) -> Result<()> {
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        Ok(())
    }
}

struct NoOpEngine;

#[async_trait::async_trait]
impl SimulationEngine for NoOpEngine {
    async fn simulate_bundle(&self, _request: SimulationRequest) -> Result<SimulationResult> {
        Err(anyhow::anyhow!("NoOpEngine should never be called"))
    }
}

struct NoOpPublisher;

#[async_trait::async_trait]
impl SimulationResultPublisher for NoOpPublisher {
    async fn publish_result(&self, _result: SimulationResult) -> Result<()> {
        Err(anyhow::anyhow!("NoOpPublisher should never be called"))
    }
    
    async fn get_results_for_bundle(&self, _bundle_id: uuid::Uuid) -> Result<Vec<SimulationResult>> {
        Err(anyhow::anyhow!("NoOpPublisher should never be called"))
    }
    
    async fn get_result_by_id(&self, _result_id: uuid::Uuid) -> Result<Option<SimulationResult>> {
        Err(anyhow::anyhow!("NoOpPublisher should never be called"))
    }
}

impl Clone for SimulationRequest {
    fn clone(&self) -> Self {
        Self {
            bundle_id: self.bundle_id,
            bundle: self.bundle.clone(),
            block_number: self.block_number,
            block_hash: self.block_hash,
        }
    }
}
