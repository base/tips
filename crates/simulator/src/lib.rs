pub mod config;
pub mod engine;
pub mod exex;
pub mod listener;
pub mod publisher;
pub mod service;
pub mod state;
pub mod types;

use anyhow::Result;
use reth_exex::ExExContext;
use reth_node_api::{FullNodeComponents, NodeAddOns};
use std::sync::Arc;
use tracing::info;

pub use config::{SimulatorConfig, SimulatorExExConfig, SimulatorNodeConfig};
pub use engine::{create_simulation_engine, SimulationEngine};
pub use exex::SimulatorExEx;
pub use listener::{MempoolEventListener, KafkaMempoolListener};
pub use publisher::{create_database_publisher, SimulationResultPublisher};
pub use service::SimulatorService;
pub use state::{create_direct_state_provider, StateProvider};
pub use types::{SimulationResult, SimulationError, ExExSimulationConfig};

/// ExEx initialization function that should be called by reth
pub async fn init_simulator_exex<Node, AddOns>(
    ctx: ExExContext<Node, AddOns>,
    config: ExExSimulationConfig,
) -> Result<SimulatorExEx<Node, AddOns>>
where
    Node: FullNodeComponents,
    AddOns: NodeAddOns<Node>,
{
    info!("Initializing Tips Simulator ExEx");

    // Create database connection and publisher
    let datastore = Arc::new(
        tips_datastore::PostgresDatastore::connect(config.database_url.clone()).await?
    );
    
    // Run database migrations
    datastore.run_migrations().await?;
    info!("Database migrations completed");

    let publisher = Box::new(create_database_publisher(datastore));
    info!("Database publisher initialized");

    // Create state provider using reth's provider factory
    let state_provider_factory = ctx.components.provider().clone();
    let current_block_number = ctx.head.number;
    let state_provider = Arc::new(create_direct_state_provider(
        state_provider_factory,
        current_block_number,
    ));
    info!(
        current_block = current_block_number,
        "Direct state provider initialized"
    );

    // Create simulation engine
    let engine = Box::new(create_simulation_engine(
        state_provider,
        config.simulation_timeout_ms,
    ));
    info!(
        timeout_ms = config.simulation_timeout_ms,
        "Simulation engine initialized"
    );

    // Create the ExEx
    let exex = SimulatorExEx::new(
        ctx,
        engine,
        publisher,
        config.max_concurrent_simulations,
    );

    info!(
        max_concurrent = config.max_concurrent_simulations,
        "Tips Simulator ExEx initialized successfully"
    );

    Ok(exex)
}
