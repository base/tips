use crate::types::ExExSimulationConfig;
use clap::Parser;

/// Combined configuration for reth node with simulator ExEx
#[derive(Parser, Debug, Clone)]
#[command(author, version, about = "Reth node with Tips Simulator ExEx")]
pub struct SimulatorNodeConfig {
    /// Reth node arguments
    #[command(flatten)]
    pub node: reth_cli::Cli,

    /// PostgreSQL database connection URL for simulator
    #[arg(long, env = "TIPS_SIMULATOR_DATABASE_URL")]
    pub database_url: String,

    /// Maximum number of concurrent simulations
    #[arg(long, env = "TIPS_SIMULATOR_MAX_CONCURRENT", default_value = "10")]
    pub max_concurrent_simulations: usize,

    /// Timeout for individual simulations in milliseconds
    #[arg(long, env = "TIPS_SIMULATOR_TIMEOUT_MS", default_value = "5000")]
    pub simulation_timeout_ms: u64,
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

impl From<SimulatorNodeConfig> for ExExSimulationConfig {
    fn from(config: SimulatorNodeConfig) -> Self {
        Self {
            database_url: config.database_url,
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
