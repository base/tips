use eyre::eyre;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::{
    consumer::{Consumer, StreamConsumer},
    message::Message,
    ClientConfig,
};
use std::time::Duration;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::{kafka, kafka::Kafka, minio::MinIO};
use uuid::Uuid;

struct TestHarness {
    pub s3_client: aws_sdk_s3::Client,
    pub bucket_name: String,
    pub kafka_producer: FutureProducer,
    pub kafka_consumer: StreamConsumer,
    _minio_container: testcontainers::ContainerAsync<MinIO>,
    _kafka_container: testcontainers::ContainerAsync<Kafka>,
}

impl TestHarness {
    pub async fn new() -> eyre::Result<Self> {
        let minio_container = MinIO::default().start().await?;
        let s3_port = minio_container.get_host_port_ipv4(9000).await?;
        let s3_endpoint = format!("http://127.0.0.1:{}", s3_port);

        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region("us-east-1")
            .endpoint_url(&s3_endpoint)
            .credentials_provider(aws_sdk_s3::config::Credentials::new(
                "minioadmin",
                "minioadmin",
                None,
                None,
                "test",
            ))
            .load()
            .await;

        let s3_client = aws_sdk_s3::Client::new(&config);
        let bucket_name = format!("test-bucket-{}", Uuid::new_v4());

        s3_client
            .create_bucket()
            .bucket(&bucket_name)
            .send()
            .await?;

        let kafka_container = Kafka::default().start().await?;
        let bootstrap_servers = format!(
            "127.0.0.1:{}",
            kafka_container
                .get_host_port_ipv4(kafka::KAFKA_PORT)
                .await?
        );

        let kafka_producer = ClientConfig::new()
            .set("bootstrap.servers", &bootstrap_servers)
            .set("message.timeout.ms", "5000")
            .create::<FutureProducer>()
            .expect("Failed to create Kafka FutureProducer");

        let kafka_consumer = ClientConfig::new()
            .set("group.id", "testcontainer-rs")
            .set("bootstrap.servers", &bootstrap_servers)
            .set("session.timeout.ms", "6000")
            .set("enable.auto.commit", "false")
            .set("auto.offset.reset", "earliest")
            .create::<StreamConsumer>()
            .expect("Failed to create Kafka StreamConsumer");

        Ok(TestHarness {
            s3_client,
            bucket_name,
            kafka_producer,
            kafka_consumer,
            _minio_container: minio_container,
            _kafka_container: kafka_container,
        })
    }
}

#[tokio::test]
async fn example_s3() -> eyre::Result<()> {
    let harness = TestHarness::new().await?;

    let test_key = "test-key";
    let test_content = "test content";

    harness
        .s3_client
        .put_object()
        .bucket(&harness.bucket_name)
        .key(test_key)
        .body(aws_sdk_s3::primitives::ByteStream::from(
            test_content.as_bytes().to_vec(),
        ))
        .send()
        .await?;

    let response = harness
        .s3_client
        .get_object()
        .bucket(&harness.bucket_name)
        .key(test_key)
        .send()
        .await?;

    let body = response.body.collect().await?;
    let retrieved_content = String::from_utf8(body.into_bytes().to_vec())?;
    assert_eq!(retrieved_content, test_content);

    Ok(())
}

#[tokio::test]
async fn example_kafka() -> Result<(), Box<dyn std::error::Error + 'static>> {
    let harness = TestHarness::new().await?;

    let topic = "test-topic";
    let number_of_messages_to_produce = 5_usize;
    let expected: Vec<String> = (0..number_of_messages_to_produce)
        .map(|i| format!("Message {i}"))
        .collect();

    for (i, message) in expected.iter().enumerate() {
        harness
            .kafka_producer
            .send(
                FutureRecord::to(topic)
                    .payload(message)
                    .key(&format!("Key {i}")),
                Duration::from_secs(0),
            )
            .await
            .unwrap();
    }

    harness
        .kafka_consumer
        .subscribe(&[topic])
        .expect("Failed to subscribe to a topic");

    use futures::stream::StreamExt;
    use std::iter::Iterator;

    let mut message_stream = harness.kafka_consumer.stream();
    for (i, produced) in expected.iter().enumerate() {
        let message = message_stream
            .next()
            .await
            .ok_or(eyre!("no messages received"))??;

        let message = message
            .detach()
            .payload_view::<str>()
            .ok_or(eyre!("error deserializing message payload"))??
            .to_string();

        assert_eq!(*produced, message);
    }

    Ok(())
}
