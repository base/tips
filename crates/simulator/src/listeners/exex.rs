use crate::types::SimulationRequest;
use crate::worker_pool::{SimulationWorkerPool, SimulationTask};
use crate::engine::SimulationEngine;
use crate::publisher::SimulationPublisher;

use alloy_consensus::BlockHeader;
use alloy_primitives::B256;
use alloy_rpc_types::BlockNumHash;
use alloy_rpc_types_mev::EthSendBundle;
use eyre::Result;
use reth_exex::{ExExContext, ExExEvent, ExExNotification};
use reth_node_api::FullNodeComponents;
use futures_util::StreamExt;
use std::sync::Arc;
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

/// ExEx event listener that processes chain events and queues bundle simulations
/// Processes chain events (commits, reorgs, reverts) and queues simulation tasks
pub struct ExExEventListener<Node, E, P, D> 
where
    Node: FullNodeComponents,
    E: SimulationEngine + Clone + 'static,
    P: SimulationPublisher + Clone + 'static,
    D: tips_datastore::BundleDatastore,
{
    /// The execution extension context
    ctx: ExExContext<Node>,
    /// Datastore for fetching bundles from mempool
    datastore: Arc<D>,
    /// Shared simulation worker pool
    worker_pool: Arc<SimulationWorkerPool<E, P, Node::Provider>>,
}

impl<Node, E, P, D> ExExEventListener<Node, E, P, D>
where
    Node: FullNodeComponents,
    E: SimulationEngine + Clone + 'static,
    P: SimulationPublisher + Clone + 'static,
    D: tips_datastore::BundleDatastore + 'static,
{
    /// Create a new ExEx event listener
    pub fn new(
        ctx: ExExContext<Node>,
        datastore: Arc<D>,
        worker_pool: Arc<SimulationWorkerPool<E, P, Node::Provider>>,
    ) -> Self {
        Self {
            ctx,
            datastore,
            worker_pool,
        }
    }

    /// Main execution loop for the ExEx event listener
    pub async fn run(mut self) -> Result<()> {
        info!("Starting ExEx event listener");

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

        info!("ExEx event listener shutting down");
        Ok(())
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

        // Update latest block for cancellation
        self.worker_pool.update_latest_block(block_number);

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
            
            // Create simulation task
            let task = SimulationTask {
                request,
            };
            
            // Send to worker queue
            if let Err(e) = self.worker_pool.queue_simulation(task).await {
                error!(
                    error = %e,
                    bundle_index = index,
                    "Failed to queue simulation task"
                );
                break;
            }
        }

        Ok(())
    }
}
