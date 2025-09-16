use alloy_primitives::{Address, TxHash, U256};
use aws_config::BehaviorVersion;
use aws_credential_types::Credentials;
use aws_sdk_s3::Client as S3Client;
use bytes::Bytes;
use rdkafka::{
    admin::{AdminClient, AdminOptions, NewTopic, TopicReplication},
    consumer::{Consumer, StreamConsumer},
    message::Message,
    ClientConfig as KafkaClientConfig,
};
use std::str::FromStr;
use std::time::Duration;
use testcontainers::{runners::AsyncRunner, ContainerAsync};
use testcontainers_modules::{kafka::Kafka, minio::MinIO};
use tips_audit::{
    CanonicalTransactionEvent, InMemoryMempoolEventPublisher, KafkaMempoolArchiver,
    KafkaMempoolEventPublisher, MempoolEvent, MempoolEventPublisher, MempoolEventWriter,
    S3MempoolEventWriter, Transaction, TransactionId, TransactionMetadata,
};
use tokio::time::{sleep, timeout};
use uuid::Uuid;

struct TestHarness {
    _kafka_container: ContainerAsync<Kafka>,
    _minio_container: ContainerAsync<MinIO>,
    kafka_bootstrap_servers: String,
    s3_client: S3Client,
    bucket_name: String,
}

impl TestHarness {
    async fn setup() -> eyre::Result<Self> {
        let kafka_container = Kafka::default().start().await?;
        let kafka_bootstrap_servers = format!(
            "127.0.0.1:{}",
            kafka_container.get_host_port_ipv4(9092).await?
        );

        let minio_container = MinIO::default().start().await?;
        let s3_port = minio_container.get_host_port_ipv4(9000).await?;
        let s3_endpoint = format!("http://127.0.0.1:{}", s3_port);

        let credentials = Credentials::new("minioadmin", "minioadmin", None, None, "test");

        let config = aws_config::defaults(BehaviorVersion::latest())
            .endpoint_url(&s3_endpoint)
            .region("us-east-1")
            .credentials_provider(credentials)
            .load()
            .await;

        let s3_config = aws_sdk_s3::config::Builder::from(&config)
            .force_path_style(true)
            .build();
        let s3_client = S3Client::from_conf(s3_config);
        let bucket_name = format!("test-bucket-{}", Uuid::new_v4());

        s3_client
            .create_bucket()
            .bucket(&bucket_name)
            .send()
            .await?;

        Ok(Self {
            _kafka_container: kafka_container,
            _minio_container: minio_container,
            kafka_bootstrap_servers,
            s3_client,
            bucket_name,
        })
    }

    async fn create_kafka_topic(&self, topic: &str) -> eyre::Result<()> {
        let admin_client: AdminClient<_> = KafkaClientConfig::new()
            .set("bootstrap.servers", &self.kafka_bootstrap_servers)
            .create()?;

        let new_topic = NewTopic::new(topic, 1, TopicReplication::Fixed(1));
        let opts = AdminOptions::new().operation_timeout(Some(Duration::from_secs(30)));

        admin_client
            .create_topics(&[new_topic], &opts)
            .await
            .map_err(|e| eyre::eyre!("Failed to create topic: {}", e))?;

        Ok(())
    }

    fn create_s3_writer(&self) -> S3MempoolEventWriter {
        S3MempoolEventWriter::with_client(self.s3_client.clone(), self.bucket_name.clone())
    }

    fn create_kafka_publisher(&self, topic: &str) -> eyre::Result<KafkaMempoolEventPublisher> {
        KafkaMempoolEventPublisher::new(&self.kafka_bootstrap_servers, topic.to_string())
            .map_err(|e| eyre::eyre!("Failed to create kafka publisher: {}", e))
    }

    async fn setup_topic_and_publisher(&self, topic: &str) -> eyre::Result<KafkaMempoolEventPublisher> {
        self.create_kafka_topic(topic).await?;
        let publisher = self.create_kafka_publisher(topic)
            .map_err(|e| eyre::eyre!("Failed to create publisher: {}", e))?;
        Ok(publisher)
    }
}

fn create_test_transaction_id() -> TransactionId {
    TransactionId {
        sender: Address::from_str("0x742d35Cc6634C0532925a3b8D9a5F1D0E8C4F4F7").unwrap(),
        nonce: U256::from(1),
        hash: TxHash::from_str(
            "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
        )
        .unwrap(),
    }
}

fn create_test_transaction() -> Transaction {
    Transaction {
        id: create_test_transaction_id(),
        data: Bytes::from(vec![1, 2, 3, 4, 5]),
    }
}

fn create_test_event() -> MempoolEvent {
    let bundle_id = Uuid::new_v4();
    let transaction = create_test_transaction();

    MempoolEvent::ReceivedBundle {
        bundle_id,
        transactions: vec![transaction],
    }
}

