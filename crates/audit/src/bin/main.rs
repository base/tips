use anyhow::Result;
use aws_sdk_s3::Client as S3Client;
use clap::Parser;
use rdkafka::consumer::Consumer;
use tips_audit::{
    create_kafka_consumer, KafkaMempoolArchiver, KafkaMempoolReader, S3MempoolEventReaderWriter,
};
use tracing::{info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser, Debug)]
#[command(name = "audit-archiver")]
#[command(about = "Audit archiver that reads from Kafka and writes to S3")]
struct Args {
    #[arg(long, env = "KAFKA_BROKERS", default_value = "localhost:9092")]
    kafka_brokers: String,

    #[arg(long, env = "KAFKA_TOPIC", default_value = "mempool-events")]
    kafka_topic: String,

    #[arg(long, env = "KAFKA_GROUP_ID", default_value = "audit-archiver")]
    kafka_group_id: String,

    #[arg(long, env = "S3_BUCKET")]
    s3_bucket: String,

    #[arg(long, env = "LOG_LEVEL", default_value = "info")]
    log_level: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let log_level = match args.log_level.to_lowercase().as_str() {
        "trace" => tracing::Level::TRACE,
        "debug" => tracing::Level::DEBUG,
        "info" => tracing::Level::INFO,
        "warn" => tracing::Level::WARN,
        "error" => tracing::Level::ERROR,
        _ => {
            warn!(
                "Invalid log level '{}', defaulting to 'info'",
                args.log_level
            );
            tracing::Level::INFO
        }
    };

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level.to_string())),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!(
        kafka_brokers = %args.kafka_brokers,
        kafka_topic = %args.kafka_topic,
        kafka_group_id = %args.kafka_group_id,
        s3_bucket = %args.s3_bucket,
        "Starting audit archiver"
    );

    let consumer = create_kafka_consumer(&args.kafka_brokers, &args.kafka_group_id)?;
    consumer.subscribe(&[&args.kafka_topic])?;

    let reader = KafkaMempoolReader::new(consumer, args.kafka_topic)?;

    let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let s3_client = S3Client::new(&config);
    let writer = S3MempoolEventReaderWriter::new(s3_client, args.s3_bucket);

    let mut archiver = KafkaMempoolArchiver::new(reader, writer);

    info!("Audit archiver initialized, starting main loop");

    archiver.run().await
}
