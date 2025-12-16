use crate::mempool::{self, Mempool};
use crate::types::WrappedUserOperation;
use async_trait::async_trait;
use rdkafka::{Message, consumer::StreamConsumer, message::OwnedMessage};
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

#[async_trait]
pub trait KafkaConsumer {
    async fn recv_msg(&self) -> anyhow::Result<OwnedMessage>;
}

#[async_trait]
impl KafkaConsumer for StreamConsumer {
    async fn recv_msg(&self) -> anyhow::Result<OwnedMessage> {
        Ok(self.recv().await?.detach())
    }
}

pub struct KafkaMempoolEngine {
    mempool: Arc<RwLock<mempool::MempoolImpl>>,
    kafka_consumer: Arc<dyn KafkaConsumer>,
}

impl KafkaMempoolEngine {
    pub fn new(
        mempool: Arc<RwLock<mempool::MempoolImpl>>,
        kafka_consumer: Arc<dyn KafkaConsumer>,
    ) -> Self {
        Self {
            mempool,
            kafka_consumer,
        }
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        loop {
            self.process_next().await?;
        }
    }

    /// Process a single Kafka message (useful for tests and controlled loops)
    pub async fn process_next(&self) -> anyhow::Result<()> {
        let msg = self.kafka_consumer.recv_msg().await?;
        let payload = msg
            .payload()
            .ok_or_else(|| anyhow::anyhow!("Kafka message missing payload"))?;
        let event: KafkaEvent = serde_json::from_slice(payload)
            .map_err(|e| anyhow::anyhow!("Failed to parse Kafka event: {e}"))?;

        self.handle_event(event).await
    }

    async fn handle_event(&self, event: KafkaEvent) -> anyhow::Result<()> {
        println!("Handling Kafka event: {:?}", event);
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
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mempool::PoolConfig;
    use crate::types::VersionedUserOperation;
    use alloy_primitives::{Address, FixedBytes, Uint};
    use alloy_rpc_types::erc4337;
    use rdkafka::Timestamp;
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

    #[tokio::test]
    async fn handle_add_operation() {
        let mempool = Arc::new(RwLock::new(mempool::MempoolImpl::new(PoolConfig::new(0))));

        let op_hash = [1u8; 32];
        let wrapped = make_wrapped_op(1_000, op_hash);

        let add_event = KafkaEvent::UserOpAdded {
            user_op: wrapped.clone(),
        };
        let mock_consumer = Arc::new(MockConsumer::new(vec![OwnedMessage::new(
            Some(serde_json::to_vec(&add_event).unwrap()),
            None,
            "topic".to_string(),
            Timestamp::NotAvailable,
            0,
            0,
            None,
        )]));

        let engine = KafkaMempoolEngine::new(mempool.clone(), mock_consumer);

        // Process add then remove deterministically
        engine.process_next().await.unwrap();
        let items: Vec<_> = mempool.read().await.get_top_operations(10).collect();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].hash, FixedBytes::from(op_hash));
    }

    #[tokio::test]
    async fn remove_opperation_should_remove_from_mempool() {
        let mempool = Arc::new(RwLock::new(mempool::MempoolImpl::new(PoolConfig::new(0))));
        let op_hash = [1u8; 32];
        let wrapped = make_wrapped_op(1_000, op_hash);
        let add_mempool = KafkaEvent::UserOpAdded {
            user_op: wrapped.clone(),
        };
        let remove_mempool = KafkaEvent::UserOpDropped {
            user_op: wrapped.clone(),
            reason: "test".to_string(),
        };
        let mock_consumer = Arc::new(MockConsumer::new(vec![
            OwnedMessage::new(
                Some(serde_json::to_vec(&add_mempool).unwrap()),
                None,
                "topic".to_string(),
                Timestamp::NotAvailable,
                0,
                0,
                None,
            ),
            OwnedMessage::new(
                Some(serde_json::to_vec(&remove_mempool).unwrap()),
                None,
                "topic".to_string(),
                Timestamp::NotAvailable,
                0,
                0,
                None,
            ),
        ]));

        let engine = KafkaMempoolEngine::new(mempool.clone(), mock_consumer);
        engine.process_next().await.unwrap();
        let items: Vec<_> = mempool.read().await.get_top_operations(10).collect();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].hash, FixedBytes::from(op_hash));
        engine.process_next().await.unwrap();
        let items: Vec<_> = mempool.read().await.get_top_operations(10).collect();
        assert_eq!(items.len(), 0);
    }
    struct MockConsumer {
        msgs: Mutex<Vec<OwnedMessage>>,
    }

    impl MockConsumer {
        fn new(msgs: Vec<OwnedMessage>) -> Self {
            Self {
                msgs: Mutex::new(msgs),
            }
        }
    }

    #[async_trait]
    impl KafkaConsumer for MockConsumer {
        async fn recv_msg(&self) -> anyhow::Result<OwnedMessage> {
            let mut guard = self.msgs.lock().await;
            if guard.is_empty() {
                Err(anyhow::anyhow!("no more messages"))
            } else {
                Ok(guard.remove(0))
            }
        }
    }
}
