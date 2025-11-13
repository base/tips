use anyhow::Result;
use async_trait::async_trait;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::{ClientConfig, Message};
use std::fmt::Debug;
use tokio::sync::mpsc;
use tracing::{error, trace};

use crate::pool::UserOpPoolItem;

#[async_trait]
pub trait UserOpSource {
    async fn run(&self) -> Result<()>;
}

pub struct KafkaUserOpSource {
    consumer: StreamConsumer,
    publisher: mpsc::UnboundedSender<UserOpPoolItem>,
}

impl Debug for KafkaUserOpSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "KafkaUserOpSource")
    }
}

impl KafkaUserOpSource {
    pub fn new(
        client_config: ClientConfig,
        topic: String,
        publisher: mpsc::UnboundedSender<UserOpPoolItem>,
    ) -> Result<Self> {
        let consumer: StreamConsumer = client_config.create()?;
        consumer.subscribe(&[topic.as_str()])?;
        Ok(Self {
            consumer,
            publisher,
        })
    }
}

#[async_trait]
impl UserOpSource for KafkaUserOpSource {
    async fn run(&self) -> Result<()> {
        loop {
            match self.consumer.recv().await {
                Ok(message) => {
                    let payload = match message.payload() {
                        Some(p) => p,
                        None => {
                            error!("Message has no payload");
                            continue;
                        }
                    };

                    let pool_item: UserOpPoolItem = match serde_json::from_slice(payload) {
                        Ok(item) => item,
                        Err(e) => {
                            error!(error = %e, "Failed to deserialize UserOp");
                            continue;
                        }
                    };

                    trace!(
                        sender = %pool_item.id.sender,
                        nonce = %pool_item.id.nonce,
                        entry_point = %pool_item.entry_point,
                        offset = message.offset(),
                        partition = message.partition(),
                        "Received UserOp from Kafka"
                    );

                    if let Err(e) = self.publisher.send(pool_item) {
                        error!(error = ?e, "Failed to publish UserOp to pool");
                    }
                }
                Err(e) => {
                    error!(error = %e, "Error receiving message from Kafka");
                }
            }
        }
    }
}

