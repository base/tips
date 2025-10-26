use alloy_signer_local::PrivateKeySigner;
use rdkafka::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::{kafka, kafka::Kafka};
use tips_audit::BundleEvent;
use tips_bundle_pool::{BundleStore, InMemoryBundlePool, KafkaBundleSource};
use tips_core::{
    BundleWithMetadata,
    test_utils::{create_test_bundle, create_transaction},
};
use tokio::sync::mpsc;

#[tokio::test]
async fn test_kafka_bundle_source_to_pool_integration() -> Result<(), Box<dyn std::error::Error>> {
    let kafka_container = Kafka::default().start().await?;
    let bootstrap_servers = format!(
        "127.0.0.1:{}",
        kafka_container
            .get_host_port_ipv4(kafka::KAFKA_PORT)
            .await?
    );

    let topic = "test-bundles";

    let kafka_producer = ClientConfig::new()
        .set("bootstrap.servers", &bootstrap_servers)
        .set("message.timeout.ms", "5000")
        .create::<FutureProducer>()?;


    let (bundle_tx, mut bundle_rx) = mpsc::unbounded_channel::<BundleWithMetadata>();
    let mut kafka_consumer_config = ClientConfig::new();
    kafka_consumer_config
        .set("group.id", "bundle-pool-test-source")
        .set("bootstrap.servers", &bootstrap_servers)
        .set("session.timeout.ms", "6000")
        .set("enable.auto.commit", "false")
        .set("auto.offset.reset", "earliest");

    let kafka_source = KafkaBundleSource::new(kafka_consumer_config, topic.to_string(), bundle_tx)?;
    tokio::spawn(async move {
        kafka_source.run().await.expect("Kafka source failed");
    });

    let (audit_tx, _audit_rx) = mpsc::unbounded_channel::<BundleEvent>();
    let pool = Arc::new(Mutex::new(InMemoryBundlePool::new(
        audit_tx,
        "test-builder".to_string(),
    )));

    let pool_clone = pool.clone();
    tokio::spawn(async move {
        while let Some(bundle) = bundle_rx.recv().await {
            pool_clone.lock().unwrap().add_bundle(bundle);
        }
    });

    let alice = PrivateKeySigner::random();
    let bob = PrivateKeySigner::random();
    let tx1 = create_transaction(alice.clone(), 1, bob.address());
    let test_bundle = create_test_bundle(vec![tx1], Some(100), None, None);
    let test_bundle_uuid = *test_bundle.uuid();

    let bundle_payload = serde_json::to_string(test_bundle.bundle())?;

    kafka_producer
        .send(
            FutureRecord::to(topic)
                .payload(&bundle_payload)
                .key("test-key"),
            Duration::from_secs(5),
        )
        .await
        .map_err(|(e, _)| e)?;

    let mut counter = 0;
    loop {
        counter += 1;
        if counter > 10 {
            panic!("Bundle was not added to pool within timeout");
        }

        tokio::time::sleep(Duration::from_secs(1)).await;

        let bundles = pool.lock().unwrap().get_bundles();
        if bundles.is_empty() {
            continue;
        }

        assert_eq!(bundles.len(), 1);
        assert_eq!(*bundles[0].uuid(), test_bundle_uuid);
        break;
    }

    Ok(())
}
