use account_abstraction_core::types::VersionedUserOperation;
use alloy_primitives::B256;
use anyhow::Result;
use async_trait::async_trait;
use backon::{ExponentialBuilder, Retryable};
use rdkafka::producer::{FutureProducer, FutureRecord};
use tips_core::AcceptedBundle;
use tokio::time::Duration;
use tracing::{error, info};

#[async_trait]
pub trait MessageQueue: Send + Sync {
    async fn publish_raw(&self, topic: &str, key: &str, payload: &[u8]) -> Result<()>;
}

pub struct KafkaMessageQueue {
    producer: FutureProducer,
}

impl KafkaMessageQueue {
    pub fn new(producer: FutureProducer) -> Self {
        Self { producer }
    }
}

#[async_trait]
impl MessageQueue for KafkaMessageQueue {
    async fn publish_raw(&self, topic: &str, key: &str, payload: &[u8]) -> Result<()> {
        let enqueue = || async {
            let record = FutureRecord::to(topic).key(key).payload(payload);

            match self.producer.send(record, Duration::from_secs(5)).await {
                Ok((partition, offset)) => {
                    info!(
                        key = %key,
                        partition = partition,
                        offset = offset,
                        topic = %topic,
                        "Successfully enqueued message"
                    );
                    Ok(())
                }
                Err((err, _)) => {
                    error!(
                        key = key,
                        error = %err,
                        topic = topic,
                        "Failed to enqueue message"
                    );
                    Err(anyhow::anyhow!("Failed to enqueue bundle: {err}"))
                }
            }
        };

        enqueue
            .retry(
                &ExponentialBuilder::default()
                    .with_min_delay(Duration::from_millis(100))
                    .with_max_delay(Duration::from_secs(5))
                    .with_max_times(3),
            )
            .notify(|err: &anyhow::Error, dur: Duration| {
                info!("retrying to enqueue message {:?} after {:?}", err, dur);
            })
            .await
    }
}

pub struct UserOpQueuePublisher<Q: MessageQueue> {
    queue: std::sync::Arc<Q>,
    topic: String,
}

impl<Q: MessageQueue> UserOpQueuePublisher<Q> {
    pub fn new(queue: std::sync::Arc<Q>, topic: String) -> Self {
        Self { queue, topic }
    }

    pub async fn publish(&self, user_op: &VersionedUserOperation, hash: &B256) -> Result<()> {
        let key = hash.to_string();
        let payload = serde_json::to_vec(&user_op)?;
        self.queue.publish_raw(&self.topic, &key, &payload).await
    }
}

pub struct BundleQueuePublisher<Q: MessageQueue> {
    queue: std::sync::Arc<Q>,
    topic: String,
}

impl<Q: MessageQueue> BundleQueuePublisher<Q> {
    pub fn new(queue: std::sync::Arc<Q>, topic: String) -> Self {
        Self { queue, topic }
    }

    pub async fn publish(&self, bundle: &AcceptedBundle, hash: &B256) -> Result<()> {
        let key = hash.to_string();
        let payload = serde_json::to_vec(bundle)?;
        self.queue.publish_raw(&self.topic, &key, &payload).await
    }
}
