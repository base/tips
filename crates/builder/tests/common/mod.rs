use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use std::time::Duration;

pub struct TestHarness {
    pub kafka_producer: FutureProducer,
    pub kafka_bootstrap_servers: String,
}

impl TestHarness {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let kafka_bootstrap_servers = std::env::var("KAFKA_BOOTSTRAP_SERVERS")
            .unwrap_or_else(|_| "localhost:9092".to_string());

        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", &kafka_bootstrap_servers)
            .set("message.timeout.ms", "5000")
            .create()?;

        Ok(Self {
            kafka_producer: producer,
            kafka_bootstrap_servers,
        })
    }
}
