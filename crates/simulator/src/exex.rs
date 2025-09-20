use crate::core::BundleSimulator;
use crate::types::SimulationRequest;

use alloy_consensus::BlockHeader;
use alloy_primitives::B256;
use alloy_rpc_types::BlockNumHash;
use alloy_rpc_types_mev::EthSendBundle;
use eyre::Result;
use reth_exex::{ExExContext, ExExEvent, ExExNotification};
use reth_node_api::FullNodeComponents;
use futures_util::StreamExt;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::task::JoinSet;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Datastore-based mempool bundle provider
pub struct DatastoreBundleProvider<D> 
where
    D: tips_datastore::BundleDatastore,
{
    datastore: Arc<D>,
}

impl<D> DatastoreBundleProvider<D> 
where
    D: tips_datastore::BundleDatastore,
{
    pub fn new(datastore: Arc<D>) -> Self {
        Self { datastore }
    }
    
    /// Get all bundles valid for a specific block
    pub async fn get_bundles_for_block(&self, block_number: u64) -> Result<Vec<(Uuid, EthSendBundle)>> {
        use tips_datastore::postgres::BundleFilter;
        
        // Create filter for bundles valid at this block
        let filter = BundleFilter::new()
            .valid_for_block(block_number);
        
        // Fetch bundles from datastore
        let bundles_with_metadata = self.datastore.select_bundles(filter).await
            .map_err(|e| eyre::eyre!("Failed to select bundles: {}", e))?;
        
        // Convert to (Uuid, EthSendBundle) pairs
        // TODO: The bundle ID should be returned from the datastore query
        // For now, we generate new IDs for each bundle
        let result = bundles_with_metadata
            .into_iter()
            .map(|bwm| (Uuid::new_v4(), bwm.bundle.clone()))
            .collect();
        
        Ok(result)
    }
}

/// Simulation task with cancellation token
struct SimulationTask {
    request: SimulationRequest,
    block_number: u64,
    cancel_tx: mpsc::Sender<()>,
}

/// ExEx event simulator that simulates bundles from committed blocks
/// Processes chain events (commits, reorgs, reverts) and simulates potential bundles
pub struct ExExEventSimulator<Node, E, P, D> 
where
    Node: FullNodeComponents,
    E: crate::engine::SimulationEngine,
    P: crate::publisher::SimulationResultPublisher,
    D: tips_datastore::BundleDatastore,
{
    /// The execution extension context
    ctx: ExExContext<Node>,
    /// Core bundle simulator for shared simulation logic
    core_simulator: Arc<BundleSimulator<E, P>>,
    /// State provider factory for creating state providers
    state_provider_factory: Arc<Node::Provider>,
    /// Datastore for fetching bundles from mempool
    datastore: Arc<D>,
    /// Channel for sending simulation requests to workers
    simulation_tx: mpsc::Sender<SimulationTask>,
    /// Channel for receiving simulation requests in workers
    simulation_rx: Arc<tokio::sync::Mutex<mpsc::Receiver<SimulationTask>>>,
    /// Map of block number to cancellation channels for pending simulations
    pending_simulations: Arc<RwLock<HashMap<u64, Vec<mpsc::Sender<()>>>>>,
    /// Worker task handles
    worker_handles: JoinSet<()>,
    /// Maximum number of concurrent simulations
    max_concurrent: usize,
}

