use std::time::Duration;
use crate::reader::{KafkaMempoolReader, MempoolEventReader, TimestampedEvent};
use crate::storage::{MempoolEventWriter, S3MempoolEventWriter};
use anyhow::Result;
use tokio::time::sleep;
use tracing::{error, info};


pub struct KafkaMempoolArchiver {
    reader: KafkaMempoolReader,
    writer: S3MempoolEventWriter,
}


impl KafkaMempoolArchiver {
    pub async fn new(
        kafka_brokers: &str,
        topic: String,
        group_id: String,
        bucket: String,
    ) -> Result<Self> {
        let reader = KafkaMempoolReader::new(kafka_brokers, topic, group_id).await?;
        let writer = S3MempoolEventWriter::new(bucket).await?;

        Ok(Self { reader, writer })
    }

    pub async fn run(&mut self) -> Result<()> {
        info!(
            topic = %self.reader.topic(),
            group_id = %self.reader.group_id(),
            "Starting Kafka mempool archiver"
        );

        loop {
            match self.reader.read_events().await {
                Ok(timestamped_events) => {
                    for timestamped_event in timestamped_events {
                        if let Err(e) = self.writer.write_event(timestamped_event.event).await {
                            error!(
                                error = %e,
                                "Failed to write event"
                            );
                            continue;
                        }
                    }

                    if let Err(e) = self.reader.commit().await {
                        error!(
                            error = %e,
                            "Failed to commit message"
                        );
                    }
                }
                Err(e) => {
                    error!(error = %e, "Error reading events");
                    sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }
}
