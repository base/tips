use crate::types::ExExSimulationConfig;
use crate::mempool::MempoolSimulatorConfig;
use clap::Parser;

/// Combined configuration for reth node with simulator ExEx
#[derive(Parser, Debug)]
#[command(author, version, about = "Reth node with Tips Simulator ExEx")]
pub struct SimulatorNodeConfig {
    /// Reth node arguments
    #[command(flatten)]
    pub node: reth::cli::Cli,

    /// Data directory for simulator
    #[arg(long, env = "TIPS_SIMULATOR_DATADIR", default_value = "~/.tips-simulator-reth")]
    pub datadir: std::path::PathBuf,

    /// PostgreSQL database connection URL for simulator
    #[arg(long, env = "TIPS_SIMULATOR_DATABASE_URL")]
    pub database_url: String,

    /// Maximum number of concurrent simulations
    #[arg(long, env = "TIPS_SIMULATOR_MAX_CONCURRENT", default_value = "10")]
    pub max_concurrent_simulations: usize,

    /// Timeout for individual simulations in milliseconds
    #[arg(long, env = "TIPS_SIMULATOR_TIMEOUT_MS", default_value = "5000")]
    pub simulation_timeout_ms: u64,

    /// Kafka brokers for mempool events (comma-separated)
    #[arg(long, env = "TIPS_SIMULATOR_KAFKA_BROKERS", default_value = "localhost:9092")]
    pub kafka_brokers: String,

    /// Kafka topic for mempool events
    #[arg(long, env = "TIPS_SIMULATOR_KAFKA_TOPIC", default_value = "mempool-events")]
    pub kafka_topic: String,

    /// Kafka consumer group ID
    #[arg(long, env = "TIPS_SIMULATOR_KAFKA_GROUP_ID", default_value = "tips-simulator")]
    pub kafka_group_id: String,
}

/// Legacy standalone ExEx config (for library use)
#[derive(Debug, Clone)]
pub struct SimulatorExExConfig {
    /// PostgreSQL database connection URL
    pub database_url: String,

    /// Maximum number of concurrent simulations
    pub max_concurrent_simulations: usize,

    /// Timeout for individual simulations in milliseconds
    pub simulation_timeout_ms: u64,
}

impl From<&SimulatorNodeConfig> for ExExSimulationConfig {
    fn from(config: &SimulatorNodeConfig) -> Self {
        Self {
            database_url: config.database_url.clone(),
            max_concurrent_simulations: config.max_concurrent_simulations,
            simulation_timeout_ms: config.simulation_timeout_ms,
        }
    }
}

impl From<SimulatorExExConfig> for ExExSimulationConfig {
    fn from(config: SimulatorExExConfig) -> Self {
        Self {
            database_url: config.database_url,
            max_concurrent_simulations: config.max_concurrent_simulations,
            simulation_timeout_ms: config.simulation_timeout_ms,
        }
    }
}

impl From<&SimulatorNodeConfig> for MempoolSimulatorConfig {
    fn from(config: &SimulatorNodeConfig) -> Self {
        Self {
            kafka_brokers: config.kafka_brokers.split(',').map(|s| s.trim().to_string()).collect(),
            kafka_topic: config.kafka_topic.clone(),
            kafka_group_id: config.kafka_group_id.clone(),
            database_url: config.database_url.clone(),
        }
    }
}
