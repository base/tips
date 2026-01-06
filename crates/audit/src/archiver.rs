use crate::metrics::{
    EventType, increment_events_processed, record_archive_event_duration, record_event_age,
    record_kafka_commit_duration, record_kafka_read_duration,
};
use crate::reader::{EventReader, UserOpEventReader};
use crate::storage::{EventWriter, UserOpEventWriter};
use anyhow::Result;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::time::sleep;
use tracing::{error, info};

pub struct KafkaAuditArchiver<R, W>
where
    R: EventReader,
    W: EventWriter + Clone + Send + 'static,
{
    reader: R,
    writer: W,
}

impl<R, W> KafkaAuditArchiver<R, W>
where
    R: EventReader,
    W: EventWriter + Clone + Send + 'static,
{
    pub fn new(reader: R, writer: W) -> Self {
        Self { reader, writer }
    }

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
                    tokio::spawn(async move {
                        let archive_start = Instant::now();
                        if let Err(e) = writer.archive_event(event).await {
                            error!(error = %e, "Failed to write event");
                        } else {
                            record_archive_event_duration(archive_start.elapsed(), EventType::Bundle);
                            increment_events_processed(EventType::Bundle);
                        }
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
                    tokio::spawn(async move {
                        let archive_start = Instant::now();
                        if let Err(e) = writer.archive_userop_event(event).await {
                            error!(error = %e, "Failed to write UserOp event");
                        } else {
                            record_archive_event_duration(archive_start.elapsed(), EventType::UserOp);
                            increment_events_processed(EventType::UserOp);
                        }
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
