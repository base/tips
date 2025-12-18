use account_abstraction_core::types::{UserOperationRequest, VersionedUserOperation};
use alloy_primitives::{Address, B256, Bytes, U256, address};
use alloy_rpc_types::erc4337::PackedUserOperation;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::message::Message;
use rdkafka::producer::{FutureProducer, FutureRecord};
use serde_json;
use std::time::Duration;
use tokio::time::timeout;

mod common;
use common::TestHarness;

#[tokio::test]
async fn test_userop_kafka_flow() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let harness = TestHarness::new().await?;
    let topic = "test-user-operation";

    let entry_point = address!("0x0000000071727De22E5E9d8BAf0edAc6f37da032");
    let sender = address!("0x3333333333333333333333333333333333333333");

    let user_op_request = UserOperationRequest {
        user_operation: VersionedUserOperation::PackedUserOperation(PackedUserOperation {
            sender,
            nonce: U256::from(0),
            call_data: Bytes::default(),
            call_gas_limit: U256::from(100000),
            verification_gas_limit: U256::from(500000),
            pre_verification_gas: U256::from(21000),
            max_fee_per_gas: U256::from(2000000000),
            max_priority_fee_per_gas: U256::from(1000000000),
            signature: Bytes::default(),
            factory: None,
            factory_data: None,
            paymaster: None,
            paymaster_verification_gas_limit: None,
            paymaster_post_op_gas_limit: None,
            paymaster_data: None,
        }),
        entry_point,
        chain_id: 10,
    };

    let user_op_hash = user_op_request.hash()?;
    let user_op_json = serde_json::to_vec(&user_op_request.user_operation)?;

    let delivery_status = harness
        .kafka_producer
        .send(
            FutureRecord::to(topic)
                .payload(&user_op_json)
                .key(&user_op_hash.0),
            Duration::from_secs(5),
        )
        .await;

    assert!(delivery_status.is_ok(), "Failed to publish UserOp to Kafka");

    let consumer: StreamConsumer = ClientConfig::new()
        .set("bootstrap.servers", &harness.kafka_bootstrap_servers)
        .set("group.id", "test-consumer-group")
        .set("auto.offset.reset", "earliest")
        .set("enable.auto.commit", "false")
        .create()?;

    consumer.subscribe(&[topic])?;

    let message_result = timeout(Duration::from_secs(10), async {
        loop {
            match consumer.recv().await {
                Ok(msg) => {
                    if let Some(payload) = msg.payload() {
                        let received_user_op: VersionedUserOperation =
                            serde_json::from_slice(payload)?;

                        match received_user_op {
                            VersionedUserOperation::PackedUserOperation(packed) => {
                                assert_eq!(packed.sender, sender);
                                assert_eq!(packed.nonce, U256::from(0));
                                break Ok::<(), Box<dyn std::error::Error + Send + Sync>>(());
                            }
                            _ => continue,
                        }
                    }
                }
                Err(e) => return Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>),
            }
        }
    })
    .await;

    assert!(
        message_result.is_ok(),
        "Failed to receive UserOp from Kafka within timeout"
    );

    Ok(())
}

