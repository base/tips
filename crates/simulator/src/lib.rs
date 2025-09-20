pub mod config;
pub mod core;
pub mod engine;
pub mod exex;
pub mod mempool;
pub mod publisher;
pub mod types;

use eyre::Result;
use reth_exex::ExExContext;
use reth_node_api::FullNodeComponents;
use std::sync::Arc;
use tracing::info;

pub use config::{SimulatorExExConfig, SimulatorNodeConfig};
pub use core::BundleSimulator;
pub use engine::{create_simulation_engine, SimulationEngine, RethSimulationEngine};
pub use exex::ExExEventSimulator;
pub use mempool::{MempoolEventSimulator, MempoolSimulatorConfig, MempoolEventListener, KafkaMempoolListener};
pub use publisher::{create_database_publisher, SimulationResultPublisher, DatabaseResultPublisher};
pub use types::{SimulationResult, SimulationError, ExExSimulationConfig};

// Type aliases for concrete implementations
pub type TipsBundleSimulator = BundleSimulator<RethSimulationEngine, DatabaseResultPublisher>;
pub type TipsExExEventSimulator<Node> = ExExEventSimulator<Node, RethSimulationEngine, DatabaseResultPublisher, tips_datastore::PostgresDatastore>;
pub type TipsMempoolEventSimulator = MempoolEventSimulator<RethSimulationEngine, DatabaseResultPublisher, KafkaMempoolListener>;

// Initialization functions

/// Initialize ExEx event simulator (ExEx) that processes committed blocks
pub async fn init_exex_event_simulator<Node>(
    ctx: ExExContext<Node>,
    config: ExExSimulationConfig,
) -> Result<TipsExExEventSimulator<Node>>
where
    Node: FullNodeComponents,
{
    info!("Initializing ExEx event simulator");

    // Create database connection and publisher
    let datastore = Arc::new(
        tips_datastore::PostgresDatastore::connect(config.database_url.clone()).await
            .map_err(|e| eyre::eyre!("Failed to connect to database: {}", e))?
    );
    
    // Run database migrations
    datastore.run_migrations().await
        .map_err(|e| eyre::eyre!("Failed to run migrations: {}", e))?;
    info!("Database migrations completed");

    let publisher = create_database_publisher(datastore);
    info!("Database publisher initialized");

    // Create simulation engine
    let engine = create_simulation_engine(config.simulation_timeout_ms);
    info!(
        timeout_ms = config.simulation_timeout_ms,
        "Simulation engine initialized"
    );

    // Create core bundle simulator with shared logic
    let core_simulator = BundleSimulator::new(
        engine,
        publisher,
    );
    info!("Core bundle simulator initialized");

    // Get state provider factory for ExEx event simulation
    let state_provider_factory = Arc::new(ctx.components.provider().clone());

    // Create the ExEx event simulator
    let consensus_simulator = ExExEventSimulator::new(
        ctx,
        core_simulator,
        state_provider_factory,
        datastore,
        config.max_concurrent_simulations,
    );

    info!(
        max_concurrent = config.max_concurrent_simulations,
        "ExEx event simulator initialized successfully"
    );

    Ok(consensus_simulator)
}

/// Initialize mempool event simulator that processes mempool transactions
pub async fn init_mempool_event_simulator(
    config: MempoolSimulatorConfig,
) -> Result<TipsMempoolEventSimulator> {
    info!("Initializing mempool event simulator");

    // Create database connection and publisher
    let datastore = Arc::new(
        tips_datastore::PostgresDatastore::connect(config.database_url.clone()).await
            .map_err(|e| eyre::eyre!("Failed to connect to database: {}", e))?
    );
    
    // Run database migrations
    datastore.run_migrations().await
        .map_err(|e| eyre::eyre!("Failed to run migrations: {}", e))?;
    info!("Database migrations completed");

    let publisher = create_database_publisher(datastore);
    info!("Database publisher initialized");

    // Create simulation engine
    let engine = create_simulation_engine(config.simulation_timeout_ms);
    info!(
        timeout_ms = config.simulation_timeout_ms,
        "Simulation engine initialized"
    );

    // Create core bundle simulator with shared logic
    let core_simulator = BundleSimulator::new(
        engine,
        publisher,
    );
    info!("Core bundle simulator initialized");

    // Create Kafka listener
    let listener = KafkaMempoolListener::new(config.clone());
    
    // Create the mempool event simulator
    let mempool_simulator = MempoolEventSimulator::new(core_simulator, listener, config);
    
    info!("Mempool event simulator initialized successfully");

    Ok(mempool_simulator)
}
