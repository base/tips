use crate::types::SimulationResult;
use async_trait::async_trait;
use eyre::Result;
use rdkafka::producer::FutureProducer;
use std::collections::HashMap;
use std::sync::Arc;
use tips_audit::{KafkaMempoolEventPublisher, MempoolEventPublisher};
use tips_datastore::{postgres::StateDiff, BundleDatastore, PostgresDatastore};
use tracing::{debug, error, info, warn};

#[async_trait]
pub trait SimulationPublisher: Send + Sync {
    /// Store a simulation result
    async fn publish_result(&self, result: SimulationResult) -> Result<()>;
}

#[derive(Clone)]
pub struct TipsSimulationPublisher {
    datastore: Arc<PostgresDatastore>,
    kafka_publisher: Option<Arc<dyn MempoolEventPublisher>>,
}

impl TipsSimulationPublisher {
    pub fn new(datastore: Arc<PostgresDatastore>, producer: FutureProducer, topic: String) -> Self {
        let kafka_publisher = Arc::new(KafkaMempoolEventPublisher::new(producer, topic));
        Self {
            datastore,
            kafka_publisher: Some(kafka_publisher),
        }
    }

    /// Store result in database
    async fn store_in_database(&self, result: &SimulationResult) -> Result<()> {
        info!(
            simulation_id = %result.id,
            bundle_id = %result.bundle_id,
            success = result.success,
            gas_used = ?result.gas_used,
            "Storing simulation result in database"
        );

        // Convert state diff from alloy format to datastore format
        let state_diff = self.convert_state_diff(&result.state_diff)?;

        // Store the simulation using the datastore interface
        let simulation_id = self
            .datastore
            .insert_simulation(
                result.bundle_id,
                result.block_number,
                format!("0x{}", hex::encode(result.block_hash.as_slice())),
                result.execution_time_us as u64,
                result.gas_used.unwrap_or(0),
                state_diff,
            )
            .await
            .map_err(|e| eyre::eyre!("Failed to insert simulation: {}", e))?;

        debug!(
            simulation_id = %simulation_id,
            bundle_id = %result.bundle_id,
            "Successfully stored simulation result in database"
        );

        Ok(())
    }

    /// Convert state diff from simulator format to datastore format
    fn convert_state_diff(
        &self,
        state_diff: &HashMap<
            alloy_primitives::Address,
            HashMap<alloy_primitives::U256, alloy_primitives::U256>,
        >,
    ) -> Result<StateDiff> {
        // StateDiff expects HashMap<Address, HashMap<B256, U256>>
        // where StorageKey is B256 and StorageValue is U256
        let mut converted = HashMap::new();

        for (address, storage) in state_diff {
            let mut storage_map = HashMap::new();
            for (key, value) in storage {
                // Convert U256 key to B256 for storage key
                let key_bytes = key.to_be_bytes::<32>();
                let storage_key = alloy_primitives::B256::from(key_bytes);
                storage_map.insert(storage_key, *value);
            }
            converted.insert(*address, storage_map);
        }

        Ok(converted)
    }

    /// Publish result to Kafka if configured
    async fn publish_to_kafka(&self, result: &SimulationResult) -> Result<()> {
        if let Some(ref publisher) = self.kafka_publisher {
            debug!(
                simulation_id = %result.id,
                bundle_id = %result.bundle_id,
                success = result.success,
                "Publishing simulation result to Kafka"
            );

            let event = tips_audit::types::MempoolEvent::Simulated {
                bundle_id: result.bundle_id,
                simulation_id: result.id,
                block_number: result.block_number,
                success: result.success,
                gas_used: result.gas_used,
                execution_time_us: result.execution_time_us,
                error_reason: result.error_reason.clone(),
            };

            publisher
                .publish(event)
                .await
                .map_err(|e| eyre::eyre!("Failed to publish simulation event: {}", e))?;

            debug!(
                simulation_id = %result.id,
                bundle_id = %result.bundle_id,
                "Successfully published simulation result to Kafka"
            );
        }

        Ok(())
    }
}

#[async_trait]
impl SimulationPublisher for TipsSimulationPublisher {
    async fn publish_result(&self, result: SimulationResult) -> Result<()> {
        info!(
            simulation_id = %result.id,
            bundle_id = %result.bundle_id,
            success = result.success,
            "Publishing simulation result"
        );

        // Store in database
        if let Err(e) = self.store_in_database(&result).await {
            error!(
                error = %e,
                simulation_id = %result.id,
                "Failed to store simulation result in database"
            );
            return Err(e);
        }

        // Publish to Kafka if configured
        if let Err(e) = self.publish_to_kafka(&result).await {
            warn!(
                error = %e,
                simulation_id = %result.id,
                "Failed to publish simulation result to Kafka"
            );
            // Don't fail the entire operation if Kafka publish fails
        }

        debug!(
            simulation_id = %result.id,
            bundle_id = %result.bundle_id,
            "Successfully published simulation result"
        );

        Ok(())
    }
}
