use alloy_provider::{ProviderBuilder, RootProvider};
use clap::Parser;
use jsonrpsee::server::Server;
use op_alloy_network::Optimism;
use rdkafka::ClientConfig;
use rdkafka::producer::FutureProducer;
use std::fs;
use std::net::IpAddr;
use tips_common::init_tracing;
use tracing::{info, warn};
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use url::Url;

mod queue;
mod service;
mod validation;
use queue::KafkaQueuePublisher;
use service::{IngressApiServer, IngressService};
use tracing_subscriber::Layer;
use tracing_subscriber::filter::{LevelFilter, Targets};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Config {
    /// Address to bind the RPC server to
    #[arg(long, env = "TIPS_INGRESS_ADDRESS", default_value = "0.0.0.0")]
    address: IpAddr,

    /// Port to bind the RPC server to
    #[arg(long, env = "TIPS_INGRESS_PORT", default_value = "8080")]
    port: u16,

    /// URL of the mempool service to proxy transactions to
    #[arg(long, env = "TIPS_INGRESS_RPC_MEMPOOL")]
    mempool_url: Url,

    /// Enable dual writing raw transactions to the mempool
    #[arg(long, env = "TIPS_INGRESS_DUAL_WRITE_MEMPOOL", default_value = "false")]
    dual_write_mempool: bool,

    /// Kafka brokers for publishing mempool events
    #[arg(long, env = "TIPS_INGRESS_KAFKA_INGRESS_PROPERTIES_FILE")]
    ingress_kafka_properties: String,

    /// Kafka topic for queuing transactions before the DB Writer
    #[arg(
        long,
        env = "TIPS_INGRESS_KAFKA_INGRESS_TOPIC",
        default_value = "tips-ingress"
    )]
    ingress_topic: String,

    #[arg(long, env = "TIPS_INGRESS_LOG_LEVEL", default_value = "info")]
    log_level: String,

    /// Default lifetime for sent transactions in seconds (default: 3 hours)
    #[arg(
        long,
        env = "TIPS_INGRESS_SEND_TRANSACTION_DEFAULT_LIFETIME_SECONDS",
        default_value = "10800"
    )]
    send_transaction_default_lifetime_seconds: u64,

    #[arg(long, env = "TIPS_INGRESS_TRACING_ENABLED", default_value = "true")]
    tracing_enabled: bool,

    #[arg(
        long,
        env = "TIPS_INGRESS_TRACING_OTLP_ENDPOINT",
        default_value = "http://localhost:4317"
    )]
    tracing_otlp_endpoint: String,

    #[arg(long, env = "TIPS_INGRESS_TRACING_OTLP_PORT", default_value = "4317")]
    tracing_otlp_port: u16,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let config = Config::parse();

    let log_level = match config.log_level.to_lowercase().as_str() {
        "trace" => tracing::Level::TRACE,
        "debug" => tracing::Level::DEBUG,
        "info" => tracing::Level::INFO,
        "warn" => tracing::Level::WARN,
        "error" => tracing::Level::ERROR,
        _ => {
            warn!(
                "Invalid log level '{}', defaulting to 'info'",
                config.log_level
            );
            tracing::Level::INFO
        }
    };

    if config.tracing_enabled {
        let (trace_filter, tracer) = init_tracing(
            env!("CARGO_PKG_NAME").to_string(),
            env!("CARGO_PKG_VERSION").to_string(),
            config.tracing_otlp_endpoint,
            log_level.to_string(),
            config.tracing_otlp_port,
        )?;

        let log_filter = Targets::new()
            .with_default(LevelFilter::INFO)
            .with_target(env!("CARGO_PKG_NAME"), log_level);

        let global_filter = Targets::new()
            .with_default(LevelFilter::INFO)
            .with_target(env!("CARGO_PKG_NAME"), LevelFilter::TRACE);

        tracing_subscriber::registry()
            .with(global_filter)
            .with(OpenTelemetryLayer::new(tracer).with_filter(trace_filter))
            /*.with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level.to_string())),
            )
            .with(tracing_subscriber::fmt::layer())
            */
            .with(tracing_subscriber::fmt::layer().with_filter(log_filter))
            .init();

        /*init_tracing(
            env!("CARGO_PKG_NAME").to_string(),
            env!("CARGO_PKG_VERSION").to_string(),
            config.tracing_otlp_endpoint,
            log_level.to_string(),
        )?;*/
    }
    info!(
        message = "Starting ingress service",
        address = %config.address,
        port = config.port,
        mempool_url = %config.mempool_url,
        host = %std::env::var("DD_AGENT_HOST").unwrap_or_else(|_| "unknown".to_string()),
        port = %config.tracing_otlp_port,
    );
    info!(
        message = "host",
        host = %std::env::var("DD_AGENT_HOST").unwrap_or_else(|_| "unknown".to_string())
    );
    info!(
        message = "port",
        port = %config.tracing_otlp_port
    );

    let provider: RootProvider<Optimism> = ProviderBuilder::new()
        .disable_recommended_fillers()
        .network::<Optimism>()
        .connect_http(config.mempool_url);

    let client_config = load_kafka_config_from_file(&config.ingress_kafka_properties)?;

    let queue_producer: FutureProducer = client_config.create()?;

    let queue = KafkaQueuePublisher::new(queue_producer, config.ingress_topic);

    let service = IngressService::new(
        provider,
        config.dual_write_mempool,
        queue,
        config.send_transaction_default_lifetime_seconds,
    );
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

fn load_kafka_config_from_file(properties_file_path: &str) -> anyhow::Result<ClientConfig> {
    let kafka_properties = fs::read_to_string(properties_file_path)?;
    info!("Kafka properties:\n{}", kafka_properties);

    let mut client_config = ClientConfig::new();

    for line in kafka_properties.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            client_config.set(key.trim(), value.trim());
        }
    }

    Ok(client_config)
}
