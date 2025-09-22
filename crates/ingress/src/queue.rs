use alloy_primitives::Address;
use alloy_rpc_types_mev::EthSendBundle;
use anyhow::{Error, Result};
use async_trait::async_trait;
use rdkafka::producer::{FutureProducer, FutureRecord};
use tracing::{error, info};

/// A queue to buffer transactions
#[async_trait]
pub trait QueuePublisher: Send + Sync {
    async fn publish(&self, bundle: &EthSendBundle, sender: Address) -> Result<()>;
}

/// A queue to buffer transactions
pub struct KafkaQueuePublisher {
    producer: FutureProducer,
    topic: String,
}

impl KafkaQueuePublisher {
    pub fn new(producer: FutureProducer, topic: String) -> Self {
        Self { producer, topic }
    }

    pub async fn enqueue_bundle(
        &self,
        bundle: &EthSendBundle,
        sender: Address,
    ) -> Result<(), Error> {
        let key = sender.to_string();
        let payload = serde_json::to_vec(bundle)?;

        let record = FutureRecord::to(&self.topic).key(&key).payload(&payload);

        match self
            .producer
            .send(record, tokio::time::Duration::from_secs(5))
            .await
        {
            Ok((partition, offset)) => {
                info!(
                    sender = %sender,
                    partition = partition,
                    offset = offset,
                    topic = %self.topic,
                    "Successfully enqueued bundle"
                );
                Ok(())
            }
            Err((err, _)) => {
                error!(
                    sender = %sender,
                    error = %err,
                    topic = %self.topic,
                    "Failed to enqueue bundle"
                );
                Err(anyhow::anyhow!("Failed to enqueue bundle: {}", err))
            }
        }
    }
}

#[async_trait]
impl QueuePublisher for KafkaQueuePublisher {
    async fn publish(&self, bundle: &EthSendBundle, sender: Address) -> Result<()> {
        self.enqueue_bundle(bundle, sender).await
    }
}
