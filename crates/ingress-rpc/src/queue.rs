use alloy_primitives::B256;
use anyhow::Result;
use backon::{ExponentialBuilder, Retryable};
use rdkafka::producer::{FutureProducer, FutureRecord};
use tips_core::{AcceptedBundle, UserOperationWithMetadata};
use tokio::time::Duration;
use tracing::{error, info};

/// Internal Kafka implementation - handles the low-level Kafka publishing with retry logic
struct KafkaPublisher {
    producer: FutureProducer,
    topic: String,
}

impl KafkaPublisher {
    fn new(producer: FutureProducer, topic: String) -> Self {
        Self { producer, topic }
    }

    /// Publish any message with a key to Kafka with automatic retry
    async fn publish(
        &self,
        key: &str,
        payload_bytes: Vec<u8>,
        entity_type: &str,
    ) -> Result<()> {
        let enqueue = || async {
            let record = FutureRecord::to(&self.topic)
                .key(key)
                .payload(&payload_bytes);

            match self.producer.send(record, Duration::from_secs(5)).await {
                Ok((partition, offset)) => {
                    info!(
                        key = %key,
                        partition = partition,
                        offset = offset,
                        topic = %self.topic,
                        entity_type = entity_type,
                        "Successfully published to Kafka"
                    );
                    Ok(())
                }
                Err((err, _)) => {
                    error!(
                        key = %key,
                        error = %err,
                        topic = %self.topic,
                        entity_type = entity_type,
                        "Failed to publish to Kafka"
                    );
                    Err(anyhow::anyhow!("Failed to publish: {err}"))
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
                info!("Retrying Kafka publish {:?} after {:?}", err, dur);
            })
            .await
    }
}

/// Publisher for bundle queues - handles bundle-specific publishing logic
pub struct BundleQueuePublisher {
    kafka: KafkaPublisher,
}

impl BundleQueuePublisher {
    pub fn new(producer: FutureProducer, topic: String) -> Self {
        Self {
            kafka: KafkaPublisher::new(producer, topic),
        }
    }

    pub async fn publish(&self, bundle: &AcceptedBundle, bundle_hash: &B256) -> Result<()> {
        let payload_bytes = serde_json::to_vec(bundle)?;
        self.kafka
            .publish(&bundle_hash.to_string(), payload_bytes, "bundle")
            .await
    }
}

/// Publisher for user operations - handles user operation-specific publishing logic
pub struct UserOperationQueuePublisher {
    kafka: KafkaPublisher,
}

impl UserOperationQueuePublisher {
    pub fn new(producer: FutureProducer, topic: String) -> Self {
        Self {
            kafka: KafkaPublisher::new(producer, topic),
        }
    }

    pub async fn publish(&self, user_op: &UserOperationWithMetadata) -> Result<()> {
        let key = user_op.user_op_hash.to_string();
        let payload_bytes = serde_json::to_vec(user_op)?;
        self.kafka
            .publish(&key, payload_bytes, "user_operation")
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rdkafka::config::ClientConfig;
    use tips_core::{
        AcceptedBundle, Bundle, BundleExtensions, test_utils::create_test_meter_bundle_response,
    };
    use tokio::time::{Duration, Instant};

    fn create_test_bundle() -> Bundle {
        Bundle::default()
    }

    #[tokio::test]
    async fn test_backoff_retry_logic() {
        // use an invalid broker address to trigger the backoff logic
        let producer = ClientConfig::new()
            .set("bootstrap.servers", "localhost:9999")
            .set("message.timeout.ms", "100")
            .create()
            .expect("Producer creation failed");

        let publisher = BundleQueuePublisher::new(producer, "tips-ingress-rpc".to_string());
        let bundle = create_test_bundle();
        let accepted_bundle = AcceptedBundle::new(
            bundle.try_into().unwrap(),
            create_test_meter_bundle_response(),
        );
        let bundle_hash = &accepted_bundle.bundle_hash();

        let start = Instant::now();
        let result = publisher.publish(&accepted_bundle, bundle_hash).await;
        let elapsed = start.elapsed();

        // the backoff tries at minimum 100ms, so verify we tried at least once
        assert!(result.is_err());
        assert!(elapsed >= Duration::from_millis(100));
    }
}
