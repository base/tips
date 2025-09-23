use alloy_rpc_types_mev::EthSendBundle;
use anyhow::Result;
use backon::{ExponentialBuilder, Retryable};
use clap::Parser;
use rdkafka::{
    config::ClientConfig,
    consumer::{Consumer, StreamConsumer},
    message::Message,
};
use tips_datastore::{BundleDatastore, postgres::PostgresDatastore};
use tokio::time::Duration;
use tracing::{debug, error, info};
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "tips-writer")]
#[command(about = "TIPS Writer Service - Consumes bundles from Kafka and writes to datastore")]
struct Args {
    #[arg(long, env = "TIPS_WRITER_DATABASE_URL")]
    database_url: String,

    #[arg(long, env = "TIPS_WRITER_KAFKA_BROKERS")]
    kafka_brokers: String,

    #[arg(long, env = "TIPS_WRITER_KAFKA_TOPIC", default_value = "tips-ingress")]
    kafka_topic: String,

    #[arg(long, env = "TIPS_WRITER_KAFKA_GROUP_ID")]
    kafka_group_id: String,

    #[arg(long, env = "TIPS_WRITER_LOG_LEVEL", default_value = "info")]
    log_level: String,
}

/// IngressWriter consumes bundles sent from the Ingress service and writes them to the datastore
pub struct IngressWriter<Store> {
    queue_consumer: StreamConsumer,
    datastore: Store,
}

impl<Store> IngressWriter<Store>
where
    Store: BundleDatastore + Send + Sync + 'static,
{
    pub fn new(
        queue_consumer: StreamConsumer,
        queue_topic: String,
        datastore: Store,
    ) -> Result<Self> {
        queue_consumer.subscribe(&[queue_topic.as_str()])?;
        Ok(Self {
            queue_consumer,
            datastore,
        })
    }

    async fn insert_bundle(&self) -> Result<Uuid> {
        match self.queue_consumer.recv().await {
            Ok(message) => {
                let payload = message
                    .payload()
                    .ok_or_else(|| anyhow::anyhow!("Message has no payload"))?;
                let bundle: EthSendBundle = serde_json::from_slice(payload)?;
                debug!(
                    bundle = ?bundle,
                    offset = message.offset(),
                    partition = message.partition(),
                    "Received bundle from queue"
                );

                let insert = || async {
                    self.datastore
                        .insert_bundle(bundle.clone())
                        .await
                        .map_err(|e| anyhow::anyhow!("Failed to insert bundle: {e}"))
                };

                insert
                    .retry(
                        &ExponentialBuilder::default()
                            .with_min_delay(Duration::from_millis(100))
                            .with_max_delay(Duration::from_secs(5))
                            .with_max_times(3),
                    )
                    .notify(|err: &anyhow::Error, dur: Duration| {
                        info!("Retrying to insert bundle {:?} after {:?}", err, dur);
                    })
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to insert bundle after retries: {e}"))
            }
            Err(e) => {
                error!(error = %e, "Error receiving message from Kafka");
                Err(e.into())
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let args = Args::parse();

    tracing_subscriber::fmt()
        .with_env_filter(&args.log_level)
        .init();

    let mut config = ClientConfig::new();
    config
        .set("group.id", &args.kafka_group_id)
        .set("bootstrap.servers", &args.kafka_brokers)
        .set("auto.offset.reset", "earliest")
        .set("enable.partition.eof", "false")
        .set("session.timeout.ms", "6000")
        .set("enable.auto.commit", "true");

    let consumer = config.create()?;
    let datastore = PostgresDatastore::connect(args.database_url).await?;
    let writer = IngressWriter::new(consumer, args.kafka_topic.clone(), datastore)?;

    info!(
        "Ingress Writer service started, consuming from topic: {}",
        args.kafka_topic
    );
    loop {
        match writer.insert_bundle().await {
            Ok(bundle_id) => {
                info!(bundle_id = %bundle_id, "Successfully inserted bundle");
            }
            Err(e) => {
                error!(error = %e, "Failed to process bundle");
            }
        }
    }
}
