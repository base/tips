use alloy_rpc_types_mev::EthSendBundle;
use anyhow::Result;
use async_trait::async_trait;
use backon::{ExponentialBuilder, Retryable};
use rdkafka::{
    consumer::{Consumer, StreamConsumer},
    message::Message,
};
use tips_datastore::BundleDatastore;
use tokio::time::Duration;
use tracing::{debug, error, info};
use uuid::Uuid;

#[async_trait]
pub trait Writer: Send + Sync {
    async fn insert_bundle(&self) -> Result<Uuid>;
}

pub struct DatastoreWriter<Store> {
    queue_consumer: StreamConsumer,
    datastore: Store,
}

impl<Store> DatastoreWriter<Store> {
    pub fn new(
        queue_consumer: StreamConsumer,
        queue_topic: String,
        datastore: Store,
    ) -> Result<Self> {
        queue_consumer.subscribe(&[queue_topic.as_str()])?;
        Ok(Self {
            queue_consumer,
            datastore,
        })
    }
}

#[async_trait]
impl<Store> Writer for DatastoreWriter<Store>
where
    Store: BundleDatastore + Send + Sync + 'static,
{
    async fn insert_bundle(&self) -> Result<Uuid> {
        match self.queue_consumer.recv().await {
            Ok(message) => {
                let payload = message
                    .payload()
                    .ok_or_else(|| anyhow::anyhow!("Message has no payload"))?;
                let bundle: EthSendBundle = serde_json::from_slice(payload)?;
                debug!(
                    bundle = ?bundle,
                    offset = message.offset(),
                    partition = message.partition(),
                    "Received bundle from queue"
                );

                let insert = || async {
                    self.datastore
                        .insert_bundle(bundle.clone())
                        .await
                        .map_err(|e| anyhow::anyhow!("Failed to insert bundle: {e}"))
                };

                insert
                    .retry(
                        &ExponentialBuilder::default()
                            .with_min_delay(Duration::from_millis(100))
                            .with_max_delay(Duration::from_secs(5))
                            .with_max_times(3),
                    )
                    .notify(|err: &anyhow::Error, dur: Duration| {
                        info!("Retrying to insert bundle {:?} after {:?}", err, dur);
                    })
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to insert bundle after retries: {e}"))
            }
            Err(e) => {
                error!(error = %e, "Error receiving message from Kafka");
                Err(e.into())
            }
        }
    }
}
