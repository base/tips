use alloy_provider::{ProviderBuilder, RootProvider};
//use anyhow::Context;
use clap::Parser;
use jsonrpsee::server::Server;
use op_alloy_network::Optimism;
use opentelemetry::trace::{Span, TraceContextExt, Tracer, TracerProvider};
use opentelemetry::{Key, KeyValue, Value, global};
//use opentelemetry_otlp::WithExportConfig;
//use opentelemetry_sdk::Resource;
use rdkafka::ClientConfig;
use rdkafka::producer::FutureProducer;
use std::env;
use std::fs;
use std::net::IpAddr;
use std::time::Duration;
use tracing::{info, warn};
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::Layer;
use tracing_subscriber::filter::{LevelFilter, Targets};
use tracing_subscriber::layer::SubscriberExt;
use url::Url;

//use opentelemetry::{
//    trace::{SamplingResult, Span, TraceContextExt, Tracer},
//    InstrumentationScope, Key, KeyValue, Value,
//};
use opentelemetry::InstrumentationScope;
//use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_datadog::{ApiVersion, new_pipeline}; //DatadogTraceStateBuilder,
use opentelemetry_sdk::trace::{self, RandomIdGenerator, Sampler}; //ShouldSample
use opentelemetry_semantic_conventions as semcov;

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

fn bar() {
    let tracer = global::tracer("component-bar");
    let mut span = tracer.start("bar");
    span.set_attribute(KeyValue::new(
        Key::new("span.type"),
        Value::String("sql".into()),
    ));
    span.set_attribute(KeyValue::new(
        Key::new("sql.query"),
        Value::String("SELECT * FROM table".into()),
    ));
    std::thread::sleep(Duration::from_millis(6));
    span.end()
}

