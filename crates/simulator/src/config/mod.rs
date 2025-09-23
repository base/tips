pub mod playground;

pub use playground::PlaygroundOptions;

use crate::types::ExExSimulationConfig;
use crate::listeners::MempoolListenerConfig;
use anyhow::{Result, anyhow};
use clap::Parser;
use eyre;
use tracing::info;

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

    /// Path to builder playground to automatically start up the node connected to it
    #[arg(
        long = "builder.playground",
        num_args = 0..=1,
        default_missing_value = "$HOME/.playground/devnet/",
        value_parser = expand_path,
        env = "TIPS_SIMULATOR_PLAYGROUND_DIR",
    )]
    pub playground: Option<std::path::PathBuf>,
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
            kafka_brokers: config.kafka_brokers.split(',').map(|s| s.trim().to_string()).collect(),
            kafka_topic: config.kafka_topic.clone(),
            kafka_group_id: config.kafka_group_id.clone(),
            database_url: config.database_url.clone(),
        }
    }
}

fn expand_path(s: &str) -> Result<std::path::PathBuf> {
    shellexpand::full(s)
        .map_err(|e| anyhow!("expansion error for `{s}`: {e}"))?
        .into_owned()
        .parse()
        .map_err(|e| anyhow!("invalid path after expansion: {e}"))
}

/// Parse CLI args with playground configuration if specified
pub fn parse_config_with_playground() -> eyre::Result<SimulatorNodeConfig> {
    // Debug: print raw args
    eprintln!("Raw args: {:?}", std::env::args().collect::<Vec<_>>());
    
    // First, parse just to check if playground is specified
    let initial_config = SimulatorNodeConfig::parse();
    
    eprintln!("Parsed initial config, playground: {:?}", initial_config.playground);
    
    if let Some(ref playground_dir) = initial_config.playground {
        eprintln!("Detected playground configuration, loading from: {}", playground_dir.display());
        
        // Load playground options
        let options = PlaygroundOptions::new(playground_dir)
            .map_err(|e| eyre::eyre!("Failed to load playground options: {}", e))?;
        
        // Get original args
        let mut args: Vec<String> = std::env::args().collect();
        
        // Get playground args
        let playground_args = options.to_cli_args();
        eprintln!("Playground args to insert: {:?}", playground_args);
        
        // Find where to insert playground args (after "node" subcommand)
        if let Some(node_pos) = args.iter().position(|arg| arg == "node") {
            // Insert playground args right after "node"
            // Insert in reverse order to maintain correct positions
            for arg in playground_args.into_iter().rev() {
                args.insert(node_pos + 1, arg);
            }
        }
        
        eprintln!("Final args with playground config: {:?}", args);
        info!("Re-parsing with playground configuration arguments");
        
        // Re-parse with playground args included
        Ok(SimulatorNodeConfig::parse_from(args))
    } else {
        Ok(initial_config)
    }
}
