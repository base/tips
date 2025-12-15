use crate::metrics::Metrics;
use crate::reader::EventReader;
use crate::storage::EventWriter;
use anyhow::Result;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info};

pub struct KafkaAuditArchiver<R, W>
where
    R: EventReader,
    W: EventWriter,
{
    reader: R,
    writer: W,
    metrics: Metrics,
}

impl<R, W> KafkaAuditArchiver<R, W>
where
    R: EventReader,
    W: EventWriter + Clone + Send + Sync + 'static,
{
    pub fn new(reader: R, writer: W) -> Self {
        Self {
            reader,
            writer,
            metrics: Metrics::default(),
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        info!("Starting Kafka bundle archiver");

        loop {
            match self.reader.read_event().await {
                Ok(event) => {
                    self.metrics.event_received.increment(1);

                    let writer = self.writer.clone();
                    let metrics = self.metrics.clone();

                    tokio::spawn(async move {
                        if let Err(e) = writer.archive_event(event).await {
                            metrics.event_writing_error.increment(1);
                            error!(
                                error = %e,
                                "Failed to write event"
                            );
                        }
                        metrics.event_written.increment(1);
                    });
                }
                Err(e) => {
                    error!(error = %e, "Error reading events");
                    sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }
}
