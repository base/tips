use alloy_primitives::{Address, B256, TxHash, U256};
use std::time::Duration;
use tips_audit::{
    KafkaAuditArchiver, KafkaAuditLogReader, KafkaUserOpAuditArchiver, KafkaUserOpAuditLogReader,
    UserOpEventReader,
    publisher::{
        BundleEventPublisher, KafkaBundleEventPublisher, KafkaUserOpEventPublisher,
        UserOpEventPublisher,
    },
    storage::{BundleEventS3Reader, S3EventReaderWriter, UserOpEventS3Reader, UserOpEventWriter},
    types::{BundleEvent, DropReason, UserOpDropReason, UserOpEvent},
};
use tips_core::test_utils::create_bundle_from_txn_data;
use uuid::Uuid;
mod common;
use common::TestHarness;

#[tokio::test]
#[ignore = "TODO doesn't appear to work with minio, should test against a real S3 bucket"]
async fn test_kafka_publisher_s3_archiver_integration()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let harness = TestHarness::new().await?;
    let topic = "test-mempool-events";

    let s3_writer =
        S3EventReaderWriter::new(harness.s3_client.clone(), harness.bucket_name.clone());

    let test_bundle_id = Uuid::new_v4();
    let test_events = [
        BundleEvent::Received {
            bundle_id: test_bundle_id,
            bundle: Box::new(create_bundle_from_txn_data()),
        },
        BundleEvent::Dropped {
            bundle_id: test_bundle_id,
            reason: DropReason::TimedOut,
        },
    ];

    let publisher = KafkaBundleEventPublisher::new(harness.kafka_producer, topic.to_string());

    for event in test_events.iter() {
        publisher.publish(event.clone()).await?;
    }

    let mut consumer = KafkaAuditArchiver::new(
        KafkaAuditLogReader::new(harness.kafka_consumer, topic.to_string())?,
        s3_writer.clone(),
    );

    tokio::spawn(async move {
        consumer.run().await.expect("error running consumer");
    });

    // Wait for the messages to be received
    let mut counter = 0;
    loop {
        counter += 1;
        if counter > 10 {
            assert!(false, "unable to complete archiving within the deadline");
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
        let bundle_history = s3_writer.get_bundle_history(test_bundle_id).await?;

        if bundle_history.is_some() {
            let history = bundle_history.unwrap();
            if history.history.len() != test_events.len() {
                continue;
            } else {
                break;
            }
        } else {
            continue;
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_userop_kafka_publisher_reader_integration()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let harness = TestHarness::new().await?;
    let topic = "test-userop-events";

    let test_user_op_hash = B256::from_slice(&[1u8; 32]);
    let test_sender = Address::from_slice(&[2u8; 20]);
    let test_entry_point = Address::from_slice(&[3u8; 20]);
    let test_nonce = U256::from(42);

    let test_event = UserOpEvent::AddedToMempool {
        user_op_hash: test_user_op_hash,
        sender: test_sender,
        entry_point: test_entry_point,
        nonce: test_nonce,
    };

    let publisher = KafkaUserOpEventPublisher::new(harness.kafka_producer, topic.to_string());
    publisher.publish(test_event.clone()).await?;

    let mut reader = KafkaUserOpAuditLogReader::new(harness.kafka_consumer, topic.to_string())?;

    let received = tokio::time::timeout(Duration::from_secs(10), reader.read_event()).await??;

    assert_eq!(received.event.user_op_hash(), test_user_op_hash);

    match received.event {
        UserOpEvent::AddedToMempool {
            user_op_hash,
            sender,
            entry_point,
            nonce,
        } => {
            assert_eq!(user_op_hash, test_user_op_hash);
            assert_eq!(sender, test_sender);
            assert_eq!(entry_point, test_entry_point);
            assert_eq!(nonce, test_nonce);
        }
        _ => panic!("Expected AddedToMempool event"),
    }

    reader.commit().await?;

    Ok(())
}

#[tokio::test]
#[ignore = "TODO doesn't appear to work with minio, should test against a real S3 bucket"]
async fn test_userop_kafka_publisher_s3_archiver_integration()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let harness = TestHarness::new().await?;
    let topic = "test-userop-audit-events";

    let s3_writer =
        S3EventReaderWriter::new(harness.s3_client.clone(), harness.bucket_name.clone());

    let test_user_op_hash = B256::from_slice(&[1u8; 32]);
    let test_sender = Address::from_slice(&[2u8; 20]);
    let test_entry_point = Address::from_slice(&[3u8; 20]);
    let test_nonce = U256::from(42);

    let test_event = UserOpEvent::AddedToMempool {
        user_op_hash: test_user_op_hash,
        sender: test_sender,
        entry_point: test_entry_point,
        nonce: test_nonce,
    };

    let publisher = KafkaUserOpEventPublisher::new(harness.kafka_producer, topic.to_string());
    publisher.publish(test_event.clone()).await?;

    let mut archiver = KafkaUserOpAuditArchiver::new(
        KafkaUserOpAuditLogReader::new(harness.kafka_consumer, topic.to_string())?,
        s3_writer.clone(),
    );

    tokio::spawn(async move {
        archiver.run().await.expect("error running archiver");
    });

    let mut counter = 0;
    loop {
        counter += 1;
        if counter > 10 {
            panic!("unable to complete archiving within the deadline");
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
        let history = s3_writer.get_userop_history(test_user_op_hash).await?;

        if let Some(h) = history {
            if !h.history.is_empty() {
                assert_eq!(h.history.len(), 1);
                break;
            }
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_userop_end_to_end_single_event()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let harness = TestHarness::new().await?;
    let topic = "test-e2e-single";

    let s3_writer =
        S3EventReaderWriter::new(harness.s3_client.clone(), harness.bucket_name.clone());

    let test_user_op_hash = B256::from_slice(&[10u8; 32]);
    let test_sender = Address::from_slice(&[11u8; 20]);
    let test_entry_point = Address::from_slice(&[12u8; 20]);
    let test_nonce = U256::from(100);

    let test_event = UserOpEvent::AddedToMempool {
        user_op_hash: test_user_op_hash,
        sender: test_sender,
        entry_point: test_entry_point,
        nonce: test_nonce,
    };

    let publisher = KafkaUserOpEventPublisher::new(harness.kafka_producer, topic.to_string());
    publisher.publish(test_event.clone()).await?;

    let mut reader = KafkaUserOpAuditLogReader::new(harness.kafka_consumer, topic.to_string())?;
    let received = tokio::time::timeout(Duration::from_secs(10), reader.read_event()).await??;

    s3_writer.archive_userop_event(received).await?;

    let history = s3_writer.get_userop_history(test_user_op_hash).await?;
    assert!(history.is_some(), "History should exist after archiving");

    let h = history.unwrap();
    assert_eq!(h.history.len(), 1, "Should have exactly one event");

    reader.commit().await?;
    Ok(())
}

#[tokio::test]
async fn test_userop_end_to_end_multiple_events_same_userop()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let harness = TestHarness::new().await?;
    let topic = "test-e2e-multiple";

    let s3_writer =
        S3EventReaderWriter::new(harness.s3_client.clone(), harness.bucket_name.clone());

    let test_user_op_hash = B256::from_slice(&[20u8; 32]);
    let test_sender = Address::from_slice(&[21u8; 20]);
    let test_entry_point = Address::from_slice(&[22u8; 20]);

    let events = vec![
        UserOpEvent::AddedToMempool {
            user_op_hash: test_user_op_hash,
            sender: test_sender,
            entry_point: test_entry_point,
            nonce: U256::from(1),
        },
        UserOpEvent::Included {
            user_op_hash: test_user_op_hash,
            block_number: 12345,
            tx_hash: TxHash::from_slice(&[99u8; 32]),
        },
    ];

    let publisher = KafkaUserOpEventPublisher::new(harness.kafka_producer, topic.to_string());
    for event in &events {
        publisher.publish(event.clone()).await?;
    }

    let mut reader = KafkaUserOpAuditLogReader::new(harness.kafka_consumer, topic.to_string())?;

    for _ in 0..events.len() {
        let received = tokio::time::timeout(Duration::from_secs(10), reader.read_event()).await??;
        s3_writer.archive_userop_event(received).await?;
        reader.commit().await?;
    }

    let history = s3_writer.get_userop_history(test_user_op_hash).await?;
    assert!(history.is_some());

    let h = history.unwrap();
    assert_eq!(h.history.len(), 2, "Should have two events in history");

    Ok(())
}

#[tokio::test]
async fn test_userop_end_to_end_kafka_redelivery_deduplication()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use tips_audit::storage::UserOpEventWrapper;

    let harness = TestHarness::new().await?;

    let s3_writer =
        S3EventReaderWriter::new(harness.s3_client.clone(), harness.bucket_name.clone());

    let test_user_op_hash = B256::from_slice(&[30u8; 32]);
    let test_sender = Address::from_slice(&[31u8; 20]);
    let test_entry_point = Address::from_slice(&[32u8; 20]);

    let test_event = UserOpEvent::AddedToMempool {
        user_op_hash: test_user_op_hash,
        sender: test_sender,
        entry_point: test_entry_point,
        nonce: U256::from(1),
    };

    let same_key = "same-key-for-redelivery".to_string();
    let wrapped1 = UserOpEventWrapper {
        key: same_key.clone(),
        event: test_event.clone(),
        timestamp: 1000,
    };
    let wrapped2 = UserOpEventWrapper {
        key: same_key.clone(),
        event: test_event.clone(),
        timestamp: 2000,
    };

    s3_writer.archive_userop_event(wrapped1).await?;
    s3_writer.archive_userop_event(wrapped2).await?;

    let history = s3_writer.get_userop_history(test_user_op_hash).await?;
    assert!(history.is_some());

    let h = history.unwrap();
    assert_eq!(
        h.history.len(),
        1,
        "Kafka redelivery (same key) should be deduplicated, expected 1 but got {}",
        h.history.len()
    );

    Ok(())
}

#[tokio::test]
async fn test_userop_end_to_end_all_event_types()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let harness = TestHarness::new().await?;
    let topic = "test-e2e-all-types";

    let s3_writer =
        S3EventReaderWriter::new(harness.s3_client.clone(), harness.bucket_name.clone());

    let test_user_op_hash = B256::from_slice(&[40u8; 32]);
    let test_sender = Address::from_slice(&[41u8; 20]);
    let test_entry_point = Address::from_slice(&[42u8; 20]);

    let events = vec![
        UserOpEvent::AddedToMempool {
            user_op_hash: test_user_op_hash,
            sender: test_sender,
            entry_point: test_entry_point,
            nonce: U256::from(1),
        },
        UserOpEvent::Dropped {
            user_op_hash: test_user_op_hash,
            reason: UserOpDropReason::Expired,
        },
    ];

    let publisher = KafkaUserOpEventPublisher::new(harness.kafka_producer, topic.to_string());
    for event in &events {
        publisher.publish(event.clone()).await?;
    }

    let mut reader = KafkaUserOpAuditLogReader::new(harness.kafka_consumer, topic.to_string())?;

    for _ in 0..events.len() {
        let received = tokio::time::timeout(Duration::from_secs(10), reader.read_event()).await??;
        s3_writer.archive_userop_event(received).await?;
        reader.commit().await?;
    }

    let history = s3_writer.get_userop_history(test_user_op_hash).await?;
    assert!(history.is_some());

    let h = history.unwrap();
    assert_eq!(h.history.len(), 2, "Should have both event types");

    Ok(())
}

#[tokio::test]
async fn test_userop_end_to_end_dropped_with_reason()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let harness = TestHarness::new().await?;
    let topic = "test-e2e-dropped";

    let s3_writer =
        S3EventReaderWriter::new(harness.s3_client.clone(), harness.bucket_name.clone());

    let test_user_op_hash = B256::from_slice(&[50u8; 32]);

    let test_event = UserOpEvent::Dropped {
        user_op_hash: test_user_op_hash,
        reason: UserOpDropReason::Invalid("gas too low".to_string()),
    };

    let publisher = KafkaUserOpEventPublisher::new(harness.kafka_producer, topic.to_string());
    publisher.publish(test_event.clone()).await?;

    let mut reader = KafkaUserOpAuditLogReader::new(harness.kafka_consumer, topic.to_string())?;
    let received = tokio::time::timeout(Duration::from_secs(10), reader.read_event()).await??;

    match &received.event {
        UserOpEvent::Dropped { reason, .. } => match reason {
            UserOpDropReason::Invalid(msg) => {
                assert_eq!(msg, "gas too low");
            }
            _ => panic!("Expected Invalid reason"),
        },
        _ => panic!("Expected Dropped event"),
    }

    s3_writer.archive_userop_event(received).await?;

    let history = s3_writer.get_userop_history(test_user_op_hash).await?;
    assert!(history.is_some());
    assert_eq!(history.unwrap().history.len(), 1);

    reader.commit().await?;
    Ok(())
}

