use crate::engine::SimulationEngine;
use crate::publisher::SimulationPublisher;
use crate::types::SimulationRequest;
use crate::worker_pool::{SimulationTask, SimulationWorkerPool};
use eyre::Result;
use rdkafka::{
    config::ClientConfig,
    consumer::{Consumer, StreamConsumer},
    message::Message,
};
use tokio::sync::mpsc;
use tracing::{debug, error, info};
use std::time::Duration;
use std::sync::Arc;
use alloy_primitives::B256;
use reth_provider::{BlockNumReader, HeaderProvider};
use reth_node_api::FullNodeComponents;
use tips_audit::types::MempoolEvent;

/// Configuration for mempool event listening
#[derive(Debug, Clone)]
pub struct MempoolListenerConfig {
    /// Kafka brokers for consuming mempool events
    pub kafka_brokers: Vec<String>,
    /// Kafka topic to consume mempool events from
    pub kafka_topic: String,
    /// Kafka consumer group ID
    pub kafka_group_id: String,
    /// PostgreSQL database connection URL
    pub database_url: String,
}


/// Mempool event listener that processes events and queues simulations
pub struct MempoolEventListener<Node, E, P> 
where
    Node: FullNodeComponents,
    E: SimulationEngine,
    P: SimulationPublisher,
{
    /// State provider factory for getting current block info
    provider: Arc<Node::Provider>,
    /// Kafka consumer for mempool events
    consumer: StreamConsumer,
    /// Kafka topic name
    topic: String,
    /// Shared simulation worker pool
    worker_pool: Arc<SimulationWorkerPool<E, P, Node::Provider>>,
}

impl<Node, E, P> MempoolEventListener<Node, E, P> 
where
    Node: FullNodeComponents,
    E: SimulationEngine + Clone + 'static,
    P: SimulationPublisher + Clone + 'static,
{
    /// Create a new mempool event listener
    pub fn new(
        provider: Arc<Node::Provider>,
        config: MempoolListenerConfig,
        worker_pool: Arc<SimulationWorkerPool<E, P, Node::Provider>>,
    ) -> Result<Self> {
        let consumer: StreamConsumer = ClientConfig::new()
            .set("group.id", &config.kafka_group_id)
            .set("bootstrap.servers", config.kafka_brokers.join(","))
            .set("enable.partition.eof", "false")
            .set("session.timeout.ms", "6000")
            .set("enable.auto.commit", "false")
            .set("auto.offset.reset", "earliest")
            .set("fetch.wait.max.ms", "100")
            .set("fetch.min.bytes", "1")
            .create()
            .map_err(|e| eyre::eyre!("Failed to create Kafka consumer: {}", e))?;

        consumer.subscribe(&[&config.kafka_topic])
            .map_err(|e| eyre::eyre!("Failed to subscribe to topic {}: {}", config.kafka_topic, e))?;

        Ok(Self {
            provider,
            consumer,
            topic: config.kafka_topic,
            worker_pool,
        })
    }

    /// Run the mempool event listener
    pub async fn run(self) -> Result<()> 
    where
        E: 'static,
        P: 'static,
    {
        info!(
            topic = %self.topic,
            "Starting mempool event listener"
        );
        
        // Create channel for simulation requests
        let (sender, mut receiver) = mpsc::channel::<SimulationRequest>(1000);
        
        // Start Kafka listener in a separate task
        let consumer = self.consumer;
        let provider = Arc::clone(&self.provider);
        let topic = self.topic.clone();
        let listener_handle: tokio::task::JoinHandle<Result<()>> = tokio::spawn(async move {
            info!(topic = %topic, "Starting Kafka mempool event listener");

            loop {
                match consumer.recv().await {
                    Ok(message) => {
                        let payload = message
                            .payload()
                            .ok_or_else(|| eyre::eyre!("Message has no payload"))?;

                        // Parse the mempool event
                        let event: MempoolEvent = serde_json::from_slice(payload)
                            .map_err(|e| eyre::eyre!("Failed to parse mempool event: {}", e))?;

                        debug!(
                            bundle_id = %event.bundle_id(),
                            offset = message.offset(),
                            partition = message.partition(),
                            "Received mempool event"
                        );

                        // Convert mempool events that contain bundles into simulation requests
                        match event {
                            MempoolEvent::Created { bundle_id, bundle } |
                            MempoolEvent::Updated { bundle_id, bundle } => {
                                let (block_number, block_hash) = match provider.best_block_number() {
                                    Ok(num) => {
                                        let hash = provider.sealed_header(num)
                                            .unwrap_or_default()
                                            .map(|h| h.hash())
                                            .unwrap_or_default();
                                        (num, hash)
                                    }
                                    Err(_) => (0, B256::ZERO),
                                };
                                
                                let simulation_request = SimulationRequest {
                                    bundle_id,
                                    bundle,
                                    block_number,
                                    block_hash,
                                };

                                if let Err(e) = sender.send(simulation_request).await {
                                    error!(
                                        error = %e,
                                        bundle_id = %bundle_id,
                                        "Failed to send simulation request"
                                    );
                                }
                            }
                            _ => {
                                // Other events (cancelled, included, dropped) don't need simulation
                                debug!(
                                    bundle_id = %event.bundle_id(),
                                    "Skipping non-simulatable event"
                                );
                            }
                        }

                        // Commit the message
                        if let Err(e) = consumer.commit_message(&message, rdkafka::consumer::CommitMode::Async) {
                            error!(error = %e, "Failed to commit Kafka message");
                        }
                    }
                    Err(e) => {
                        error!(error = %e, "Error receiving message from Kafka");
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                }
            }
        });
        
        // Process simulation requests using the shared worker pool
        let worker_pool = Arc::clone(&self.worker_pool);
        let processing_handle = tokio::spawn(async move {
            while let Some(request) = receiver.recv().await {
                let bundle_id = request.bundle_id;
                debug!(
                    bundle_id = %bundle_id,
                    block_number = request.block_number,
                    "Queuing bundle simulation for mempool event"
                );
                
                // Create simulation task
                let task = SimulationTask {
                    request,
                };
                
                // Queue simulation using shared worker pool
                if let Err(e) = worker_pool.queue_simulation(task).await {
                    error!(
                        error = %e,
                        bundle_id = %bundle_id,
                        "Failed to queue simulation task"
                    );
                }
            }
        });
        
        // Wait for both tasks to complete
        let (listener_result, _processing_result) = tokio::try_join!(listener_handle, processing_handle)
            .map_err(|e| eyre::eyre!("Task join error: {}", e))?;
        
        if let Err(e) = listener_result {
            error!(error = %e, "Mempool listener task failed");
            return Err(e);
        }
        
        info!("Mempool event listener completed");
        Ok(())
    }
}