/*#[derive(Debug, Clone)]
struct AgentBasedSampler;

impl ShouldSample for AgentBasedSampler {
    fn should_sample(
        &self,
        parent_context: Option<&opentelemetry::Context>,
        _trace_id: opentelemetry::trace::TraceId,
        _name: &str,
        _span_kind: &opentelemetry::trace::SpanKind,
        _attributes: &[opentelemetry::KeyValue],
        _links: &[opentelemetry::trace::Link],
    ) -> opentelemetry::trace::SamplingResult {
        let trace_state = parent_context
            .map(
                |parent_context| parent_context.span().span_context().trace_state().clone(), // inherit sample decision from parent span
            )
            .unwrap_or_else(|| {
                DatadogTraceStateBuilder::default()
                    .with_priority_sampling(true) // always sample root span(span without remote or local parent)
                    .with_measuring(true) // datadog-agent will create metric for this span for APM
                    .build()
            });

        SamplingResult {
            decision: opentelemetry::trace::SamplingDecision::RecordAndSample, // send all spans to datadog-agent
            attributes: vec![],
            trace_state,
        }
    }
}*/

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

    let dd_host = env::var("DD_AGENT_HOST").unwrap_or_else(|_| "localhost".to_string());
    let otlp_endpoint = format!("http://{}:{}", &dd_host, &config.tracing_otlp_port);

    let handle = std::thread::spawn(move || {
        // from: https://github.com/flashbots/rollup-boost/blob/08ebd3e75a8f4c7ebc12db13b042dee04e132c05/crates/rollup-boost/src/tracing.rs#L127
        let filter_name = "tips-ingress-rpc".to_string();

        let global_filter = Targets::new()
            .with_default(LevelFilter::INFO)
            .with_target(&filter_name, LevelFilter::TRACE);

        let registry = tracing_subscriber::registry().with(global_filter);

        let log_filter = Targets::new()
            .with_default(LevelFilter::INFO)
            .with_target(&filter_name, log_level);

        let writer = tracing_subscriber::fmt::writer::BoxMakeWriter::new(std::io::stdout);

        global::set_text_map_propagator(opentelemetry_datadog::DatadogPropagator::default());
        let mut trace_cfg = trace::Config::default();
        trace_cfg.sampler = Box::new(Sampler::AlwaysOn);
        trace_cfg.id_generator = Box::new(RandomIdGenerator::default());

        let provider = new_pipeline()
            .with_service_name(&filter_name)
            .with_api_version(ApiVersion::Version05)
            .with_trace_config(trace_cfg)
            //.with_http_client(reqwest::Client::new())
            .with_agent_endpoint(&otlp_endpoint) // TODO: do we need to configure HTTP client?
            .install_batch()
            .expect("Failed to build provider"); // TODO: use batch exporter later
        global::set_tracer_provider(provider.clone());
        let scope = InstrumentationScope::builder(filter_name.clone())
            .with_version(env!("CARGO_PKG_VERSION"))
            .with_schema_url(semcov::SCHEMA_URL)
            .with_attributes(None)
            .build();
        let tracer = provider.tracer_with_scope(scope);
        tracer.in_span("foo", |cx| {
            let span = cx.span();
            span.set_attribute(KeyValue::new(
                Key::new("span.type"),
                Value::String("web".into()),
            ));
            span.set_attribute(KeyValue::new(
                Key::new("http.url"),
                Value::String("http://localhost:8080/foo".into()),
            ));
            span.set_attribute(KeyValue::new(
                Key::new("http.method"),
                Value::String("GET".into()),
            ));
            span.set_attribute(KeyValue::new(Key::new("http.status_code"), Value::I64(200)));

            std::thread::sleep(Duration::from_millis(6));
            bar();
            std::thread::sleep(Duration::from_millis(6));
        });

        let trace_filter = Targets::new()
            .with_default(LevelFilter::OFF)
            .with_target(&filter_name, LevelFilter::TRACE);

        let registry = registry.with(OpenTelemetryLayer::new(tracer).with_filter(trace_filter));

        tracing::subscriber::set_global_default(
            registry.with(
                tracing_subscriber::fmt::layer()
                    .json()
                    .with_ansi(false)
                    .with_writer(writer)
                    .with_filter(log_filter.clone()),
            ),
        )
        .expect("Failed to set global default");
    });
    handle.join().expect("Failed to join thread");

    info!(
        message = "Starting ingress service",
        address = %config.address,
        port = config.port,
        mempool_url = %config.mempool_url,
        endpoint = %format!("http://{}:{}", &dd_host, &config.tracing_otlp_port)
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
    // TODO: might need shutdown
    // let _ = provider.shutdown();
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

#[cfg(test)]
mod tests {
    use super::*;
    use opentelemetry_sdk::trace::SdkTracerProvider;
    use tracing_subscriber::Registry;
    use tracing_subscriber::fmt::writer::BoxMakeWriter;
    use tracing_subscriber::layer::Layered;

    const FILTER_NAME: &str = "tips-ingress-rpc";

    fn setup() -> (Layered<Targets, Registry>, Targets, BoxMakeWriter) {
        let global_filter = Targets::new()
            .with_default(LevelFilter::INFO)
            .with_target(FILTER_NAME, LevelFilter::TRACE);

        let registry: Layered<Targets, Registry> =
            tracing_subscriber::registry().with(global_filter);

        let log_filter = Targets::new()
            .with_default(LevelFilter::INFO)
            .with_target(FILTER_NAME, LevelFilter::TRACE);

        let writer = BoxMakeWriter::new(std::io::stdout);

        (registry, log_filter, writer)
    }

    fn build_provider() -> SdkTracerProvider {
        let mut trace_cfg = trace::Config::default();
        trace_cfg.sampler = Box::new(Sampler::AlwaysOn);
        trace_cfg.id_generator = Box::new(RandomIdGenerator::default());

        let provider = new_pipeline()
            .with_service_name(FILTER_NAME)
            .with_api_version(ApiVersion::Version05)
            .with_trace_config(trace_cfg)
            .with_agent_endpoint("http://localhost:4317")
            .install_simple()
            .expect("Failed to build provider");
        provider
    }

    #[tokio::test]
    async fn test_build_provider() {
        let handle = std::thread::spawn(|| {
            build_provider();
        });

        handle.join().unwrap();
    }

    #[tokio::test]
    async fn test_build_tracer() {
        let handle = std::thread::spawn(move || {
            let (registry, log_filter, writer) = setup();

            let provider = build_provider();
            global::set_tracer_provider(provider.clone());
            let scope = InstrumentationScope::builder(FILTER_NAME.to_string())
                .with_version(env!("CARGO_PKG_VERSION"))
                .with_schema_url(semcov::SCHEMA_URL)
                .with_attributes(None)
                .build();
            let tracer = provider.tracer_with_scope(scope);

            let trace_filter = Targets::new()
                .with_default(LevelFilter::OFF)
                .with_target(FILTER_NAME, LevelFilter::TRACE);

            let registry = registry.with(OpenTelemetryLayer::new(tracer).with_filter(trace_filter));

            tracing::subscriber::set_global_default(
                registry.with(
                    tracing_subscriber::fmt::layer()
                        .json()
                        .with_ansi(false)
                        .with_writer(writer)
                        .with_filter(log_filter.clone()),
                ),
            )
            .expect("Failed to set global default");
        });
        handle.join().expect("Failed to join thread");
    }
}