#[tokio::test]
async fn test_mempool_event_serialization() {
    let event = create_test_event();

    let serialized = serde_json::to_string(&event).expect("Failed to serialize event");
    let deserialized: MempoolEvent =
        serde_json::from_str(&serialized).expect("Failed to deserialize event");

    assert_eq!(event.bundle_id(), deserialized.bundle_id());
    assert_eq!(event.transaction_ids(), deserialized.transaction_ids());
}

#[tokio::test]
async fn test_different_event_types() {
    let bundle_id = Uuid::new_v4();
    let tx_id = create_test_transaction_id();

    let events = vec![
        MempoolEvent::ReceivedBundle {
            bundle_id,
            transactions: vec![create_test_transaction()],
        },
        MempoolEvent::CancelledBundle {
            bundle_id,
            transaction_ids: vec![tx_id.clone()],
        },
        MempoolEvent::BuilderMined {
            bundle_id,
            transaction_ids: vec![tx_id.clone()],
            block_number: 12345,
            flashblock_index: 1,
        },
        MempoolEvent::FlashblockInclusion {
            bundle_id,
            transaction_ids: vec![tx_id.clone()],
            block_number: 12345,
            flashblock_index: 1,
        },
        MempoolEvent::BlockInclusion {
            bundle_id,
            transaction_ids: vec![tx_id],
            block_hash: TxHash::from_str(
                "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
            )
            .unwrap(),
            block_number: 12345,
            flashblock_index: 1,
        },
    ];

    for event in events {
        let serialized = serde_json::to_string(&event).expect("Failed to serialize event");
        let deserialized: MempoolEvent =
            serde_json::from_str(&serialized).expect("Failed to deserialize event");

        assert_eq!(event.bundle_id(), deserialized.bundle_id());
    }
}

#[tokio::test]
async fn test_kafka_publisher_integration() -> eyre::Result<()> {
    let harness = TestHarness::setup().await?;
    let topic = "test-mempool-events";

    let publisher = harness.setup_topic_and_publisher(topic).await?;

    let event = create_test_event();
    let bundle_id = event.bundle_id();

    publisher
        .publish(event.clone())
        .await
        .map_err(|e| eyre::eyre!("Failed to publish event: {}", e))?;

    let consumer: StreamConsumer = KafkaClientConfig::new()
        .set("group.id", "test-consumer")
        .set("bootstrap.servers", &harness.kafka_bootstrap_servers)
        .set("auto.offset.reset", "earliest")
        .set("enable.auto.commit", "false")
        .create()?;

    consumer.subscribe(&[topic])?;

    let message = timeout(Duration::from_secs(10), consumer.recv())
        .await
        .map_err(|_| eyre::eyre!("Timeout waiting for message"))?
        .map_err(|e| eyre::eyre!("Failed to receive message: {}", e))?;

    let payload = message.payload().ok_or_else(|| eyre::eyre!("No payload"))?;
    let received_event: MempoolEvent = serde_json::from_slice(payload)?;

    assert_eq!(received_event.bundle_id(), bundle_id);
    assert_eq!(received_event.transaction_ids(), event.transaction_ids());

    Ok(())
}

#[tokio::test]
async fn test_kafka_publisher_batch_integration() -> eyre::Result<()> {
    let harness = TestHarness::setup().await?;
    let topic = "test-batch-events";

    let publisher = harness.setup_topic_and_publisher(topic).await?;

    let events = vec![
        create_test_event(),
        create_test_event(),
        create_test_event(),
    ];
    let bundle_ids: Vec<_> = events.iter().map(|e| e.bundle_id()).collect();

    publisher
        .publish_all(events)
        .await
        .map_err(|e| eyre::eyre!("Failed to publish batch events: {}", e))?;

    let consumer: StreamConsumer = KafkaClientConfig::new()
        .set("group.id", "test-batch-consumer")
        .set("bootstrap.servers", &harness.kafka_bootstrap_servers)
        .set("auto.offset.reset", "earliest")
        .set("enable.auto.commit", "false")
        .create()?;

    consumer.subscribe(&[topic])?;

    let mut received_bundle_ids = Vec::new();
    for _ in 0..3 {
        let message = timeout(Duration::from_secs(10), consumer.recv())
            .await
            .map_err(|_| eyre::eyre!("Timeout waiting for message"))?
            .map_err(|e| eyre::eyre!("Failed to receive message: {}", e))?;

        let payload = message.payload().ok_or_else(|| eyre::eyre!("No payload"))?;
        let received_event: MempoolEvent = serde_json::from_slice(payload)?;
        received_bundle_ids.push(received_event.bundle_id());
    }

    for bundle_id in bundle_ids {
        assert!(received_bundle_ids.contains(&bundle_id));
    }

    Ok(())
}

