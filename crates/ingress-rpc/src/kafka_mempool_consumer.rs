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
    let consumer: StreamConsumer =
        create_user_operation_consumer(properties_file, topic, consumer_group_id)?;
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
