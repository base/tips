mod config;
mod load;
mod metrics;
mod output;
mod poller;
mod sender;
mod setup;
mod tracker;
mod wallet;

use anyhow::Result;
use clap::Parser;
use config::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<()> {

    let cli = Cli::parse();

    match cli.command {
        Commands::Setup(args) => setup::run(args).await,
        Commands::Load(args) => load::run(args).await,
    }
}
