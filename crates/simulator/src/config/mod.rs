mod simulator_node;
pub mod playground;

pub use playground::PlaygroundOptions;
pub use simulator_node::SimulatorNodeConfig;

use crate::listeners::MempoolListenerConfig;
use crate::types::ExExSimulationConfig;
use clap::{CommandFactory, FromArgMatches};
use reth_optimism_cli::{chainspec::OpChainSpecParser, commands::Commands, Cli as OpCli};

pub type Cli = OpCli<OpChainSpecParser, SimulatorNodeConfig>;

/// Parse CLI args with playground configuration if specified
pub trait CliExt {
    /// Populates default reth node args when `--builder.playground` is provided.
    fn populate_defaults(self) -> Self;

    /// Returns parsed config with defaults applied if applicable.
    fn parsed() -> Self;
}

impl CliExt for Cli {
    fn populate_defaults(self) -> Self {
        let Commands::Node(ref node_command) = self.command else {
            return self;
        };

        let Some(ref playground_dir) = node_command.ext.playground else {
            return self;
        };

        let options = PlaygroundOptions::new(playground_dir).unwrap_or_else(|e| exit(e));

        options.apply(self)
    }

    fn parsed() -> Self {
        let matches = Cli::command().get_matches();
        Cli::from_arg_matches(&matches)
            .expect("Parsing args")
            .populate_defaults()
    }
}

impl SimulatorNodeConfig {
    pub fn into_parts(
        self,
        cli: Cli,
    ) -> (Cli, ExExSimulationConfig, MempoolListenerConfig, u64) {
        let exex_config = (&self).into();
        let mempool_config = (&self).into();
        (
            cli,
            exex_config,
            mempool_config,
            self.chain_block_time,
        )
    }
}

/// Following clap's convention, a failure to apply defaults exits non-zero.
fn exit(error: anyhow::Error) -> ! {
    eprintln!("{error}");
    std::process::exit(-1);
}
