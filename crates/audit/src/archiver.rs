use crate::metrics::{
    EventType, decrement_in_flight_archive_tasks, increment_events_processed,
    increment_failed_archive_tasks, increment_in_flight_archive_tasks,
    record_archive_event_duration, record_event_age, record_kafka_commit_duration,
    record_kafka_read_duration,
};
use crate::reader::{EventReader, UserOpEventReader};
use crate::storage::{EventWriter, UserOpEventWriter};
use anyhow::Result;
use std::fmt;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::time::sleep;
use tracing::{error, info};

/// Archives audit events from Kafka to S3 storage.
pub struct KafkaAuditArchiver<R, W>
where
    R: EventReader,
    W: EventWriter + Clone + Send + 'static,
{
    reader: R,
    writer: W,
}

impl<R, W> fmt::Debug for KafkaAuditArchiver<R, W>
where
    R: EventReader,
    W: EventWriter + Clone + Send + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KafkaAuditArchiver").finish_non_exhaustive()
    }
}

impl<R, W> KafkaAuditArchiver<R, W>
where
    R: EventReader,
    W: EventWriter + Clone + Send + 'static,
{
    /// Creates a new archiver with the given reader and writer.
    pub fn new(reader: R, writer: W) -> Self {
        Self { reader, writer }
    }

    /// Runs the archiver loop, reading events and writing them to storage.
    pub async fn run(&mut self) -> Result<()> {
        info!("Starting Kafka bundle archiver");

        loop {
            let read_start = Instant::now();
            match self.reader.read_event().await {
                Ok(event) => {
                    record_kafka_read_duration(read_start.elapsed(), EventType::Bundle);

                    let now_ms = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as i64;
                    let event_age_ms = now_ms.saturating_sub(event.timestamp);
                    record_event_age(event_age_ms as f64, EventType::Bundle);

                    // TODO: the integration test breaks because Minio doesn't support etag
                    let writer = self.writer.clone();
                    increment_in_flight_archive_tasks(EventType::Bundle);
                    tokio::spawn(async move {
                        let archive_start = Instant::now();
                        if let Err(e) = writer.archive_event(event).await {
                            error!(error = %e, "Failed to write event");
                            increment_failed_archive_tasks(EventType::Bundle);
                        } else {
                            record_archive_event_duration(archive_start.elapsed(), EventType::Bundle);
                            increment_events_processed(EventType::Bundle);
                        }
                        decrement_in_flight_archive_tasks(EventType::Bundle);
                    });

                    let commit_start = Instant::now();
                    if let Err(e) = self.reader.commit().await {
                        error!(error = %e, "Failed to commit message");
                    }
                    record_kafka_commit_duration(commit_start.elapsed(), EventType::Bundle);
                }
                Err(e) => {
                    error!(error = %e, "Error reading events");
                    sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }
}

pub struct KafkaUserOpAuditArchiver<R, W>
where
    R: UserOpEventReader,
    W: UserOpEventWriter + Clone + Send + 'static,
{
    reader: R,
    writer: W,
}

impl<R, W> KafkaUserOpAuditArchiver<R, W>
where
    R: UserOpEventReader,
    W: UserOpEventWriter + Clone + Send + 'static,
{
    pub fn new(reader: R, writer: W) -> Self {
        Self { reader, writer }
    }

    pub async fn run(&mut self) -> Result<()> {
        info!("Starting Kafka UserOp archiver");

        loop {
            let read_start = Instant::now();
            match self.reader.read_event().await {
                Ok(event) => {
                    record_kafka_read_duration(read_start.elapsed(), EventType::UserOp);

                    let now_ms = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as i64;
                    let event_age_ms = now_ms.saturating_sub(event.timestamp);
                    record_event_age(event_age_ms as f64, EventType::UserOp);

                    let writer = self.writer.clone();
                    increment_in_flight_archive_tasks(EventType::UserOp);
                    tokio::spawn(async move {
                        let archive_start = Instant::now();
                        if let Err(e) = writer.archive_userop_event(event).await {
                            error!(error = %e, "Failed to write UserOp event");
                            increment_failed_archive_tasks(EventType::UserOp);
                        } else {
                            record_archive_event_duration(archive_start.elapsed(), EventType::UserOp);
                            increment_events_processed(EventType::UserOp);
                        }
                        decrement_in_flight_archive_tasks(EventType::UserOp);
                    });

                    let commit_start = Instant::now();
                    if let Err(e) = self.reader.commit().await {
                        error!(error = %e, "Failed to commit message");
                    }
                    record_kafka_commit_duration(commit_start.elapsed(), EventType::UserOp);
                }
                Err(e) => {
                    error!(error = %e, "Error reading UserOp events");
                    sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }
}
