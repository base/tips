pub mod queue;
pub mod service;
pub mod validation;

use tokio::time::Duration;

fn record_histogram(rpc_latency: Duration, rpc: String) {
    metrics::histogram!("tips_ingress_rpc_rpc_latency", "rpc" => rpc.clone())
        .record(rpc_latency.as_millis() as f64);
}
