use anyhow::Context;
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::{SpanExporter, WithExportConfig};
use opentelemetry_sdk::{propagation::TraceContextPropagator, trace::SdkTracerProvider, Resource};
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{filter::{LevelFilter, Targets}, layer::SubscriberExt};
use opentelemetry::trace::TracerProvider;

pub fn init_tracing(
    service_name: String,
    service_version: String,
    otlp_endpoint: String,
) -> anyhow::Result<()> {
    global::set_text_map_propagator(TraceContextPropagator::new());

    let otlp_exporter = SpanExporter::builder()
        .with_tonic()
        .with_endpoint(otlp_endpoint)
        .build()
        .context("Failed to create OTLP exporter")?;

    let provider_builder = SdkTracerProvider::builder()
        .with_batch_exporter(otlp_exporter)
        .with_resource(
            Resource::builder_empty()
                .with_attributes([
                    KeyValue::new("service.name", service_name.clone()),
                    KeyValue::new("service.version", service_version),
                ])
                .build(),
        );

    let provider = provider_builder.build();
    let tracer = provider.tracer(service_name.clone());

    let trace_filter = Targets::new()
        .with_default(LevelFilter::OFF)
        .with_target(service_name, LevelFilter::TRACE);

    tracing::subscriber::set_global_default(
        tracing_subscriber::registry()
            .with(trace_filter)
            .with(OpenTelemetryLayer::new(tracer)),
    )?;

    Ok(())
}