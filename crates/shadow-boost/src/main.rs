//! Minimal proxy for driving a shadow builder from a non-sequencer op-node.
//!
//! Intercepts Engine API calls and modifies them to force block building while keeping
//! the op-node synchronized with the canonical chain.

mod auth;
mod config;
mod proxy;
mod server;

use clap::Parser;
use config::Config;
use eyre::Result;
use proxy::ShadowBuilderProxy;
use server::{build_rpc_module, start_server};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();

    let config = Config::parse();

    info!("Starting Shadow Builder Proxy");
    info!("Builder URL: {}", config.builder_url);
    info!("Listen address: {}", config.listen_addr);

    let builder_jwt = config.load_jwt_secret()?;
    let proxy = ShadowBuilderProxy::new(&config.builder_url, builder_jwt, config.timeout_ms)?;
    let rpc_module = build_rpc_module(proxy);

    info!("Shadow Builder Proxy listening on {}", config.listen_addr);
    info!("Point your op-node to this proxy as the execution engine");

    start_server(&config.listen_addr, rpc_module).await?;

    Ok(())
}
