use metrics::Histogram;
use metrics_derive::Metrics;
use tokio::time::Duration;

/// `record_histogram` lets us record with tags.
pub fn record_histogram(rpc_latency: Duration, rpc: String) {
    metrics::histogram!("tips_ingress_rpc_rpc_latency", "rpc" => rpc.clone())
        .record(rpc_latency.as_secs_f64());
}

/// Metrics for the `tips_ingress_rpc` component.
/// Conventions:
/// - Durations are recorded in seconds (histograms).
/// - Counters are monotonic event counts.
/// - Gauges reflect the current value/state.
#[derive(Metrics, Clone)]
#[metrics(scope = "tips_ingress_rpc")]
pub struct Metrics {
    #[metric(describe = "Duration of validate_tx")]
    pub validate_tx_duration: Histogram,

    #[metric(describe = "Duration of validate_bundle")]
    pub validate_bundle_duration: Histogram,

    #[metric(describe = "Duration of meter_bundle")]
    pub meter_bundle_duration: Histogram,

    #[metric(describe = "Duration of send_raw_transaction")]
    pub send_raw_transaction_duration: Histogram,
}