impl<Node, E, P, D> ExExEventSimulator<Node, E, P, D>
where
    Node: FullNodeComponents,
    E: crate::engine::SimulationEngine + Clone + 'static,
    P: crate::publisher::SimulationResultPublisher + Clone + 'static,
    D: tips_datastore::BundleDatastore + 'static,
{
    /// Create a new ExEx event simulator
    pub fn new(
        ctx: ExExContext<Node>,
        core_simulator: BundleSimulator<E, P>,
        state_provider_factory: Arc<Node::Provider>,
        datastore: Arc<D>,
        max_concurrent_simulations: usize,
    ) -> Self {
        let (simulation_tx, simulation_rx) = mpsc::channel(1000);
        
        Self {
            ctx,
            core_simulator: Arc::new(core_simulator),
            state_provider_factory,
            datastore,
            simulation_tx,
            simulation_rx: Arc::new(tokio::sync::Mutex::new(simulation_rx)),
            pending_simulations: Arc::new(RwLock::new(HashMap::new())),
            worker_handles: JoinSet::new(),
            max_concurrent: max_concurrent_simulations,
        }
    }

    /// Main execution loop for the ExEx event simulator
    pub async fn run(mut self) -> Result<()> {
        info!("Starting ExEx event simulator");

        // Initialize simulation workers
        self.start_simulation_workers();

        loop {
            match self.ctx.notifications.next().await {
                Some(Ok(notification)) => {
                    if let Err(e) = self.handle_notification(notification).await {
                        error!(error = %e, "Failed to handle ExEx notification");
                    }
                }
                Some(Err(e)) => {
                    error!(error = %e, "Failed to receive ExEx notification");
                    break;
                }
                None => {
                    info!("ExEx notification channel closed, shutting down");
                    break;
                }
            }
        }

        info!("ExEx event simulator shutting down");
        
        // Cancel all pending simulations
        self.cancel_all_simulations().await;
        
        // Wait for workers to complete
        while let Some(result) = self.worker_handles.join_next().await {
            if let Err(e) = result {
                error!(error = %e, "Worker task failed");
            }
        }
        
        Ok(())
    }
    
    /// Start simulation worker tasks
    fn start_simulation_workers(&mut self) {
        info!(num_workers = self.max_concurrent, "Starting simulation workers");
        
        for worker_id in 0..self.max_concurrent {
            let core_simulator = self.core_simulator.clone();
            let state_provider_factory = self.state_provider_factory.clone();
            let simulation_rx = self.simulation_rx.clone();
            let pending_simulations = self.pending_simulations.clone();
            
            self.worker_handles.spawn(async move {
                Self::simulation_worker(
                    worker_id,
                    core_simulator,
                    state_provider_factory,
                    simulation_rx,
                    pending_simulations,
                ).await
            });
        }
    }
    
    /// Worker task that processes simulation requests
    async fn simulation_worker(
        worker_id: usize,
        core_simulator: Arc<BundleSimulator<E, P>>,
        state_provider_factory: Arc<Node::Provider>,
        simulation_rx: Arc<tokio::sync::Mutex<mpsc::Receiver<SimulationTask>>>,
        pending_simulations: Arc<RwLock<HashMap<u64, Vec<mpsc::Sender<()>>>>>,
    ) {
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
            
            // Create a cancellation receiver
            let (cancel_tx, mut cancel_rx) = mpsc::channel(1);
            
            // Check if simulation should be cancelled
            tokio::select! {
                _ = cancel_rx.recv() => {
                    debug!(
                        worker_id,
                        bundle_id = %task.request.bundle_id,
                        block_number = task.block_number,
                        "Simulation cancelled before starting"
                    );
                    continue;
                }
                result = core_simulator.simulate(task.request.clone(), &state_provider_factory) => {
                    match result {
                        Ok(_) => {
                            debug!(
                                worker_id,
                                bundle_id = %task.request.bundle_id,
                                "Simulation completed successfully"
                            );
                        }
                        Err(e) => {
                            error!(
                                worker_id,
                                bundle_id = %task.request.bundle_id,
                                error = %e,
                                "Simulation failed"
                            );
                        }
                    }
                }
            }
            
            // Remove cancellation channel from pending simulations
            let mut pending = pending_simulations.write().await;
            if let Some(channels) = pending.get_mut(&task.block_number) {
                channels.retain(|tx| !tx.same_channel(&cancel_tx));
                if channels.is_empty() {
                    pending.remove(&task.block_number);
                }
            }
        }
        
        debug!(worker_id, "Simulation worker stopped");
    }

    /// Handle ExEx notifications
    async fn handle_notification(&mut self, notification: ExExNotification<<<Node as reth_node_api::FullNodeTypes>::Types as reth_node_api::NodeTypes>::Primitives>) -> Result<()> {
        match notification {
            ExExNotification::ChainCommitted { new } => {
                info!(
                    block_range = ?new.range(),
                    num_blocks = new.blocks().len(),
                    "Processing committed blocks"
                );
                
                // Process each block in the committed chain
                for (_block_num, block) in new.blocks() {
                    let block_hash = block.hash();
                    self.process_block((&block_hash, block)).await?;
                }

                // Notify that we've processed this notification
                self.ctx
                    .events
                    .send(ExExEvent::FinishedHeight(BlockNumHash::new(new.tip().number(), new.tip().hash())))?;
            }
            ExExNotification::ChainReorged { old: _, new } => {
                warn!(
                    block_range = ?new.range(),
                    "Chain reorg detected, processing new chain"
                );
                
                // Process the new canonical chain
                for (_block_num, block) in new.blocks() {
                    let block_hash = block.hash();
                    self.process_block((&block_hash, block)).await?;
                }

                self.ctx
                    .events
                    .send(ExExEvent::FinishedHeight(BlockNumHash::new(new.tip().number(), new.tip().hash())))?;
            }
            ExExNotification::ChainReverted { old } => {
                warn!(
                    block_range = ?old.range(),
                    "Chain reverted, no simulation needed"
                );

                self.ctx
                    .events
                    .send(ExExEvent::FinishedHeight(BlockNumHash::new(old.tip().number(), old.tip().hash())))?;
            }
        }

        Ok(())
    }

    /// Process a single block for potential bundle simulations
    async fn process_block<B>(&mut self, block: (&B256, &reth_primitives::RecoveredBlock<B>)) -> Result<()> 
    where
        B: reth_node_api::Block,
    {
        let (block_hash, sealed_block) = block;
        let block_number = sealed_block.number();
        
        debug!(
            block_number = block_number,
            block_hash = ?block_hash,
            "Processing block for bundle simulation"
        );

        // Cancel simulations for older blocks
        self.cancel_simulations_before_block(block_number).await;

        // Fetch all bundles valid for this block from datastore
        use tips_datastore::postgres::BundleFilter;
        let filter = BundleFilter::new()
            .valid_for_block(block_number);
        
        let bundles_with_metadata = match self.datastore.select_bundles(filter).await {
            Ok(bundles) => bundles,
            Err(e) => {
                error!(
                    error = %e,
                    block_number,
                    "Failed to fetch bundles from datastore"
                );
                return Ok(());
            }
        };
        
        info!(
            block_number,
            num_bundles = bundles_with_metadata.len(),
            "Queuing bundle simulations for new block"
        );

        // Create a list to track cancellation channels for this block
        let mut cancellation_channels = Vec::new();

        // Queue simulations for each bundle
        for (index, bundle_metadata) in bundles_with_metadata.into_iter().enumerate() {
            // TODO: The bundle ID should be returned from the datastore query
            // For now, we generate new IDs for each bundle
            let bundle_id = Uuid::new_v4();
            
            // Create simulation request
            let request = SimulationRequest {
                bundle_id,
                bundle: bundle_metadata.bundle,
                block_number,
                block_hash: *block_hash,
            };
            
            // Create cancellation channel
            let (cancel_tx, _cancel_rx) = mpsc::channel(1);
            cancellation_channels.push(cancel_tx.clone());
            
            // Create simulation task
            let task = SimulationTask {
                request,
                block_number,
                cancel_tx,
            };
            
            // Send to worker queue
            if let Err(e) = self.simulation_tx.send(task).await {
                error!(
                    error = %e,
                    bundle_index = index,
                    "Failed to queue simulation task"
                );
                break;
            }
        }
        
        // Store cancellation channels for this block
        if !cancellation_channels.is_empty() {
            let mut pending = self.pending_simulations.write().await;
            pending.insert(block_number, cancellation_channels);
        }

        Ok(())
    }
    
    /// Cancel all simulations for blocks before the given block number
    async fn cancel_simulations_before_block(&self, block_number: u64) {
        let mut pending = self.pending_simulations.write().await;
        
        // Find all blocks to cancel
        let blocks_to_cancel: Vec<u64> = pending.keys()
            .filter(|&&block| block < block_number)
            .copied()
            .collect();
        
        if blocks_to_cancel.is_empty() {
            return;
        }
        
        info!(
            current_block = block_number,
            num_blocks = blocks_to_cancel.len(),
            "Cancelling simulations for older blocks"
        );
        
        // Cancel simulations for each old block
        for old_block in blocks_to_cancel {
            if let Some(channels) = pending.remove(&old_block) {
                debug!(
                    old_block,
                    num_simulations = channels.len(),
                    "Cancelling simulations for block"
                );
                
                // Send cancellation signal to all tasks for this block
                for cancel_tx in channels {
                    let _ = cancel_tx.send(()).await;
                }
            }
        }
    }
    
    /// Cancel all pending simulations
    async fn cancel_all_simulations(&self) {
        let mut pending = self.pending_simulations.write().await;
        
        info!(
            num_blocks = pending.len(),
            "Cancelling all pending simulations"
        );
        
        // Cancel all simulations
        for (block_number, channels) in pending.drain() {
            debug!(
                block_number,
                num_simulations = channels.len(),
                "Cancelling simulations for block"
            );
            
            for cancel_tx in channels {
                let _ = cancel_tx.send(()).await;
            }
        }
    }
}
