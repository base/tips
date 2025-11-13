//! Kafka consumer for UserOperations

use anyhow::Result;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::{ClientConfig, Message};
use tokio::sync::mpsc;
use tracing::{error, info};

use crate::types::UserOperationMessage;

pub struct UserOperationConsumer {
    consumer: StreamConsumer,
    sender: mpsc::UnboundedSender<UserOperationMessage>,
}

impl UserOperationConsumer {
    pub fn new(
        client_config: ClientConfig,
        topic: String,
        sender: mpsc::UnboundedSender<UserOperationMessage>,
    ) -> Result<Self> {
        let consumer: StreamConsumer = client_config.create()?;
        consumer.subscribe(&[&topic])?;
        
        info!(
            topic = %topic,
            "UserOperation consumer subscribed to topic"
        );

        Ok(Self { consumer, sender })
    }

    pub async fn run(&self) -> Result<()> {
        info!("Starting UserOperation consumer loop");

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

                    let user_op_message: UserOperationMessage = match serde_json::from_slice(payload) {
                        Ok(msg) => msg,
                        Err(e) => {
                            error!(
                                error = %e,
                                offset = message.offset(),
                                partition = message.partition(),
                                "Failed to deserialize UserOperationMessage"
                            );
                            continue;
                        }
                    };

                    info!(
                        sender = %user_op_message.user_operation.sender(),
                        entry_point = %user_op_message.entry_point,
                        hash = %user_op_message.hash,
                        version = user_op_message.user_operation.version(),
                        offset = message.offset(),
                        partition = message.partition(),
                        "Received UserOperation from Kafka"
                    );

                    if let Err(e) = self.sender.send(user_op_message) {
                        error!(error = ?e, "Failed to send UserOperation to processor");
                    }
                }
                Err(e) => {
                    error!(error = %e, "Error receiving message from Kafka");
                }
            }
        }
    }
}

