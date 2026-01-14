use metrics::{Counter, Gauge, Histogram};
use metrics_derive::Metrics;
use std::time::Duration;

/// Event type tag for metrics differentiation
#[derive(Clone, Copy)]
pub(crate) enum EventType {
    Bundle,
    UserOp,
}

impl EventType {
    fn as_str(&self) -> &'static str {
        match self {
            EventType::Bundle => "bundle",
            EventType::UserOp => "userop",
        }
    }
}

pub(crate) fn record_archive_event_duration(duration: Duration, event_type: EventType) {
    metrics::histogram!("tips_audit_archive_event_duration", "type" => event_type.as_str())
        .record(duration.as_secs_f64());
}

pub(crate) fn record_event_age(age_ms: f64, event_type: EventType) {
    metrics::histogram!("tips_audit_event_age", "type" => event_type.as_str()).record(age_ms);
}

pub(crate) fn record_kafka_read_duration(duration: Duration, event_type: EventType) {
    metrics::histogram!("tips_audit_kafka_read_duration", "type" => event_type.as_str())
        .record(duration.as_secs_f64());
}

pub(crate) fn record_kafka_commit_duration(duration: Duration, event_type: EventType) {
    metrics::histogram!("tips_audit_kafka_commit_duration", "type" => event_type.as_str())
        .record(duration.as_secs_f64());
}

pub(crate) fn increment_events_processed(event_type: EventType) {
    metrics::counter!("tips_audit_events_processed", "type" => event_type.as_str()).increment(1);
}

pub(crate) fn increment_in_flight_archive_tasks(event_type: EventType) {
    metrics::gauge!("tips_audit_in_flight_archive_tasks", "type" => event_type.as_str())
        .increment(1.0);
}

pub(crate) fn decrement_in_flight_archive_tasks(event_type: EventType) {
    metrics::gauge!("tips_audit_in_flight_archive_tasks", "type" => event_type.as_str())
        .decrement(1.0);
}

pub(crate) fn increment_failed_archive_tasks(event_type: EventType) {
    metrics::counter!("tips_audit_failed_archive_tasks", "type" => event_type.as_str())
        .increment(1);
}

/// Metrics for audit operations including Kafka reads, S3 writes, and event processing.
#[derive(Metrics, Clone)]
#[metrics(scope = "tips_audit")]
pub struct Metrics {
    /// Duration of archiving a single event.
    #[metric(describe = "Duration of archiving a single event")]
    pub archive_event_duration: Histogram,

    /// Age of events when processed (ms since event creation).
    #[metric(describe = "Age of events when processed")]
    pub event_age: Histogram,

    /// Duration of Kafka read operations.
    #[metric(describe = "Duration of Kafka read operations")]
    pub kafka_read_duration: Histogram,

    /// Duration of Kafka commit operations.
    #[metric(describe = "Duration of Kafka commit operations")]
    pub kafka_commit_duration: Histogram,

    /// Total events processed.
    #[metric(describe = "Total events processed")]
    pub events_processed: Counter,

    /// Number of in-flight archive tasks.
    #[metric(describe = "Number of in-flight archive tasks")]
    pub in_flight_archive_tasks: Gauge,

    /// Duration of update_bundle_history operations.
    #[metric(describe = "Duration of update_bundle_history")]
    pub update_bundle_history_duration: Histogram,

    /// Duration of updating all transaction indexes.
    #[metric(describe = "Duration of update all transaction indexes")]
    pub update_tx_indexes_duration: Histogram,

    /// Duration of S3 get_object operations.
    #[metric(describe = "Duration of S3 get_object")]
    pub s3_get_duration: Histogram,

    /// Duration of S3 put_object operations.
    #[metric(describe = "Duration of S3 put_object")]
    pub s3_put_duration: Histogram,

    /// Total S3 writes skipped due to deduplication.
    #[metric(describe = "Total S3 writes skipped due to dedup")]
    pub s3_writes_skipped: Counter,
}
