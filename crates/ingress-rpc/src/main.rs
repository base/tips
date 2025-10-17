use alloy_provider::{ProviderBuilder, RootProvider};
use clap::Parser;
use jsonrpsee::server::Server;
use op_alloy_network::Optimism;
use opentelemetry::global;
//use opentelemetry::trace::Tracer;
//use opentelemetry::{InstrumentationScope, trace::TracerProvider};
//use opentelemetry_sdk::trace;
use opentelemetry::trace::TracerProvider;
use opentelemetry_sdk::trace::BatchSpanProcessor;
use opentelemetry_sdk::trace::Sampler;
//use opentelemetry_semantic_conventions as semcov;
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::trace::SdkTracerProvider;
use rdkafka::ClientConfig;
use rdkafka::producer::FutureProducer;
use std::env;
use std::fs;
use std::net::IpAddr;
use tracing::{info, warn};
//use tracing_subscriber::Layer;
//use tracing_subscriber::filter::{LevelFilter, Targets};
use opentelemetry_otlp::{SpanExporter, WithExportConfig};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{Layer, Registry};
use url::Url;

mod queue;
mod service;
mod validation;
use queue::KafkaQueuePublisher;
use service::{IngressApiServer, IngressService};

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

    /// Enable tracing
    #[arg(long, env = "TIPS_INGRESS_TRACING_ENABLED", default_value = "false")]
    tracing_enabled: bool,

    /// Port for the OTLP endpoint
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

    /*tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level.to_string())),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let global_filter = Targets::new()
        .with_default(LevelFilter::INFO)
        .with_target(env!("CARGO_PKG_NAME"), LevelFilter::TRACE);
    */

    /*global::set_text_map_propagator(opentelemetry_datadog::DatadogPropagator::default());

    let log_filter = Targets::new()
        .with_default(LevelFilter::INFO)
        .with_target(env!("CARGO_PKG_NAME"), log_level);

    let dd_host = env::var("DD_AGENT_HOST").unwrap_or_else(|_| "localhost".to_string());
    let otlp_endpoint = format!("http://{}:{}", dd_host, config.tracing_otlp_port);

    let mut trace_config = trace::Config::default();
    trace_config.sampler = Box::new(Sampler::AlwaysOn);
    trace_config.id_generator = Box::new(RandomIdGenerator::default());

    let provider = opentelemetry_datadog::new_pipeline()
        .with_service_name(env!("CARGO_PKG_NAME"))
        .with_api_version(opentelemetry_datadog::ApiVersion::Version05)
        .with_agent_endpoint(&otlp_endpoint)
        .with_trace_config(trace_config)
        .install_batch()?;

    global::set_tracer_provider(provider.clone());

    let scope = InstrumentationScope::builder(env!("CARGO_PKG_NAME"))
        .with_version(env!("CARGO_PKG_VERSION"))
        .with_schema_url(semcov::SCHEMA_URL)
        .with_attributes(None)
        .build();

    let tracer = provider.tracer_with_scope(scope);
    tracer.in_span("span_main", |_span| {
        info!(
            message = "Tracing enabled",
            endpoint = %otlp_endpoint
        );
    });

    tracing_subscriber::registry()
        .with(tracing_opentelemetry::OpenTelemetryLayer::new(tracer))
        .with(tracing_subscriber::fmt::layer().with_filter(log_filter))
        .init();*/

    let filter = tracing_subscriber::EnvFilter::new(log_level.to_string());

    let log_layer = tracing_subscriber::fmt::layer()
        .with_line_number(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE)
        .json()
        .boxed();

    let dd_host = env::var("DD_AGENT_HOST").unwrap_or_else(|_| "localhost".to_string());
    let otlp_endpoint = format!("http://{}:{}", dd_host, config.tracing_otlp_port);

    // https://github.com/commonwarexyz/monorepo/blob/27e6f73fce91fc46ef7170e928cbcf96cc635fea/runtime/src/tokio/tracing.rs#L10
    let exporter = SpanExporter::builder()
        .with_http()
        .with_endpoint(&otlp_endpoint)
        .build()?;

    let batch_processor = BatchSpanProcessor::builder(exporter).build();

    let resource = Resource::builder_empty()
        .with_service_name(env!("CARGO_PKG_NAME"))
        .build();

    let tracer_provider = SdkTracerProvider::builder()
        .with_span_processor(batch_processor)
        .with_resource(resource)
        .with_sampler(Sampler::AlwaysOn)
        .build();

    // Create the tracer and set it globally
    let tracer = tracer_provider.tracer(env!("CARGO_PKG_NAME"));
    global::set_tracer_provider(tracer_provider);

    let trace_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    let register = Registry::default()
        .with(filter)
        .with(log_layer)
        .with(trace_layer);
    tracing::subscriber::set_global_default(register)?;

    info!(
        message = "Starting ingress service",
        address = %config.address,
        port = config.port,
        mempool_url = %config.mempool_url,
        endpoint = %otlp_endpoint
    );

    let op_provider: RootProvider<Optimism> = ProviderBuilder::new()
        .disable_recommended_fillers()
        .network::<Optimism>()
        .connect_http(config.mempool_url);

    let client_config = load_kafka_config_from_file(&config.ingress_kafka_properties)?;

    let queue_producer: FutureProducer = client_config.create()?;

    let queue = KafkaQueuePublisher::new(queue_producer, config.ingress_topic);

    let service = IngressService::new(
        op_provider,
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