#[tokio::test]
async fn test_userop_batching() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let harness = TestHarness::new().await?;
    let topic = "test-user-operation-batching";

    let entry_point = address!("0x0000000071727De22E5E9d8BAf0edAc6f37da032");
    let sender = address!("0x3333333333333333333333333333333333333333");

    let mut user_op_hashes = Vec::new();

    for nonce in 0..5 {
        let user_op_request = UserOperationRequest {
            user_operation: VersionedUserOperation::PackedUserOperation(PackedUserOperation {
                sender,
                nonce: U256::from(nonce),
                call_data: Bytes::default(),
                call_gas_limit: U256::from(100000),
                verification_gas_limit: U256::from(500000),
                pre_verification_gas: U256::from(21000),
                max_fee_per_gas: U256::from(2000000000),
                max_priority_fee_per_gas: U256::from(1000000000),
                signature: Bytes::default(),
                factory: None,
                factory_data: None,
                paymaster: None,
                paymaster_verification_gas_limit: None,
                paymaster_post_op_gas_limit: None,
                paymaster_data: None,
            }),
            entry_point,
            chain_id: 10,
        };

        let user_op_hash = user_op_request.hash()?;
        user_op_hashes.push(user_op_hash);

        let user_op_json = serde_json::to_vec(&user_op_request.user_operation)?;

        harness
            .kafka_producer
            .send(
                FutureRecord::to(topic)
                    .payload(&user_op_json)
                    .key(&user_op_hash.0),
                Duration::from_secs(5),
            )
            .await
            .expect("Failed to send UserOp");
    }

    let consumer: StreamConsumer = ClientConfig::new()
        .set("bootstrap.servers", &harness.kafka_bootstrap_servers)
        .set("group.id", "test-batch-consumer")
        .set("auto.offset.reset", "earliest")
        .set("enable.auto.commit", "false")
        .create()?;

    consumer.subscribe(&[topic])?;

    let mut received_count = 0;
    let receive_result = timeout(Duration::from_secs(15), async {
        while received_count < 5 {
            match consumer.recv().await {
                Ok(msg) => {
                    if msg.payload().is_some() {
                        received_count += 1;
                    }
                }
                Err(e) => return Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>),
            }
        }
        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    })
    .await;

    assert!(
        receive_result.is_ok(),
        "Failed to receive all UserOps within timeout"
    );
    assert_eq!(received_count, 5, "Expected to receive 5 UserOps");

    Ok(())
}

#[test]
fn test_userop_hash_consistency() {
    let entry_point = address!("0x0000000071727De22E5E9d8BAf0edAc6f37da032");
    let sender = address!("0x3333333333333333333333333333333333333333");

    let user_op_request = UserOperationRequest {
        user_operation: VersionedUserOperation::PackedUserOperation(PackedUserOperation {
            sender,
            nonce: U256::from(0),
            call_data: Bytes::default(),
            call_gas_limit: U256::from(100000),
            verification_gas_limit: U256::from(500000),
            pre_verification_gas: U256::from(21000),
            max_fee_per_gas: U256::from(2000000000),
            max_priority_fee_per_gas: U256::from(1000000000),
            signature: Bytes::default(),
            factory: None,
            factory_data: None,
            paymaster: None,
            paymaster_verification_gas_limit: None,
            paymaster_post_op_gas_limit: None,
            paymaster_data: None,
        }),
        entry_point,
        chain_id: 10,
    };

    let hash1 = user_op_request.hash().expect("Failed to hash UserOp");
    let hash2 = user_op_request.hash().expect("Failed to hash UserOp");

    assert_eq!(hash1, hash2, "UserOp hash should be deterministic");
    assert_ne!(hash1, B256::ZERO, "UserOp hash should not be zero");
}

#[test]
fn test_userop_serialization() {
    let entry_point = address!("0x0000000071727De22E5E9d8BAf0edAc6f37da032");
    let sender = address!("0x3333333333333333333333333333333333333333");

    let user_op = VersionedUserOperation::PackedUserOperation(PackedUserOperation {
        sender,
        nonce: U256::from(0),
        call_data: Bytes::from_static(b"test"),
        call_gas_limit: U256::from(100000),
        verification_gas_limit: U256::from(500000),
        pre_verification_gas: U256::from(21000),
        max_fee_per_gas: U256::from(2000000000),
        max_priority_fee_per_gas: U256::from(1000000000),
        signature: Bytes::from_static(b"signature"),
        factory: None,
        factory_data: None,
        paymaster: None,
        paymaster_verification_gas_limit: None,
        paymaster_post_op_gas_limit: None,
        paymaster_data: None,
    });

    let json = serde_json::to_string(&user_op).expect("Failed to serialize");
    let deserialized: VersionedUserOperation =
        serde_json::from_str(&json).expect("Failed to deserialize");

    match (user_op, deserialized) {
        (
            VersionedUserOperation::PackedUserOperation(original),
            VersionedUserOperation::PackedUserOperation(restored),
        ) => {
            assert_eq!(original.sender, restored.sender);
            assert_eq!(original.nonce, restored.nonce);
            assert_eq!(original.call_data, restored.call_data);
            assert_eq!(original.signature, restored.signature);
        }
        _ => panic!("UserOp type mismatch after deserialization"),
    }
}
