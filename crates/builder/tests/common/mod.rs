use rdkafka::config::ClientConfig;
use rdkafka::producer::FutureProducer;

pub struct TestHarness {
    pub kafka_producer: FutureProducer,
    pub kafka_bootstrap_servers: String,
}

impl TestHarness {
    pub async fn new() -> anyhow::Result<Self> {
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
