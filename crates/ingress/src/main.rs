use alloy_provider::{ProviderBuilder, RootProvider};
use clap::Parser;
use jsonrpsee::server::Server;
use std::net::IpAddr;
use tracing::info;
use url::Url;

mod service;
use service::{IngressApiServer, IngressService};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Config {
    /// Address to bind the RPC server to
    #[arg(long, env = "INGRESS_ADDRESS", default_value = "127.0.0.1")]
    address: IpAddr,

    /// Port to bind the RPC server to
    #[arg(long, env = "INGRESS_PORT", default_value = "8080")]
    port: u16,

    /// URL of the mempool service to proxy transactions to
    #[arg(long, env = "MEMPOOL_URL")]
    mempool_url: Url,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let config = Config::parse();
    info!(
        message = "Starting ingress service",
        address = %config.address,
        port = config.port,
        mempool_url = %config.mempool_url
    );

    let provider: RootProvider = ProviderBuilder::new()
        .disable_recommended_fillers()
        .connect_http(config.mempool_url);

    let service = IngressService::new(provider);
    let bind_addr = format!("{}:{}", config.address, config.port);

    let server = Server::builder().build(&bind_addr).await?;
    let addr = server.local_addr()?;
    let handle = server.start(service.into_rpc());

    info!(
        message = "Ingress RPC server started",
        address = %addr
    );

    handle.stopped().await;
    Ok(())
}
