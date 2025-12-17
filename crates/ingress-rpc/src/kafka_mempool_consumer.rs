use std::sync::Arc;
use std::time::Duration;

use account_abstraction_core::{kafka_mempool_engine::KafkaMempoolEngine, mempool::PoolConfig};
use backon::{ExponentialBuilder, Retryable};
use rdkafka::{
    ClientConfig,
    consumer::{Consumer, StreamConsumer},
};
use tips_core::kafka::load_kafka_config_from_file;
use tracing::warn;

/// Build a Kafka consumer for the user operation topic.
/// Ensures the consumer group id is set per deployment and subscribes to the topic.
fn create_user_operation_consumer(
    properties_file: &str,
    topic: &str,
    consumer_group_id: &str,
) -> anyhow::Result<StreamConsumer> {
    let mut client_config = ClientConfig::from_iter(load_kafka_config_from_file(properties_file)?);

    // Allow deployments to control group id even if the properties file omits it.
    client_config.set("group.id", consumer_group_id);
    // Rely on Kafka for at-least-once; we keep auto commit enabled unless overridden in the file.
    client_config.set("enable.auto.commit", "true");

    let consumer: StreamConsumer = client_config.create()?;
    consumer.subscribe(&[topic])?;

    Ok(consumer)
}

/// Factory function that creates a fully configured KafkaMempoolEngine.
/// Handles consumer creation, engine instantiation, and Arc wrapping.
pub fn create_mempool_engine(
    properties_file: &str,
    topic: &str,
    consumer_group_id: &str,
    pool_config: Option<PoolConfig>,
) -> anyhow::Result<Arc<KafkaMempoolEngine>> {
    let consumer = create_user_operation_consumer(properties_file, topic, consumer_group_id)?;
    Ok(Arc::new(KafkaMempoolEngine::with_kafka_consumer(
        Arc::new(consumer),
        pool_config,
    )))
}

/// Process a single Kafka message with exponential backoff retries.
pub async fn process_next_with_backoff(engine: &KafkaMempoolEngine) -> anyhow::Result<()> {
    let process = || async { engine.process_next().await };

    process
        .retry(
            &ExponentialBuilder::default()
                .with_min_delay(Duration::from_millis(100))
                .with_max_delay(Duration::from_secs(5))
                .with_max_times(5),
        )
        .notify(|err: &anyhow::Error, dur: Duration| {
            warn!(
                error = %err,
                retry_in_ms = dur.as_millis(),
                "Retrying Kafka mempool engine step"
            );
        })
        .await
}

/// Run the mempool engine forever, applying backoff on individual message failures.
pub async fn run_mempool_engine(engine: Arc<KafkaMempoolEngine>) {
    loop {
        if let Err(err) = process_next_with_backoff(&engine).await {
            // We log and continue to avoid stalling the consumer; repeated failures
            // will still observe backoff inside `process_next_with_backoff`.
            warn!(error = %err, "Kafka mempool engine exhausted retries, continuing");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use account_abstraction_core::{
        kafka_mempool_engine::{KafkaConsumer, KafkaEvent},
        mempool::PoolConfig,
        types::{VersionedUserOperation, WrappedUserOperation},
    };
    use alloy_primitives::{Address, FixedBytes, Uint};
    use alloy_rpc_types::erc4337;
    use rdkafka::{Message, Timestamp, message::OwnedMessage};
    use tokio::sync::Mutex;

    fn make_wrapped_op(max_fee: u128, hash: [u8; 32]) -> WrappedUserOperation {
        let op = VersionedUserOperation::UserOperation(erc4337::UserOperation {
            sender: Address::ZERO,
            nonce: Uint::from(0u64),
            init_code: Default::default(),
            call_data: Default::default(),
            call_gas_limit: Uint::from(100_000u64),
            verification_gas_limit: Uint::from(100_000u64),
            pre_verification_gas: Uint::from(21_000u64),
            max_fee_per_gas: Uint::from(max_fee),
            max_priority_fee_per_gas: Uint::from(max_fee),
            paymaster_and_data: Default::default(),
            signature: Default::default(),
        });

        WrappedUserOperation {
            operation: op,
            hash: FixedBytes::from(hash),
        }
    }

    struct MockConsumer {
        msgs: Mutex<Vec<anyhow::Result<OwnedMessage>>>,
    }

    impl MockConsumer {
        fn new(msgs: Vec<anyhow::Result<OwnedMessage>>) -> Self {
            Self {
                msgs: Mutex::new(msgs),
            }
        }
    }

    #[async_trait::async_trait]
    impl KafkaConsumer for MockConsumer {
        async fn recv_msg(&self) -> anyhow::Result<OwnedMessage> {
            let mut guard = self.msgs.lock().await;
            if guard.is_empty() {
                Err(anyhow::anyhow!("no more messages"))
            } else {
                guard.remove(0)
            }
        }
    }

    #[tokio::test]
    async fn process_next_with_backoff_recovers_after_error() {
        let add_event = KafkaEvent::UserOpAdded {
            user_op: make_wrapped_op(1_000, [1u8; 32]),
        };

        let payload = serde_json::to_vec(&add_event).unwrap();
        let good_msg = OwnedMessage::new(
            Some(payload),
            None,
            "topic".to_string(),
            Timestamp::NotAvailable,
            0,
            0,
            None,
        );

        // First call fails, second succeeds.
        let consumer = Arc::new(MockConsumer::new(vec![
            Err(anyhow::anyhow!("transient error")),
            Ok(good_msg),
        ]));

        let engine = KafkaMempoolEngine::with_kafka_consumer(consumer, Some(PoolConfig::default()));

        // Should succeed after retrying the transient failure.
        let result = process_next_with_backoff(&engine).await;
        assert!(result.is_ok());

        // The mempool should contain the added op.
        let items: Vec<_> = engine
            .get_mempool()
            .read()
            .await
            .get_top_operations(10)
            .collect();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].hash, FixedBytes::from([1u8; 32]));
    }
}
