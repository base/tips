use crate::{listeners::MempoolListenerConfig, types::ExExSimulationConfig};
use anyhow::{anyhow, Result};
use clap::Args;
use std::path::PathBuf;

#[derive(Debug, Clone, Args)]
#[command(next_help_heading = "Simulator")]
pub struct SimulatorNodeConfig {
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
    #[arg(
        long,
        env = "TIPS_SIMULATOR_KAFKA_BROKERS",
        default_value = "localhost:9092"
    )]
    pub kafka_brokers: String,

    /// Kafka topic for mempool events
    #[arg(long, env = "TIPS_SIMULATOR_KAFKA_TOPIC", default_value = "tips-audit")]
    pub kafka_topic: String,

    /// Kafka consumer group ID
    #[arg(
        long,
        env = "TIPS_SIMULATOR_KAFKA_GROUP_ID",
        default_value = "tips-simulator"
    )]
    pub kafka_group_id: String,

    /// Chain block time for simulator extensions
    #[arg(long = "chain.block-time", default_value_t = 1000)]
    pub chain_block_time: u64,

    /// Path to builder playground to automatically start up the node connected to it
    #[arg(
        long = "builder.playground",
        num_args = 0..=1,
        default_missing_value = "$HOME/.playground/devnet/",
        value_parser = expand_path,
        env = "TIPS_SIMULATOR_PLAYGROUND_DIR",
    )]
    pub playground: Option<PathBuf>,
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

impl From<&SimulatorNodeConfig> for MempoolListenerConfig {
    fn from(config: &SimulatorNodeConfig) -> Self {
        Self {
            kafka_brokers: config
                .kafka_brokers
                .split(',')
                .map(|s| s.trim().to_string())
                .collect(),
            kafka_topic: config.kafka_topic.clone(),
            kafka_group_id: config.kafka_group_id.clone(),
            database_url: config.database_url.clone(),
        }
    }
}

impl SimulatorNodeConfig {
    pub fn chain_block_time(&self) -> u64 {
        self.chain_block_time
    }

    pub fn has_playground(&self) -> bool {
        self.playground.is_some()
    }
}

fn expand_path(s: &str) -> Result<PathBuf> {
    shellexpand::full(s)
        .map_err(|e| anyhow!("expansion error for `{s}`: {e}"))?
        .into_owned()
        .parse()
        .map_err(|e| anyhow!("invalid path after expansion: {e}"))
}
