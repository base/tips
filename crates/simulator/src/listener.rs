use crate::types::{SimulationConfig, SimulationRequest};
use alloy_primitives::B256;
use anyhow::Result;
use async_trait::async_trait;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::Message;
use std::time::Duration;
use tips_audit::{create_kafka_consumer, types::MempoolEvent};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

#[async_trait]
pub trait MempoolEventListener: Send + Sync {
    /// Start listening for mempool events and send simulation requests
    async fn start(&mut self, sender: mpsc::Sender<SimulationRequest>) -> Result<()>;
    /// Stop the listener
    async fn stop(&mut self) -> Result<()>;
}

pub struct KafkaMempoolListener {
    consumer: StreamConsumer,
    topic: String,
    running: bool,
}

impl KafkaMempoolListener {
    pub fn new(config: &SimulationConfig) -> Result<Self> {
        let consumer = create_kafka_consumer(
            &config.kafka_brokers.join(","),
            &config.kafka_group_id,
        )?;
        
        Ok(Self {
            consumer,
            topic: config.kafka_topic.clone(),
            running: false,
        })
    }

    async fn process_event(
        &self,
        event: MempoolEvent,
        sender: &mpsc::Sender<SimulationRequest>,
        current_block: u64,
        current_block_hash: B256,
    ) -> Result<()> {
        match event {
            MempoolEvent::Created { bundle_id, bundle } | MempoolEvent::Updated { bundle_id, bundle } => {
                debug!(
                    bundle_id = %bundle_id,
                    num_transactions = bundle.txs.len(),
                    "Processing bundle for simulation"
                );

                let request = SimulationRequest {
                    bundle_id,
                    bundle,
                    block_number: current_block,
                    block_hash: current_block_hash,
                };

                if let Err(e) = sender.try_send(request) {
                    match e {
                        mpsc::error::TrySendError::Full(_) => {
                            warn!(
                                bundle_id = %bundle_id,
                                "Simulation queue is full, dropping request"
                            );
                        }
                        mpsc::error::TrySendError::Closed(_) => {
                            error!("Simulation queue receiver has been dropped");
                            return Err(anyhow::anyhow!("Simulation queue closed"));
                        }
                    }
                }
            }
            // We only care about Created and Updated events for simulation
            _ => {
                debug!(event = ?event, "Ignoring non-creation event");
            }
        }

        Ok(())
    }

    // TODO: This should be updated to get current block info from the state provider
    // For now, we'll use dummy values
    fn get_current_block_info(&self) -> (u64, B256) {
        (0, B256::ZERO)
    }
}

#[async_trait]
impl MempoolEventListener for KafkaMempoolListener {
    async fn start(&mut self, sender: mpsc::Sender<SimulationRequest>) -> Result<()> {
        info!(topic = %self.topic, "Starting mempool listener");
        
        self.consumer.subscribe(&[&self.topic])?;
        self.running = true;

        while self.running {
            match self.consumer.recv().await {
                Ok(message) => {
                    let payload = match message.payload() {
                        Some(payload) => payload,
                        None => {
                            warn!("Received message with empty payload");
                            continue;
                        }
                    };

                    match serde_json::from_slice::<MempoolEvent>(payload) {
                        Ok(event) => {
                            let (current_block, current_block_hash) = self.get_current_block_info();
                            
                            if let Err(e) = self.process_event(event, &sender, current_block, current_block_hash).await {
                                error!(error = %e, "Failed to process mempool event");
                            }
                        }
                        Err(e) => {
                            error!(
                                error = %e,
                                payload_size = payload.len(),
                                "Failed to deserialize mempool event"
                            );
                        }
                    }
                }
                Err(e) => {
                    error!(error = %e, "Error receiving message from Kafka");
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }

        info!("Mempool listener stopped");
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        info!("Stopping mempool listener");
        self.running = false;
        Ok(())
    }
}

/// Create a mempool listener using the provided configuration
pub fn create_mempool_listener(config: &SimulationConfig) -> Result<impl MempoolEventListener> {
    KafkaMempoolListener::new(config)
}
