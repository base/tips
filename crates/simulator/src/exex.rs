use crate::engine::SimulationEngine;
use crate::publisher::SimulationResultPublisher;
use crate::types::{SimulationError, SimulationRequest, SimulationResult};

use alloy_primitives::{B256, U256};
use alloy_rpc_types_mev::EthSendBundle;
use anyhow::Result;
use reth_exex::{ExExContext, ExExEvent, ExExNotification};
use reth_node_api::{FullNodeComponents, NodeAddOns};
use reth_primitives::{BlockNumber, TransactionSignedEcRecovered};
use reth_provider::{CanonicalInMemoryState, Chain, StateProviderFactory};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// ExEx that simulates bundles when new blocks are committed
pub struct SimulatorExEx<Node: FullNodeComponents, AddOns: NodeAddOns<Node>> {
    /// The execution extension context
    ctx: ExExContext<Node, AddOns>,
    /// Simulation engine for processing bundles
    engine: Box<dyn SimulationEngine>,
    /// Publisher for simulation results
    publisher: Box<dyn SimulationResultPublisher>,
    /// Channel for receiving simulation requests
    simulation_rx: mpsc::UnboundedReceiver<SimulationRequest>,
    /// Sender for simulation requests
    simulation_tx: mpsc::UnboundedSender<SimulationRequest>,
    /// Maximum number of concurrent simulations
    max_concurrent: usize,
}

