use alloy_primitives::Address;
use alloy_rpc_types_mev::EthSendBundle;
use chrono::Utc;
use eyre::{Error, Result};
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::util::Timeout;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use tracing::{error, info};
use uuid::Uuid;

pub struct TransactionQueue {
    producer: FutureProducer,
    topic: String,
}

impl TransactionQueue {
    pub fn new(producer: FutureProducer, topic: String) -> Self {
        Self { producer, topic }
    }

    pub async fn enqueue_bundle(
        &self,
        bundle_id: Uuid,
        bundle: &EthSendBundle,
        sender: Address,
    ) -> Result<(), Error> {
        let partition_key = self.compute_partition_key(sender);
        let payload = self.serialize_bundle_message(bundle_id, bundle)?;

        let record = FutureRecord::to(&self.topic)
            .key(&partition_key)
            .payload(&payload);

        match self
            .producer
            .send(record, Timeout::After(std::time::Duration::from_secs(5)))
            .await
        {
            Ok((partition, offset)) => {
                info!(
                    message = "Bundle enqueued successfully",
                    bundle_id = %bundle_id,
                    sender = %sender,
                    partition = partition,
                    offset = offset,
                    topic = %self.topic
                );
                Ok(())
            }
            Err((kafka_error, _)) => {
                error!(
                    message = "Failed to enqueue bundle",
                    bundle_id = %bundle_id,
                    sender = %sender,
                    error = %kafka_error,
                    topic = %self.topic
                );
                Err(kafka_error.into())
            }
        }
    }

    fn compute_partition_key(&self, sender: Address) -> String {
        let mut hasher = DefaultHasher::new();
        sender.hash(&mut hasher);
        format!("sender_{:x}", hasher.finish())
    }

    fn serialize_bundle_message(
        &self,
        bundle_id: Uuid,
        bundle: &EthSendBundle,
    ) -> Result<String, Error> {
        let message = BundleQueueMessage {
            bundle_id,
            bundle: bundle.clone(),
            timestamp: Utc::now().timestamp(),
        };

        serde_json::to_string(&message).map_err(Error::from)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct BundleQueueMessage {
    bundle_id: Uuid,
    bundle: EthSendBundle,
    timestamp: i64,
}