#[tokio::test]
async fn test_userop_end_to_end_included_event()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let harness = TestHarness::new().await?;
    let topic = "test-e2e-included";

    let s3_writer =
        S3EventReaderWriter::new(harness.s3_client.clone(), harness.bucket_name.clone());

    let test_user_op_hash = B256::from_slice(&[60u8; 32]);
    let test_tx_hash = TxHash::from_slice(&[61u8; 32]);
    let test_block_number = 999999u64;

    let test_event = UserOpEvent::Included {
        user_op_hash: test_user_op_hash,
        block_number: test_block_number,
        tx_hash: test_tx_hash,
    };

    let publisher = KafkaUserOpEventPublisher::new(harness.kafka_producer, topic.to_string());
    publisher.publish(test_event.clone()).await?;

    let mut reader = KafkaUserOpAuditLogReader::new(harness.kafka_consumer, topic.to_string())?;
    let received = tokio::time::timeout(Duration::from_secs(10), reader.read_event()).await??;

    match &received.event {
        UserOpEvent::Included {
            block_number,
            tx_hash,
            ..
        } => {
            assert_eq!(*block_number, test_block_number);
            assert_eq!(*tx_hash, test_tx_hash);
        }
        _ => panic!("Expected Included event"),
    }

    s3_writer.archive_userop_event(received).await?;

    let history = s3_writer.get_userop_history(test_user_op_hash).await?;
    assert!(history.is_some());
    assert_eq!(history.unwrap().history.len(), 1);

    reader.commit().await?;
    Ok(())
}