#[tokio::test]
async fn test_s3_archiver_integration() -> eyre::Result<()> {
    let harness = TestHarness::setup().await?;

    let response = harness.s3_client.list_buckets().send().await?;

    let bucket_exists = response
        .buckets()
        .iter()
        .any(|b| b.name() == Some(&harness.bucket_name));

    assert!(bucket_exists, "Test bucket should exist");

    let test_key = "test-object";
    let test_content = b"test content";

    harness.s3_client
        .put_object()
        .bucket(&harness.bucket_name)
        .key(test_key)
        .body(test_content.to_vec().into())
        .send()
        .await?;

    let response = harness.s3_client
        .get_object()
        .bucket(&harness.bucket_name)
        .key(test_key)
        .send()
        .await?;

    let body = response.body.collect().await?;
    let retrieved_content = body.to_vec();

    assert_eq!(retrieved_content, test_content);

    Ok(())
}

#[tokio::test]
async fn test_full_kafka_s3_pipeline() -> eyre::Result<()> {
    let harness = TestHarness::setup().await?;
    let topic = "test-pipeline";

    let publisher = harness.setup_topic_and_publisher(topic).await?;

    let event = create_test_event();
    let bundle_id = event.bundle_id();

    publisher
        .publish(event.clone())
        .await
        .map_err(|e| eyre::eyre!("Failed to publish event: {}", e))?;

    sleep(Duration::from_millis(500)).await;

    let mut archiver = KafkaMempoolArchiver::new(
        &harness.kafka_bootstrap_servers,
        topic.to_string(),
        "test-archiver-group".to_string(),
        harness.bucket_name.clone(),
    )
    .await
    .map_err(|e| eyre::eyre!("Failed to create archiver: {}", e))?;

    tokio::spawn(async move {
        let _ = archiver.run().await;
    });

    sleep(Duration::from_secs(3)).await;

    let bundle_key = format!("bundles/{}", bundle_id);
    let tx_id = event.transaction_ids()[0].clone();
    let tx_hash_key = format!("transactions/by_hash/{:?}", tx_id.hash);
    let canonical_key = format!("transactions/canonical/{:?}/{}", tx_id.sender, tx_id.nonce);

    let bundle_exists = timeout(Duration::from_secs(10), async {
        loop {
            let exists = harness
                .s3_client
                .head_object()
                .bucket(&harness.bucket_name)
                .key(&bundle_key)
                .send()
                .await
                .is_ok();
            if exists {
                break;
            }
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await;

    assert!(
        bundle_exists.is_ok(),
        "Bundle index should be created in S3"
    );

    let tx_hash_exists = harness
        .s3_client
        .head_object()
        .bucket(&harness.bucket_name)
        .key(&tx_hash_key)
        .send()
        .await
        .is_ok();
    assert!(
        tx_hash_exists,
        "Transaction by hash index should be created in S3"
    );

    let canonical_exists = harness
        .s3_client
        .head_object()
        .bucket(&harness.bucket_name)
        .key(&canonical_key)
        .send()
        .await
        .is_ok();
    assert!(
        canonical_exists,
        "Canonical transaction log should be created in S3"
    );

    Ok(())
}

#[tokio::test]
async fn test_s3_mempool_event_writer() -> eyre::Result<()> {
    let harness = TestHarness::setup().await?;
    let writer = harness.create_s3_writer();

    let event = create_test_event();
    let bundle_id = event.bundle_id();
    let tx_id = event.transaction_ids()[0].clone();

    writer
        .write_event(event.clone())
        .await
        .map_err(|e| eyre::eyre!("Failed to write event: {}", e))?;

    let bundle_key = format!("bundles/{}", bundle_id);
    let tx_hash_key = format!("transactions/by_hash/{:?}", tx_id.hash);
    let canonical_key = format!("transactions/canonical/{:?}/{}", tx_id.sender, tx_id.nonce);

    let bundle_exists = harness.s3_client
        .head_object()
        .bucket(&harness.bucket_name)
        .key(&bundle_key)
        .send()
        .await
        .is_ok();
    assert!(bundle_exists, "Bundle index should be created in S3");

    let tx_hash_exists = harness.s3_client
        .head_object()
        .bucket(&harness.bucket_name)
        .key(&tx_hash_key)
        .send()
        .await
        .is_ok();
    assert!(
        tx_hash_exists,
        "Transaction by hash index should be created in S3"
    );

    let canonical_exists = harness.s3_client
        .head_object()
        .bucket(&harness.bucket_name)
        .key(&canonical_key)
        .send()
        .await
        .is_ok();
    assert!(
        canonical_exists,
        "Canonical transaction log should be created in S3"
    );

    let bundle_response = harness.s3_client
        .get_object()
        .bucket(&harness.bucket_name)
        .key(&bundle_key)
        .send()
        .await?;
    let bundle_body = bundle_response.body.collect().await?;
    let bundle_content: Vec<String> = serde_json::from_slice(&bundle_body.to_vec())?;
    assert_eq!(bundle_content.len(), 1);
    assert_eq!(bundle_content[0], format!("{:?}", tx_id.hash));

    let tx_response = harness.s3_client
        .get_object()
        .bucket(&harness.bucket_name)
        .key(&tx_hash_key)
        .send()
        .await?;
    let tx_body = tx_response.body.collect().await?;
    let tx_metadata: TransactionMetadata = serde_json::from_slice(&tx_body.to_vec())?;
    assert_eq!(tx_metadata.bundle_ids, vec![bundle_id]);
    assert_eq!(tx_metadata.sender, format!("{:?}", tx_id.sender));
    assert_eq!(tx_metadata.nonce, tx_id.nonce.to_string());

    let canonical_response = harness.s3_client
        .get_object()
        .bucket(&harness.bucket_name)
        .key(&canonical_key)
        .send()
        .await?;
    let canonical_body = canonical_response.body.collect().await?;
    let canonical_event: CanonicalTransactionEvent =
        serde_json::from_slice(&canonical_body.to_vec())?;
    assert_eq!(canonical_event.event_log.len(), 1);
    assert_eq!(canonical_event.event_log[0].bundle_id(), bundle_id);

    Ok(())
}

#[tokio::test]
async fn test_s3_mempool_event_writer_idempotent() -> eyre::Result<()> {
    let harness = TestHarness::setup().await?;
    let writer = harness.create_s3_writer();

    let event = create_test_event();
    let bundle_id = event.bundle_id();

    writer
        .write_event(event.clone())
        .await
        .map_err(|e| eyre::eyre!("Failed to write event first time: {}", e))?;

    writer
        .write_event(event.clone())
        .await
        .map_err(|e| eyre::eyre!("Failed to write event second time: {}", e))?;

    let bundle_key = format!("bundles/{}", bundle_id);
    let bundle_response = harness.s3_client
        .get_object()
        .bucket(&harness.bucket_name)
        .key(&bundle_key)
        .send()
        .await?;
    let bundle_body = bundle_response.body.collect().await?;
    let bundle_content: Vec<String> = serde_json::from_slice(&bundle_body.to_vec())?;

    assert_eq!(
        bundle_content.len(),
        1,
        "Writing the same event twice should be idempotent"
    );

    Ok(())
}

#[tokio::test]
async fn test_s3_mempool_event_writer_multiple_events() -> eyre::Result<()> {
    let harness = TestHarness::setup().await?;
    let writer = harness.create_s3_writer();

    let bundle_id = Uuid::new_v4();
    let tx1 = create_test_transaction();
    let mut tx2 = create_test_transaction();
    tx2.id.nonce = U256::from(2);

    let event1 = MempoolEvent::ReceivedBundle {
        bundle_id,
        transactions: vec![tx1.clone(), tx2.clone()],
    };

    let event2 = MempoolEvent::CancelledBundle {
        bundle_id,
        transaction_ids: vec![tx1.id.clone()],
    };

    writer
        .write_event(event1.clone())
        .await
        .map_err(|e| eyre::eyre!("Failed to write first event: {:?}", e))?;

    writer
        .write_event(event2.clone())
        .await
        .map_err(|e| eyre::eyre!("Failed to write second event: {}", e))?;

    let bundle_key = format!("bundles/{}", bundle_id);
    let bundle_response = harness.s3_client
        .get_object()
        .bucket(&harness.bucket_name)
        .key(&bundle_key)
        .send()
        .await?;
    let bundle_body = bundle_response.body.collect().await?;
    let bundle_content: Vec<String> = serde_json::from_slice(&bundle_body.to_vec())?;

    assert_eq!(
        bundle_content.len(),
        2,
        "Should have both transactions in bundle index"
    );
    assert!(bundle_content.contains(&format!("{:?}", tx1.id.hash)));
    assert!(bundle_content.contains(&format!("{:?}", tx2.id.hash)));

    let canonical_key = format!(
        "transactions/canonical/{:?}/{}",
        tx1.id.sender, tx1.id.nonce
    );
    let canonical_response = harness.s3_client
        .get_object()
        .bucket(&harness.bucket_name)
        .key(&canonical_key)
        .send()
        .await?;
    let canonical_body = canonical_response.body.collect().await?;
    let canonical_event: CanonicalTransactionEvent =
        serde_json::from_slice(&canonical_body.to_vec())?;

    assert_eq!(
        canonical_event.event_log.len(),
        2,
        "Should have both events in canonical log"
    );
    assert_eq!(canonical_event.event_log[0].bundle_id(), bundle_id);
    assert_eq!(canonical_event.event_log[1].bundle_id(), bundle_id);
    Ok(())
}
