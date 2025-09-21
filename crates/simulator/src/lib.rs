pub mod config;
pub mod core;
pub mod engine;
pub mod listeners;
pub mod publisher;
pub mod types;
pub mod worker_pool;

use eyre::Result;
use reth_exex::ExExContext;
use reth_node_api::FullNodeComponents;
use std::sync::Arc;
use tracing::{info, error};
use crate::worker_pool::SimulationWorkerPool;

pub use config::{SimulatorExExConfig, SimulatorNodeConfig};
pub use core::BundleSimulator;
pub use engine::{create_simulation_engine, SimulationEngine, RethSimulationEngine};
pub use listeners::{ExExEventListener, MempoolEventListener, MempoolListenerConfig};
pub use publisher::{create_database_publisher, SimulationResultPublisher, DatabaseResultPublisher};
pub use types::{SimulationResult, SimulationError, ExExSimulationConfig};

// Type aliases for concrete implementations
pub type TipsBundleSimulator = BundleSimulator<RethSimulationEngine, DatabaseResultPublisher>;
pub type TipsExExEventListener<Node> = ExExEventListener<Node, RethSimulationEngine, DatabaseResultPublisher, tips_datastore::PostgresDatastore>;
pub type TipsMempoolEventListener<Node> = MempoolEventListener<Node, RethSimulationEngine, DatabaseResultPublisher>;

// Initialization functions

/// Common initialization components shared across listeners
struct CommonListenerComponents {
    datastore: Arc<tips_datastore::PostgresDatastore>,
    simulator: BundleSimulator<RethSimulationEngine, DatabaseResultPublisher>,
}

/// Initialize common listener components (database, publisher, engine, core simulator)
async fn init_common_components(database_url: String, simulation_timeout_ms: u64) -> Result<CommonListenerComponents> {
    let datastore = Arc::new(
        tips_datastore::PostgresDatastore::connect(database_url).await
            .map_err(|e| eyre::eyre!("Failed to connect to database: {}", e))?
    );

    let publisher = create_database_publisher(datastore.clone());
    info!("Database publisher initialized");

    let engine = create_simulation_engine(simulation_timeout_ms);
    info!(
        timeout_ms = simulation_timeout_ms,
        "Simulation engine initialized"
    );

    let simulator = BundleSimulator::new(engine, publisher);
    info!("Core bundle simulator initialized");

    Ok(CommonListenerComponents {
        datastore,
        simulator,
    })
}

/// Initialize ExEx event listener (ExEx) that processes committed blocks
/// 
/// Note: The worker pool is created but NOT started.
pub async fn init_exex_event_listener<Node>(
    ctx: ExExContext<Node>,
    config: ExExSimulationConfig,
) -> Result<TipsExExEventListener<Node>>
where
    Node: FullNodeComponents,
{
    info!("Initializing ExEx event listener");

    let common_components = init_common_components(config.database_url.clone(), config.simulation_timeout_ms).await?;

    let state_provider_factory = Arc::new(ctx.components.provider().clone());

    let worker_pool = crate::worker_pool::SimulationWorkerPool::new(
        Arc::new(common_components.simulator),
        state_provider_factory,
        config.max_concurrent_simulations,
    );

    let consensus_listener = ExExEventListener::new(
        ctx,
        common_components.datastore,
        Arc::new(worker_pool),
    );

    info!(
        max_concurrent = config.max_concurrent_simulations,
        "ExEx event listener initialized successfully"
    );

    Ok(consensus_listener)
}

/// Initialize mempool event listener that processes mempool transactions
/// 
/// Note: The worker pool is created but NOT started.
pub async fn init_mempool_event_listener<Node>(
    provider: Arc<Node::Provider>,
    config: MempoolListenerConfig,
    max_concurrent_simulations: usize,
    simulation_timeout_ms: u64,
) -> Result<TipsMempoolEventListener<Node>>
where
    Node: FullNodeComponents,
{
    info!("Initializing mempool event listener");

    let common_components = init_common_components(config.database_url.clone(), simulation_timeout_ms).await?;

    let worker_pool = crate::worker_pool::SimulationWorkerPool::new(
        Arc::new(common_components.simulator),
        provider.clone(),
        max_concurrent_simulations,
    );

    let mempool_listener = MempoolEventListener::new(
        provider,
        config,
        Arc::new(worker_pool),
    )?;
    
    info!(
        max_concurrent = max_concurrent_simulations,
        "Mempool event listener initialized successfully"
    );

    Ok(mempool_listener)
}


/// Initialize both event listeners with a shared worker pool
/// 
/// Returns the shared worker pool and both listeners. The worker pool is created
/// but NOT started.
pub async fn init_shared_event_listeners<Node>(
    exex_ctx: ExExContext<Node>,
    exex_config: ExExSimulationConfig,
    mempool_config: MempoolListenerConfig,
    max_concurrent_simulations: usize,
    simulation_timeout_ms: u64,
) -> Result<(Arc<SimulationWorkerPool<RethSimulationEngine, DatabaseResultPublisher, Node::Provider>>, TipsExExEventListener<Node>, TipsMempoolEventListener<Node>)>
where
    Node: FullNodeComponents,
{
    info!("Initializing shared event listeners");

    let common_components = init_common_components(exex_config.database_url.clone(), simulation_timeout_ms).await?;

    let state_provider_factory = Arc::new(exex_ctx.components.provider().clone());

    let shared_worker_pool = Arc::new(SimulationWorkerPool::new(
        Arc::new(common_components.simulator),
        state_provider_factory.clone(),
        max_concurrent_simulations,
    ));

    let exex_listener = ExExEventListener::new(
        exex_ctx,
        common_components.datastore,
        shared_worker_pool.clone(),
    );

    let mempool_listener = MempoolEventListener::new(
        state_provider_factory,
        mempool_config,
        shared_worker_pool.clone(),
    )?;
    
    Ok((shared_worker_pool, exex_listener, mempool_listener))
}

/// Run both listeners with lifecycle management for the shared worker pool
/// Starts the worker pool, runs both listeners concurrently, and ensures proper shutdown
pub async fn run_listeners_with_shared_workers<Node>(
    mut worker_pool: Arc<SimulationWorkerPool<RethSimulationEngine, DatabaseResultPublisher, Node::Provider>>,
    exex_listener: TipsExExEventListener<Node>,
    mempool_listener: TipsMempoolEventListener<Node>,
) -> Result<()>
where
    Node: FullNodeComponents,
{
    info!("Starting shared worker pool");
    
    Arc::get_mut(&mut worker_pool)
        .ok_or_else(|| eyre::eyre!("Cannot get mutable reference to worker pool"))?
        .start();
    
    info!("Running listeners concurrently");
    
    let result = tokio::select! {
        res = exex_listener.run() => {
            info!("ExEx listener completed");
            res
        },
        res = mempool_listener.run() => {
            info!("Mempool listener completed");
            res
        },
    };
    
    info!("Shutting down worker pool");
    match Arc::try_unwrap(worker_pool) {
        Ok(pool) => {
            pool.shutdown().await;
            info!("Worker pool shutdown complete");
        }
        Err(_) => {
            error!("Failed to get ownership of worker pool for shutdown");
        }
    }
    
    result
}
