use crate::types::SimulationResult;
use eyre::Result;
use async_trait::async_trait;
use rdkafka::producer::FutureProducer;
use serde_json;
use std::sync::Arc;
use tips_audit::{MempoolEventPublisher, KafkaMempoolEventPublisher};
use tips_datastore::PostgresDatastore;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

#[async_trait]
pub trait SimulationResultPublisher: Send + Sync {
    /// Store a simulation result
    async fn publish_result(&self, result: SimulationResult) -> Result<()>;
    
    /// Get simulation results for a bundle
    async fn get_results_for_bundle(&self, bundle_id: Uuid) -> Result<Vec<SimulationResult>>;
    
    /// Get a specific simulation result by ID
    async fn get_result_by_id(&self, result_id: Uuid) -> Result<Option<SimulationResult>>;
}

#[derive(Clone)]
pub struct DatabaseResultPublisher {
    datastore: Arc<PostgresDatastore>,
    kafka_publisher: Option<Arc<dyn MempoolEventPublisher>>,
}

impl DatabaseResultPublisher {
    pub fn new(
        datastore: Arc<PostgresDatastore>,
        kafka_publisher: Option<Arc<dyn MempoolEventPublisher>>,
    ) -> Self {
        Self {
            datastore,
            kafka_publisher,
        }
    }

    pub fn with_kafka(
        datastore: Arc<PostgresDatastore>,
        producer: FutureProducer,
        topic: String,
    ) -> Self {
        let publisher = Arc::new(KafkaMempoolEventPublisher::new(producer, topic));
        Self::new(datastore, Some(publisher))
    }

    /// Convert SimulationResult to database format
    fn result_to_db_format(&self, result: &SimulationResult) -> Result<DatabaseSimulation> {
        Ok(DatabaseSimulation {
            id: result.id,
            bundle_id: result.bundle_id,
            block_number: result.block_number as i64,
            block_hash: format!("0x{}", hex::encode(result.block_hash.as_slice())),
            success: result.success,
            gas_used: result.gas_used.map(|g| g as i64),
            execution_time_us: result.execution_time_us as i64,
            state_diff: serde_json::to_value(&result.state_diff)?,
            error_reason: result.error_reason.clone(),
            created_at: result.created_at,
            updated_at: result.created_at, // For new records, created_at == updated_at
        })
    }

    /// Store result in database
    async fn store_in_database(&self, result: &SimulationResult) -> Result<()> {
        let _db_result = self.result_to_db_format(result)?;
        
        info!(
            simulation_id = %result.id,
            bundle_id = %result.bundle_id,
            success = result.success,
            gas_used = ?result.gas_used,
            "Storing simulation result in database"
        );

        // TODO: This would need to be implemented with proper sqlx queries
        // For now, we'll use the datastore interface if it has simulation methods
        // Otherwise, we need to add simulation-specific methods to the datastore
        
        // Placeholder implementation - in a real scenario, we'd add methods to PostgresDatastore
        // like: datastore.store_simulation_result(result).await?;
        
        debug!(
            simulation_id = %result.id,
            "Database storage placeholder - would insert simulation result here"
        );
        
        Ok(())
    }

    /// Publish result to Kafka if configured
    async fn publish_to_kafka(&self, result: &SimulationResult) -> Result<()> {
        if let Some(ref _publisher) = self.kafka_publisher {
            // Create a custom event type for simulation results
            // For now, we'll create a mock event - in the future, we might want to extend
            // the MempoolEvent enum to include simulation results
            
            debug!(
                simulation_id = %result.id,
                bundle_id = %result.bundle_id,
                success = result.success,
                "Publishing simulation result to Kafka"
            );
            
            // TODO: Implement proper simulation result event
            // For now, this is commented out as we'd need to extend the MempoolEvent enum
            
            // let event = MempoolEvent::SimulationComplete {
            //     bundle_id: result.bundle_id,
            //     simulation_id: result.id,
            //     success: result.success,
            //     gas_used: result.gas_used,
            //     execution_time_us: result.execution_time_us,
            // };
            
            // publisher.publish(event).await?;
        }
        
        Ok(())
    }
}

#[async_trait]
impl SimulationResultPublisher for DatabaseResultPublisher {
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

    async fn get_results_for_bundle(&self, bundle_id: Uuid) -> Result<Vec<SimulationResult>> {
        info!(bundle_id = %bundle_id, "Fetching simulation results for bundle");
        
        // TODO: Implement actual database query
        // For now, return empty vec as placeholder
        
        debug!(bundle_id = %bundle_id, "No simulation results found");
        Ok(vec![])
    }

    async fn get_result_by_id(&self, result_id: Uuid) -> Result<Option<SimulationResult>> {
        info!(simulation_id = %result_id, "Fetching simulation result by ID");
        
        // TODO: Implement actual database query
        // For now, return None as placeholder
        
        debug!(simulation_id = %result_id, "Simulation result not found");
        Ok(None)
    }
}

/// Database representation of a simulation result
/// This matches the expected database schema
#[derive(Debug, Clone)]
struct DatabaseSimulation {
    id: Uuid,
    bundle_id: Uuid,
    block_number: i64,
    block_hash: String,
    success: bool,
    gas_used: Option<i64>,
    execution_time_us: i64,
    state_diff: serde_json::Value,
    error_reason: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

/// Create a result publisher with database storage
pub fn create_database_publisher(
    datastore: Arc<PostgresDatastore>,
) -> DatabaseResultPublisher {
    DatabaseResultPublisher::new(datastore, None)
}

/// Create a result publisher with database storage and Kafka publishing
pub fn create_database_kafka_publisher(
    datastore: Arc<PostgresDatastore>,
    producer: FutureProducer,
    topic: String,
) -> impl SimulationResultPublisher {
    DatabaseResultPublisher::with_kafka(datastore, producer, topic)
}

// We'll need to add hex as a dependency for block hash formatting
// For now, using a simple placeholder