impl<Node, AddOns> SimulatorExEx<Node, AddOns>
where
    Node: FullNodeComponents,
    AddOns: NodeAddOns<Node>,
{
    /// Create a new simulator ExEx
    pub fn new(
        ctx: ExExContext<Node, AddOns>,
        engine: Box<dyn SimulationEngine>,
        publisher: Box<dyn SimulationResultPublisher>,
        max_concurrent: usize,
    ) -> Self {
        let (simulation_tx, simulation_rx) = mpsc::unbounded_channel();
        
        Self {
            ctx,
            engine,
            publisher,
            simulation_rx,
            simulation_tx,
            max_concurrent,
        }
    }

    /// Main execution loop for the ExEx
    pub async fn run(mut self) -> Result<()> {
        info!("Starting Tips Simulator ExEx");

        // Spawn the simulation worker
        let simulation_handle = {
            let engine = std::mem::replace(&mut self.engine, Box::new(NoOpEngine));
            let publisher = std::mem::replace(&mut self.publisher, Box::new(NoOpPublisher));
            let mut rx = std::mem::replace(&mut self.simulation_rx, mpsc::unbounded_channel().1);
            let max_concurrent = self.max_concurrent;
            
            tokio::spawn(async move {
                Self::simulation_worker(&mut rx, engine.as_ref(), publisher.as_ref(), max_concurrent).await
            })
        };

        loop {
            tokio::select! {
                notification = self.ctx.notifications.recv() => {
                    match notification {
                        Some(notification) => {
                            if let Err(e) = self.handle_notification(notification).await {
                                error!(error = %e, "Failed to handle ExEx notification");
                            }
                        }
                        None => {
                            info!("ExEx notification channel closed, shutting down");
                            break;
                        }
                    }
                }
                result = &mut simulation_handle => {
                    match result {
                        Ok(_) => info!("Simulation worker completed"),
                        Err(e) => error!(error = %e, "Simulation worker failed"),
                    }
                    break;
                }
            }
        }

        // Clean shutdown
        simulation_handle.abort();
        info!("Tips Simulator ExEx shutting down");
        Ok(())
    }

    /// Handle ExEx notifications
    async fn handle_notification(&mut self, notification: ExExNotification) -> Result<()> {
        match notification {
            ExExNotification::ChainCommitted { new } => {
                info!(
                    block_range = ?new.range(),
                    num_blocks = new.blocks().len(),
                    "Processing committed blocks"
                );
                
                // Process each block in the committed chain
                for block in new.blocks() {
                    self.process_block(block).await?;
                }

                // Notify that we've processed this notification
                self.ctx
                    .events
                    .send(ExExEvent::FinishedHeight(new.tip().number))?;
            }
            ExExNotification::ChainReorged { old: _, new } => {
                warn!(
                    block_range = ?new.range(),
                    "Chain reorg detected, processing new chain"
                );
                
                // Process the new canonical chain
                for block in new.blocks() {
                    self.process_block(block).await?;
                }

                self.ctx
                    .events
                    .send(ExExEvent::FinishedHeight(new.tip().number))?;
            }
            ExExNotification::ChainReverted { old } => {
                warn!(
                    block_range = ?old.range(),
                    "Chain reverted, no simulation needed"
                );

                self.ctx
                    .events
                    .send(ExExEvent::FinishedHeight(old.tip().number))?;
            }
        }

        Ok(())
    }

    /// Process a single block for potential bundle simulations
    async fn process_block(&mut self, execution_outcome: &reth_execution_types::ExecutionOutcome) -> Result<()> {
        debug!(
            block_number = execution_outcome.block_number(),
            "Processing block for bundle simulation"
        );

        // TODO: Extract potential bundles from the block's transactions
        // For now, this is a placeholder that would need to implement logic to:
        // 1. Group transactions that could be bundles
        // 2. Identify MEV opportunities
        // 3. Create simulation requests for those bundles

        // This would be where we analyze transactions in the block
        // and create simulation requests for potential bundles
        let _block_number = execution_outcome.block_number();
        let _block_hash = execution_outcome.block_hash();

        // Placeholder: Create a mock bundle simulation request
        // In a real implementation, this would extract actual bundles from transactions
        self.create_mock_simulation_request().await?;

        Ok(())
    }

    /// Create a mock simulation request (placeholder)
    async fn create_mock_simulation_request(&self) -> Result<()> {
        // This is a placeholder for bundle extraction logic
        let bundle_id = Uuid::new_v4();
        let mock_bundle = EthSendBundle {
            txs: vec![], // Would contain actual transaction data
            block_number: None,
            min_timestamp: None,
            max_timestamp: None,
            reverting_tx_hashes: vec![],
            replacement_uuid: None,
        };

        let request = SimulationRequest {
            bundle_id,
            bundle: mock_bundle,
            block_number: 0, // Would be actual block number
            block_hash: B256::ZERO, // Would be actual block hash
        };

        if let Err(e) = self.simulation_tx.send(request) {
            warn!(error = %e, "Failed to queue simulation request");
        }

        Ok(())
    }

    /// Simulation worker that processes simulation requests
    async fn simulation_worker(
        queue: &mut mpsc::UnboundedReceiver<SimulationRequest>,
        engine: &dyn SimulationEngine,
        publisher: &dyn SimulationResultPublisher,
        max_concurrent: usize,
    ) -> Result<()> {
        info!(max_concurrent = max_concurrent, "Starting ExEx simulation worker");

        let semaphore = Arc::new(tokio::sync::Semaphore::new(max_concurrent));

        while let Some(request) = queue.recv().await {
            let semaphore_clone = semaphore.clone();
            let request_clone = request.clone();

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
                    "Processing ExEx simulation request"
                );

                match engine.simulate_bundle(request_clone.clone()).await {
                    Ok(result) => {
                        info!(
                            bundle_id = %request_clone.bundle_id,
                            simulation_id = %result.id,
                            success = result.success,
                            "ExEx simulation completed"
                        );

                        if let Err(e) = publisher.publish_result(result).await {
                            error!(
                                error = %e,
                                bundle_id = %request_clone.bundle_id,
                                "Failed to publish ExEx simulation result"
                            );
                        }
                    }
                    Err(e) => {
                        error!(
                            error = %e,
                            bundle_id = %request_clone.bundle_id,
                            "ExEx simulation failed"
                        );
                    }
                }
            });
        }

        info!("ExEx simulation worker shutting down");
        Ok(())
    }
}

/// No-op engine for move semantics
struct NoOpEngine;

#[async_trait::async_trait]
impl SimulationEngine for NoOpEngine {
    async fn simulate_bundle(&self, _request: SimulationRequest) -> Result<SimulationResult> {
        Err(anyhow::anyhow!("NoOpEngine should never be called"))
    }
}

/// No-op publisher for move semantics  
struct NoOpPublisher;

#[async_trait::async_trait]
impl SimulationResultPublisher for NoOpPublisher {
    async fn publish_result(&self, _result: SimulationResult) -> Result<()> {
        Err(anyhow::anyhow!("NoOpPublisher should never be called"))
    }
    
    async fn get_results_for_bundle(&self, _bundle_id: Uuid) -> Result<Vec<SimulationResult>> {
        Err(anyhow::anyhow!("NoOpPublisher should never be called"))
    }
    
    async fn get_result_by_id(&self, _result_id: Uuid) -> Result<Option<SimulationResult>> {
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
