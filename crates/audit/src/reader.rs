use crate::types::MempoolEvent;
use anyhow::Result;
use async_trait::async_trait;
use rdkafka::{
    config::ClientConfig,
    consumer::{Consumer, StreamConsumer},
    message::Message,
    Timestamp, TopicPartitionList,
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::sleep;
use tracing::{error, info};

#[derive(Debug, Clone)]
pub struct TimestampedEvent {
    pub event: MempoolEvent,
    pub timestamp: i64,
}

#[async_trait]
pub trait MempoolEventReader {
    async fn read_events(&mut self) -> Result<Vec<TimestampedEvent>>;
    async fn commit(&mut self) -> Result<()>;
}

pub struct KafkaMempoolReader {
    consumer: StreamConsumer,
    topic: String,
    group_id: String,
    last_message_offset: Option<i64>,
    last_message_partition: Option<i32>,
}

impl KafkaMempoolReader {
    pub async fn new(kafka_brokers: &str, topic: String, group_id: String) -> Result<Self> {
        let consumer: StreamConsumer = ClientConfig::new()
            .set("group.id", &group_id)
            .set("bootstrap.servers", kafka_brokers)
            .set("enable.partition.eof", "false")
            .set("session.timeout.ms", "6000")
            .set("enable.auto.commit", "false")
            .set("auto.offset.reset", "earliest")
            .set("fetch.wait.max.ms", "100")
            .set("fetch.min.bytes", "1")
            .create()?;

        let mut tpl = TopicPartitionList::new();
        tpl.add_partition(&topic, 0);
        consumer.assign(&tpl)?;

        Ok(Self {
            consumer,
            topic,
            group_id,
            last_message_offset: None,
            last_message_partition: None,
        })
    }

    pub fn topic(&self) -> &str {
        &self.topic
    }

    pub fn group_id(&self) -> &str {
        &self.group_id
    }
}

#[async_trait]
impl MempoolEventReader for KafkaMempoolReader {
    async fn read_events(&mut self) -> Result<Vec<TimestampedEvent>> {
        match self.consumer.recv().await {
            Ok(message) => {
                let payload = message
                    .payload()
                    .ok_or_else(|| anyhow::anyhow!("Message has no payload"))?;

                // Extract Kafka timestamp, use current time as fallback
                let timestamp = match message.timestamp() {
                    Timestamp::CreateTime(millis) => millis,
                    Timestamp::LogAppendTime(millis) => millis,
                    Timestamp::NotAvailable => SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as i64,
                };

                let event: MempoolEvent = serde_json::from_slice(payload)?;

                info!(
                    bundle_id = %event.bundle_id(),
                    timestamp = timestamp,
                    offset = message.offset(),
                    partition = message.partition(),
                    "Received event with timestamp"
                );

                self.last_message_offset = Some(message.offset());
                self.last_message_partition = Some(message.partition());

                let timestamped_event = TimestampedEvent { event, timestamp };

                Ok(vec![timestamped_event])
            }
            Err(e) => {
                error!(error = %e, "Error receiving message from Kafka");
                sleep(Duration::from_secs(1)).await;
                Ok(vec![])
            }
        }
    }

    async fn commit(&mut self) -> Result<()> {
        if let (Some(offset), Some(partition)) =
            (self.last_message_offset, self.last_message_partition)
        {
            let mut tpl = TopicPartitionList::new();
            tpl.add_partition_offset(&self.topic, partition, rdkafka::Offset::Offset(offset + 1))?;
            self.consumer
                .commit(&tpl, rdkafka::consumer::CommitMode::Sync)
                .map_err(|e| anyhow::anyhow!("Failed to commit message: {}", e))?;
        }
        Ok(())
    }
}
