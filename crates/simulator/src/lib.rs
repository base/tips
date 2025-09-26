pub mod config;
pub mod core;
pub mod engine;
pub mod listeners;
pub mod publisher;
pub mod types;
pub mod worker_pool;

use eyre::Result;
use reth_evm::ConfigureEvm;
use reth_exex::ExExContext;
use reth_node_api::FullNodeComponents;
use reth_optimism_evm::OpNextBlockEnvAttributes;
use std::sync::Arc;
use tracing::{error, info};

pub use config::SimulatorNodeConfig;
pub use core::{BundleSimulator, BundleSimulatorImpl};
pub use engine::{RethSimulationEngine, SimulationEngine};
pub use listeners::{ExExEventListener, MempoolEventListener, MempoolListenerConfig};
pub use publisher::{SimulationPublisher, TipsSimulationPublisher};
pub use types::{ExExSimulationConfig, SimulationError, SimulationResult};
pub use worker_pool::SimulationWorkerPool;

// Type aliases for concrete implementations
pub type TipsBundleSimulator<Node> =
    BundleSimulatorImpl<RethSimulationEngine<Node>, TipsSimulationPublisher>;
pub type TipsExExEventListener<Node> =
    ExExEventListener<Node, TipsBundleSimulator<Node>, tips_datastore::PostgresDatastore>;
pub type TipsMempoolEventListener<Node> = MempoolEventListener<Node, TipsBundleSimulator<Node>>;

// Initialization functions

/// Dependencies shared across listeners
struct ListenerDependencies<B>
where
    B: BundleSimulator,
{
    datastore: Arc<tips_datastore::PostgresDatastore>,
    simulator: B,
}

/// Initialize listener dependencies (database, publisher, engine, core simulator)
async fn init_dependencies<Node>(
    provider: Arc<Node::Provider>,
    evm_config: Node::Evm,
    database_url: String,
    kafka_brokers: String,
    kafka_topic: String,
) -> Result<
    ListenerDependencies<BundleSimulatorImpl<RethSimulationEngine<Node>, TipsSimulationPublisher>>,
>
where
    Node: FullNodeComponents,
    <Node as FullNodeComponents>::Evm: ConfigureEvm<NextBlockEnvCtx = OpNextBlockEnvAttributes>,
{
    let datastore = Arc::new(
        tips_datastore::PostgresDatastore::connect(database_url)
            .await
            .map_err(|e| eyre::eyre!("Failed to connect to database: {}", e))?,
    );

    // Create Kafka producer
    let kafka_producer = rdkafka::config::ClientConfig::new()
        .set("bootstrap.servers", &kafka_brokers)
        .set("message.timeout.ms", "5000")
        .create::<rdkafka::producer::FutureProducer>()
        .map_err(|e| eyre::eyre!("Failed to create Kafka producer: {}", e))?;

    let publisher =
        TipsSimulationPublisher::new(Arc::clone(&datastore), kafka_producer, kafka_topic);
    info!(
        kafka_brokers = %kafka_brokers,
        "Database publisher with Kafka initialized"
    );

    let engine = RethSimulationEngine::new(Arc::clone(&provider), evm_config);
    info!("Simulation engine initialized");

    let simulator = BundleSimulatorImpl::new(engine, publisher);
    info!("Core bundle simulator initialized");

    Ok(ListenerDependencies {
        datastore,
        simulator,
    })
}

/// Encapsulates both event listeners with their shared worker pool
///
/// This struct ensures that the ExEx and mempool listeners always use the same
/// worker pool instance, preventing potential misconfigurations.
pub struct ListenersWithWorkers<Node>
where
    Node: FullNodeComponents,
    <Node as FullNodeComponents>::Evm: ConfigureEvm<NextBlockEnvCtx = OpNextBlockEnvAttributes>,
{
    worker_pool: Arc<SimulationWorkerPool<TipsBundleSimulator<Node>>>,
    exex_listener: TipsExExEventListener<Node>,
    mempool_listener: TipsMempoolEventListener<Node>,
}

impl<Node> ListenersWithWorkers<Node>
where
    Node: FullNodeComponents,
    <Node as FullNodeComponents>::Evm: ConfigureEvm<NextBlockEnvCtx = OpNextBlockEnvAttributes>,
{
    /// Initialize both event listeners with a shared worker pool
    ///
    /// The worker pool is created but NOT started. Call `run()` to start
    /// the worker pool and begin processing events.
    pub async fn new(
        exex_ctx: ExExContext<Node>,
        exex_config: ExExSimulationConfig,
        mempool_config: MempoolListenerConfig,
        max_concurrent_simulations: usize,
        _simulation_timeout_ms: u64,
    ) -> Result<Self> {
        info!("Initializing shared event listeners");

        let provider = Arc::new(exex_ctx.components.provider().clone());
        let evm_config = exex_ctx.components.evm_config().clone();

        let dependencies = init_dependencies(
            Arc::clone(&provider),
            evm_config,
            exex_config.database_url.clone(),
            mempool_config.kafka_brokers.join(","),
            mempool_config.kafka_topic.clone(),
        )
        .await?;

        let shared_worker_pool =
            SimulationWorkerPool::new(Arc::new(dependencies.simulator), max_concurrent_simulations);

        let exex_listener = ExExEventListener::new(
            exex_ctx,
            dependencies.datastore,
            Arc::clone(&shared_worker_pool),
        );

        let mempool_listener = MempoolEventListener::new(
            Arc::clone(&provider),
            mempool_config,
            Arc::clone(&shared_worker_pool),
        )?;

        info!(
            max_concurrent = max_concurrent_simulations,
            "Both ExEx and mempool event listeners initialized successfully"
        );

        Ok(Self {
            worker_pool: shared_worker_pool,
            exex_listener,
            mempool_listener,
        })
    }

    /// Run both listeners with lifecycle management for the shared worker pool
    ///
    /// Starts the worker pool, runs both listeners concurrently, and ensures proper shutdown
    pub async fn run(self) -> Result<()> {
        info!("Starting shared worker pool");

        self.worker_pool.start().await;

        info!("Running listeners concurrently");

        let result = tokio::select! {
            res = self.exex_listener.run() => {
                info!("ExEx listener completed");
                res
            },
            res = self.mempool_listener.run() => {
                info!("Mempool listener completed");
                res
            },
        };

        info!("Shutting down worker pool");
        match Arc::try_unwrap(self.worker_pool) {
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
}
