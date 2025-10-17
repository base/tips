use anyhow::Context;
use opentelemetry::global;
use opentelemetry::trace::TracerProvider;
use opentelemetry_otlp::SpanExporter;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_otlp::WithHttpConfig;
use opentelemetry_sdk::trace::SdkTracer;
use opentelemetry_sdk::{propagation::TraceContextPropagator, trace::SdkTracerProvider, Resource};
use tracing::info;
use tracing_subscriber::{
    filter::{LevelFilter, Targets},
    //layer::SubscriberExt,
};

// from: https://github.com/flashbots/rollup-boost/blob/08ebd3e75a8f4c7ebc12db13b042dee04e132c05/crates/rollup-boost/src/tracing.rs#L127
pub fn init_tracing(
    service_name: String,
    _service_version: String,
    otlp_endpoint: String,
    _log_level: String,
    _otlp_port: u16,
) -> anyhow::Result<(Targets, SdkTracer)> {
    global::set_text_map_propagator(TraceContextPropagator::new());

    info!(
        message = "OTLP endpoint",
        endpoint = %otlp_endpoint
    );
    let otlp_exporter = SpanExporter::builder()
        .with_http()
        .with_http_client(reqwest::Client::new())
        .with_endpoint(otlp_endpoint)
        .build()
        .context("Failed to create OTLP exporter")?;

    let provider_builder = SdkTracerProvider::builder()
        .with_batch_exporter(otlp_exporter)
        .with_resource(Resource::builder().build());

    let provider = provider_builder.build();
    let tracer = provider.tracer(service_name.clone());

    let trace_filter = Targets::new()
        .with_default(LevelFilter::OFF)
        .with_target(service_name, LevelFilter::TRACE);

    /*
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level.to_string())),
            )
            .with(tracing_subscriber::fmt::layer())
            .init();
    */

    /*tracing::subscriber::set_global_default(
        tracing_subscriber::registry()
            .with(trace_filter)
            .with(OpenTelemetryLayer::new(tracer))
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level)),
            )
            .with(tracing_subscriber::fmt::layer()),
    )?;*/

    Ok((trace_filter, tracer))
}
