use crate::types::{BundleId, MempoolEvent};
use alloy_primitives::TxHash;
use anyhow::Result;
use async_trait::async_trait;
use rdkafka::{
    config::ClientConfig,
    producer::{FutureProducer, FutureRecord},
};
use serde_json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info};

#[async_trait]
pub trait MempoolEventPublisher: Send + Sync {
    async fn publish(&self, event: MempoolEvent) -> Result<()>;
    async fn publish_all(&self, events: Vec<MempoolEvent>) -> Result<()>;
}

#[derive(Clone)]
pub struct KafkaMempoolEventPublisher {
    producer: FutureProducer,
    topic: String,
}

impl KafkaMempoolEventPublisher {
    pub fn new(brokers: &str, topic: String) -> Result<Self> {
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("message.timeout.ms", "5000")
            .set("batch.size", "16384")
            .set("linger.ms", "10")
            .set("compression.type", "snappy")
            .set("acks", "all")
            .set("retries", "10")
            .create()?;

        Ok(Self { producer, topic })
    }

    async fn send_event(&self, event: &MempoolEvent) -> Result<()> {
        let bundle_id = event.bundle_id();
        let key = bundle_id.to_string();
        let payload = serde_json::to_vec(event)?;

        let record = FutureRecord::to(&self.topic).key(&key).payload(&payload);

        match self
            .producer
            .send(record, tokio::time::Duration::from_secs(5))
            .await
        {
            Ok(_) => {
                debug!(
                    bundle_id = %bundle_id,
                    topic = %self.topic,
                    payload_size = payload.len(),
                    "Successfully published event"
                );
                Ok(())
            }
            Err((err, _)) => {
                error!(
                    bundle_id = %bundle_id,
                    topic = %self.topic,
                    error = %err,
                    "Failed to publish event"
                );
                Err(anyhow::anyhow!("Failed to publish event: {}", err))
            }
        }
    }
}

#[async_trait]
impl MempoolEventPublisher for KafkaMempoolEventPublisher {
    async fn publish(&self, event: MempoolEvent) -> Result<()> {
        self.send_event(&event).await
    }

    async fn publish_all(&self, events: Vec<MempoolEvent>) -> Result<()> {
        let mut handles = Vec::new();

        for event in events {
            let self_clone = self.clone();
            let handle = tokio::spawn(async move { self_clone.send_event(&event).await });
            handles.push(handle);
        }

        let total_events = handles.len();
        let mut errors = Vec::new();
        for handle in handles {
            if let Err(e) = handle.await? {
                errors.push(e);
            }
        }

        if !errors.is_empty() {
            error!(
                error_count = errors.len(),
                total_events = total_events,
                "Failed to publish some events"
            );
            return Err(anyhow::anyhow!(
                "Failed to publish {} events: {:?}",
                errors.len(),
                errors
            ));
        }

        info!(
            event_count = total_events,
            "Successfully published all events"
        );
        Ok(())
    }
}

#[derive(Default, Clone)]
pub struct InMemoryMempoolEventPublisher {
    events_by_bundle: Arc<Mutex<HashMap<BundleId, Vec<MempoolEvent>>>>,
    events_by_txn_hash: Arc<Mutex<HashMap<TxHash, Vec<MempoolEvent>>>>,
    all_events: Arc<Mutex<Vec<MempoolEvent>>>,
}

impl InMemoryMempoolEventPublisher {
    pub fn new() -> Self {
        Self {
            events_by_bundle: Arc::new(Mutex::new(HashMap::new())),
            events_by_txn_hash: Arc::new(Mutex::new(HashMap::new())),
            all_events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn get_events(&self) -> Vec<MempoolEvent> {
        self.all_events.lock().await.clone()
    }

    pub async fn get_events_by_bundle(&self, bundle_id: BundleId) -> Vec<MempoolEvent> {
        self.events_by_bundle
            .lock()
            .await
            .get(&bundle_id)
            .cloned()
            .unwrap_or_default()
    }

    pub async fn get_events_by_txn_hash(&self, txn_hash: TxHash) -> Vec<MempoolEvent> {
        self.events_by_txn_hash
            .lock()
            .await
            .get(&txn_hash)
            .cloned()
            .unwrap_or_default()
    }

    pub async fn clear(&self) {
        self.events_by_bundle.lock().await.clear();
        self.events_by_txn_hash.lock().await.clear();
        self.all_events.lock().await.clear();
    }

    pub async fn count(&self) -> usize {
        self.all_events.lock().await.len()
    }

    async fn store_event(&self, event: MempoolEvent) {
        let bundle_id = event.bundle_id();
        let transaction_ids = event.transaction_ids();

        self.events_by_bundle
            .lock()
            .await
            .entry(bundle_id)
            .or_insert_with(Vec::new)
            .push(event.clone());

        for txn_id in transaction_ids {
            self.events_by_txn_hash
                .lock()
                .await
                .entry(txn_id.hash)
                .or_insert_with(Vec::new)
                .push(event.clone());
        }

        self.all_events.lock().await.push(event);
    }
}

#[async_trait]
impl MempoolEventPublisher for InMemoryMempoolEventPublisher {
    async fn publish(&self, event: MempoolEvent) -> Result<()> {
        self.store_event(event).await;
        Ok(())
    }

    async fn publish_all(&self, events: Vec<MempoolEvent>) -> Result<()> {
        for event in events {
            self.store_event(event).await;
        }
        Ok(())
    }
}
