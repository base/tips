use account_abstraction_core::types::VersionedUserOperation;
use alloy_primitives::{Address, B256};
use anyhow::{Context, Result};
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::message::Message;
use rdkafka::{ClientConfig, message::BorrowedMessage};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{debug, error, info};

use crate::{InsertUserOpBundle, UserOpBundle};

pub struct UserOpKafkaConsumer {
    consumer: StreamConsumer,
    userops_step: InsertUserOpBundle,
    pending_ops: Arc<Mutex<HashMap<Address, Vec<VersionedUserOperation>>>>,
    batch_size: usize,
    batch_timeout_ms: u64,
}

impl UserOpKafkaConsumer {
    pub fn new(
        kafka_brokers: &str,
        kafka_properties_file: Option<&str>,
        topic: &str,
        group_id: &str,
        userops_step: InsertUserOpBundle,
        batch_size: usize,
        batch_timeout_ms: u64,
    ) -> Result<Self> {
        let mut config = ClientConfig::new();
        config
            .set("bootstrap.servers", kafka_brokers)
            .set("group.id", group_id)
            .set("enable.auto.commit", "true")
            .set("auto.commit.interval.ms", "5000")
            .set("session.timeout.ms", "6000")
            .set("enable.partition.eof", "false")
            .set("auto.offset.reset", "earliest");

        if let Some(properties_file) = kafka_properties_file
            && let Ok(properties) = std::fs::read_to_string(properties_file)
        {
            for line in properties.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some((key, value)) = line.split_once('=') {
                    config.set(key.trim(), value.trim());
                }
            }
        }

        let consumer: StreamConsumer =
            config.create().context("Failed to create Kafka consumer")?;

        consumer
            .subscribe(&[topic])
            .context("Failed to subscribe to topic")?;

        info!(
            message = "UserOp Kafka consumer initialized",
            topic = topic,
            group_id = group_id
        );

        Ok(Self {
            consumer,
            userops_step,
            pending_ops: Arc::new(Mutex::new(HashMap::new())),
            batch_size,
            batch_timeout_ms,
        })
    }

    pub async fn run(self: Arc<Self>) -> Result<()> {
        let flush_interval = Duration::from_millis(self.batch_timeout_ms);
        let mut last_flush = tokio::time::Instant::now();

        loop {
            match tokio::time::timeout(flush_interval, self.consumer.recv()).await {
                Ok(Ok(msg)) => {
                    if let Err(e) = self.handle_message(&msg).await {
                        error!(
                            message = "Failed to handle user operation message",
                            error = %e,
                            offset = msg.offset()
                        );
                    }

                    if last_flush.elapsed() >= flush_interval {
                        self.flush_pending_ops().await;
                        last_flush = tokio::time::Instant::now();
                    }
                }
                Ok(Err(e)) => {
                    error!(
                        message = "Error receiving from Kafka",
                        error = %e
                    );
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                Err(_) => {
                    self.flush_pending_ops().await;
                    last_flush = tokio::time::Instant::now();
                }
            }
        }
    }

    async fn handle_message(&self, msg: &BorrowedMessage<'_>) -> Result<()> {
        let payload = msg.payload().context("Message has no payload")?;

        let user_op: VersionedUserOperation =
            serde_json::from_slice(payload).context("Failed to deserialize user operation")?;

        let key = msg.key().context("Message has no key")?;
        let user_op_hash = B256::from_slice(key);

        let entry_point = match &user_op {
            VersionedUserOperation::UserOperation(_) => Address::ZERO,
            VersionedUserOperation::PackedUserOperation(_) => Address::ZERO,
        };

        debug!(
            message = "Received user operation",
            user_op_hash = %user_op_hash,
            entry_point = %entry_point
        );

        let mut pending = self.pending_ops.lock().await;
        pending
            .entry(entry_point)
            .or_insert_with(Vec::new)
            .push(user_op);

        let total_pending: usize = pending.values().map(|v| v.len()).sum();
        if total_pending >= self.batch_size {
            drop(pending);
            self.flush_pending_ops().await;
        }

        Ok(())
    }

    async fn flush_pending_ops(&self) {
        let mut pending = self.pending_ops.lock().await;
        if pending.is_empty() {
            return;
        }

        let entry_points: Vec<Address> = pending.keys().copied().collect();

        for entry_point in entry_points {
            if let Some(user_ops) = pending.remove(&entry_point) {
                if user_ops.is_empty() {
                    continue;
                }

                let bundle = self.create_bundle(entry_point, user_ops);

                info!(
                    message = "Flushing user operations bundle",
                    entry_point = %entry_point,
                    user_op_count = bundle.user_ops.len()
                );

                self.userops_step.add_bundle(bundle);
            }
        }
    }

    fn create_bundle(
        &self,
        entry_point: Address,
        user_ops: Vec<VersionedUserOperation>,
    ) -> UserOpBundle {
        use account_abstraction_core::types::UserOperationRequest;

        let beneficiary = self.userops_step.bundler_address;

        let user_op_requests: Vec<UserOperationRequest> = user_ops
            .into_iter()
            .map(|user_operation| UserOperationRequest {
                user_operation,
                entry_point,
                chain_id: 10,
            })
            .collect();

        let mut bundle = UserOpBundle::new(entry_point, beneficiary);
        for req in user_op_requests {
            bundle = bundle.with_user_op(req);
        }

        bundle
    }
}
