use crate::mempool::{self, Mempool};
use crate::types::WrappedUserOperation;
use rdkafka::{consumer::StreamConsumer, Message};
use serde::{Deserialize, Serialize};
use serde_json;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", content = "data")]
pub enum KafkaEvent {
    UserOpAdded {
        user_op: WrappedUserOperation,
    },
    UserOpIncluded {
        user_op: WrappedUserOperation,
    },
    UserOpDropped {
        user_op: WrappedUserOperation,
        reason: String,
    },
}

pub struct KafkaMempoolEngine {
    mempool: Arc<RwLock<mempool::MempoolImpl>>,
    kafka_producer: StreamConsumer,
}

impl KafkaMempoolEngine {
    pub fn new(
        mempool: Arc<RwLock<mempool::MempoolImpl>>,
        kafka_producer: StreamConsumer,
    ) -> Self {
        Self {
            mempool,
            kafka_producer,
        }
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        loop {
            let msg = self.kafka_producer.recv().await?.detach();
            let payload = msg
                .payload()
                .ok_or_else(|| anyhow::anyhow!("Kafka message missing payload"))?;
            let event: KafkaEvent = serde_json::from_slice(payload).map_err(|e| anyhow::anyhow!("Failed to parse Kafka event: {e}"))?;

            match event {
                KafkaEvent::UserOpAdded { user_op } => {
                    self.mempool.write().await.add_operation(&user_op)?;
                }
                KafkaEvent::UserOpIncluded { user_op } => {
                    self.mempool.write().await.remove_operation(&user_op.hash)?;
                }
                KafkaEvent::UserOpDropped { user_op, reason: _ } => {
                    self.mempool.write().await.remove_operation(&user_op.hash)?;
                }
            }
        }
    }
}

