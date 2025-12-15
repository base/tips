use metrics::Counter;
use metrics_derive::Metrics;
use metrics_exporter_prometheus::PrometheusBuilder;
use std::net::SocketAddr;

/// Metrics for the `tips_audit` component.
#[derive(Metrics, Clone)]
#[metrics(scope = "tips_audit")]
pub struct Metrics {
    #[metric(describe = "Number of events received from Kafka")]
    pub event_received: Counter,

    #[metric(describe = "Number of events persisted to S3")]
    pub event_written: Counter,

    #[metric(describe = "Number of times error writing to S3")]
    pub event_writing_error: Counter,
}

/// Initialize Prometheus metrics exporter
pub fn init_prometheus_exporter(addr: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
    PrometheusBuilder::new()
        .with_http_listener(addr)
        .install()
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
}