#[tokio::test]
async fn test_userop_end_to_end_full_lifecycle()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let harness = TestHarness::new().await?;
    let topic = "test-e2e-lifecycle";

    let s3_writer =
        S3EventReaderWriter::new(harness.s3_client.clone(), harness.bucket_name.clone());

    let test_user_op_hash = B256::from_slice(&[70u8; 32]);
    let test_sender = Address::from_slice(&[71u8; 20]);
    let test_entry_point = Address::from_slice(&[72u8; 20]);
    let test_tx_hash = TxHash::from_slice(&[73u8; 32]);

    let lifecycle_events = vec![
        UserOpEvent::AddedToMempool {
            user_op_hash: test_user_op_hash,
            sender: test_sender,
            entry_point: test_entry_point,
            nonce: U256::from(1),
        },
        UserOpEvent::Included {
            user_op_hash: test_user_op_hash,
            block_number: 12345,
            tx_hash: test_tx_hash,
        },
    ];

    let publisher = KafkaUserOpEventPublisher::new(harness.kafka_producer, topic.to_string());
    for event in &lifecycle_events {
        publisher.publish(event.clone()).await?;
    }

    let mut reader = KafkaUserOpAuditLogReader::new(harness.kafka_consumer, topic.to_string())?;

    for _ in 0..lifecycle_events.len() {
        let received = tokio::time::timeout(Duration::from_secs(10), reader.read_event()).await??;
        s3_writer.archive_userop_event(received).await?;
        reader.commit().await?;
    }

    let history = s3_writer.get_userop_history(test_user_op_hash).await?;
    assert!(history.is_some());

    let h = history.unwrap();
    assert_eq!(
        h.history.len(),
        2,
        "Full lifecycle should have 2 events (AddedToMempool, Included)"
    );

    Ok(())
}
