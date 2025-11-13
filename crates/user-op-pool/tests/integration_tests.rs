use alloy_primitives::{address, b256, Address, U256};
use rdkafka::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::testcontainers::ContainerAsync;
use testcontainers_modules::{kafka, kafka::Kafka};
use tips_user_op_pool::{
    InMemoryUserOpPool, KafkaUserOpSource, UserOpPoolItem, UserOpStore, connect_sources_to_pool,
};
use tokio::sync::mpsc;

async fn setup_kafka()
-> Result<(ContainerAsync<Kafka>, FutureProducer, ClientConfig), Box<dyn std::error::Error>> {
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
        .create::<FutureProducer>()?;

    let mut kafka_consumer_config = ClientConfig::new();
    kafka_consumer_config
        .set("group.id", "user-op-pool-test-source")
        .set("bootstrap.servers", &bootstrap_servers)
        .set("session.timeout.ms", "6000")
        .set("enable.auto.commit", "false")
        .set("auto.offset.reset", "earliest");

    Ok((kafka_container, kafka_producer, kafka_consumer_config))
}

fn create_test_user_op_item(sender: Address, nonce: u64) -> UserOpPoolItem {
    use tips_ingress_rpc::{UserOperation, UserOperationV06};
    
    let user_op = UserOperation::V06(UserOperationV06 {
        sender,
        nonce: U256::from(nonce),
        init_code: Default::default(),
        call_data: Default::default(),
        call_gas_limit: U256::from(100000),
        verification_gas_limit: U256::from(100000),
        pre_verification_gas: U256::from(21000),
        max_fee_per_gas: U256::from(1000000000),
        max_priority_fee_per_gas: U256::from(1000000000),
        paymaster_and_data: Default::default(),
        signature: Default::default(),
    });

    let entry_point = address!("0000000071727De22E5E9d8BAf0edAc6f37da032");
    let hash = b256!("0000000000000000000000000000000000000000000000000000000000000001");

    UserOpPoolItem::new(user_op, entry_point, hash)
}

#[tokio::test]
async fn test_kafka_user_op_source_to_pool_integration() -> Result<(), Box<dyn std::error::Error>>
{
    let topic = "test-user-operations";
    let (_kafka_container, kafka_producer, kafka_consumer_config) = setup_kafka().await?;

    let (user_op_tx, user_op_rx) = mpsc::unbounded_channel::<UserOpPoolItem>();

    let kafka_source =
        KafkaUserOpSource::new(kafka_consumer_config, topic.to_string(), user_op_tx)?;

    let pool = Arc::new(Mutex::new(InMemoryUserOpPool::new()));

    connect_sources_to_pool(vec![kafka_source], user_op_rx, pool.clone());

    let sender = address!("1000000000000000000000000000000000000001");
    let test_item = create_test_user_op_item(sender, 1);
    let test_item_id = test_item.id.clone();

    let item_payload = serde_json::to_string(&test_item)?;

    kafka_producer
        .send(
            FutureRecord::to(topic)
                .payload(&item_payload)
                .key("test-key"),
            Duration::from_secs(5),
        )
        .await
        .map_err(|(e, _)| e)?;

    let mut counter = 0;
    loop {
        counter += 1;
        assert!(counter < 10, "Timeout waiting for UserOp in pool");

        tokio::time::sleep(Duration::from_millis(500)).await;

        let user_ops = pool.lock().unwrap().get_user_ops();
        if user_ops.is_empty() {
            continue;
        }

        assert_eq!(user_ops.len(), 1);
        assert_eq!(user_ops[0].id, test_item_id);
        break;
    }

    Ok(())
}

