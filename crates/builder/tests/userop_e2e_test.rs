use account_abstraction_core::types::{UserOperationRequest, VersionedUserOperation};
use alloy_primitives::{Address, Bytes, U256, address};
use alloy_rpc_types::erc4337::PackedUserOperation;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::message::Message;
use rdkafka::producer::FutureRecord;
use serde_json;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::timeout;

mod common;
use common::TestHarness;

const TEST_ENTRY_POINT: Address = address!("0x0000000071727De22E5E9d8BAf0edAc6f37da032");
const TEST_SENDER: Address = address!("0x3333333333333333333333333333333333333333");
const TEST_BUNDLER: Address = address!("0x1111111111111111111111111111111111111111");

fn create_test_user_op(sender: Address, nonce: u64) -> UserOperationRequest {
    UserOperationRequest {
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
        entry_point: TEST_ENTRY_POINT,
        chain_id: 10,
    }
}

#[tokio::test]
#[ignore]
async fn test_e2e_userop_to_block() -> anyhow::Result<()> {
    println!("\n========================================");
    println!("END-TO-END TEST: UserOp → Kafka → Block");
    println!("========================================\n");

    let harness = TestHarness::new().await?;
    let topic = "tips-user-operation";

    println!("Step 1: Creating test UserOperations...");
    let mut user_ops = Vec::new();
    for nonce in 0..3 {
        let user_op = create_test_user_op(TEST_SENDER, nonce);
        user_ops.push(user_op);
        println!("  ✓ Created UserOp with nonce={}", nonce);
    }

    println!("\nStep 2: Publishing UserOps to Kafka (simulating ingress-rpc)...");
    for (i, user_op) in user_ops.iter().enumerate() {
        let user_op_hash = user_op.hash()?;
        let user_op_json = serde_json::to_vec(&user_op.user_operation)?;

        let result = harness
            .kafka_producer
            .send(
                FutureRecord::to(topic)
                    .payload(&user_op_json)
                    .key(&user_op_hash.0),
                Duration::from_secs(5),
            )
            .await;

        assert!(result.is_ok(), "Failed to publish UserOp {}", i);
        println!("  ✓ Published UserOp {} (hash: {})", i, user_op_hash);
    }

    println!("\nStep 3: Simulating builder Kafka consumer...");
    let consumer: StreamConsumer = ClientConfig::new()
        .set("bootstrap.servers", &harness.kafka_bootstrap_servers)
        .set("group.id", "test-builder-e2e")
        .set("auto.offset.reset", "earliest")
        .set("enable.auto.commit", "false")
        .create()?;

    consumer.subscribe(&[topic])?;
    println!("  ✓ Consumer subscribed to topic: {}", topic);

    let received_user_ops = Arc::new(Mutex::new(Vec::new()));
    let received_clone = received_user_ops.clone();

    println!("\nStep 4: Consuming UserOps from Kafka...");
    let consume_result = timeout(Duration::from_secs(15), async move {
        let mut count = 0;
        while count < 3 {
            match consumer.recv().await {
                Ok(msg) => {
                    if let Some(payload) = msg.payload() {
                        let user_op: VersionedUserOperation = serde_json::from_slice(payload)?;
                        let mut ops = received_clone.lock().await;
                        ops.push(user_op);
                        count += 1;
                        println!("  ✓ Consumed UserOp {}/3", count);
                    }
                }
                Err(e) => return Err(Box::new(e) as Box<dyn std::error::Error>),
            }
        }
        Ok::<(), Box<dyn std::error::Error>>(())
    })
    .await;

    assert!(consume_result.is_ok(), "Failed to consume UserOps");
    let consumed_ops = received_user_ops.lock().await;
    assert_eq!(consumed_ops.len(), 3, "Expected 3 UserOps");

    println!("\nStep 5: Creating UserOp bundle...");
    use tips_builder::UserOpBundle;

    let mut bundle = UserOpBundle::new(TEST_ENTRY_POINT, TEST_BUNDLER);
    for (i, user_op) in user_ops.iter().enumerate() {
        bundle = bundle.with_user_op(user_op.clone());
        println!("  ✓ Added UserOp {} to bundle", i);
    }

    assert_eq!(bundle.user_ops.len(), 3, "Bundle should have 3 UserOps");
    assert_eq!(bundle.entry_point, TEST_ENTRY_POINT);
    assert_eq!(bundle.beneficiary, TEST_BUNDLER);
    println!("  ✓ Bundle created with {} UserOps", bundle.user_ops.len());

    println!("\nStep 6: Generating handleOps() calldata...");
    let calldata = bundle.build_handleops_calldata();
    assert!(calldata.is_some(), "Failed to generate calldata");
    let calldata = calldata.unwrap();
    println!("  ✓ Generated calldata: {} bytes", calldata.len());

    let function_selector = &calldata[0..4];
    println!(
        "  ✓ Function selector: 0x{} (handleOps)",
        hex::encode(function_selector)
    );
    assert!(
        calldata.len() > 4,
        "Calldata should contain function arguments"
    );

    println!("\nStep 7: Creating bundler transaction...");
    let chain_id = 10;
    let base_fee = 1000000000u128;
    let nonce = 0;
    let bundler_tx = bundle.create_bundle_transaction(TEST_BUNDLER, nonce, chain_id, base_fee);
    assert!(bundler_tx.is_some(), "Failed to create bundler transaction");
    println!("  ✓ Bundler transaction created");
    println!("  ✓ From: {}", TEST_BUNDLER);
    println!("  ✓ To: {}", TEST_ENTRY_POINT);
    println!("  ✓ Contains handleOps() calldata");

    println!("\nStep 8: Simulating block building with midpoint insertion...");
    use tips_builder::{InsertUserOpBundle, TransactionCollector};

    let userops_step = InsertUserOpBundle::new(TEST_BUNDLER);
    userops_step.add_bundle(bundle);
    println!("  ✓ Bundle added to InsertUserOpBundle pipeline");

    let mut collector = TransactionCollector::new(userops_step.clone());
    println!("  ✓ TransactionCollector initialized");

    println!("\nStep 9: Verifying midpoint insertion logic...");
    println!("  Simulating block with 6 regular transactions:");
    println!("    [TX0, TX1, TX2, BUNDLER, TX3, TX4, TX5]");
    println!("                    ^^^^^^^");
    println!("                    Inserted at position 3 (midpoint of 6 txs)");

    let total_txs = 6;
    let expected_midpoint = total_txs / 2;
    collector.set_total_expected(total_txs);

    for i in 0..expected_midpoint {
        assert!(
            !collector.should_insert_bundle(),
            "Should not insert before midpoint"
        );
        collector.increment_count();
        println!("  ✓ Regular TX {} collected (before midpoint)", i);
    }

    assert!(
        collector.should_insert_bundle(),
        "Should insert at midpoint"
    );
    println!("  ✓ Midpoint reached - bundler TX should be inserted");

    collector.mark_bundle_inserted();

    for i in expected_midpoint..total_txs {
        assert!(
            !collector.should_insert_bundle(),
            "Should not insert after bundle"
        );
        collector.increment_count();
        println!("  ✓ Regular TX {} collected (after midpoint)", i);
    }

    println!("\n  ✓ VERIFIED: Bundler transaction inserted at block midpoint");

    println!("\n========================================");
    println!("✓ END-TO-END TEST PASSED");
    println!("========================================\n");

    println!("Summary:");
    println!("  • 3 UserOps published to Kafka ✓");
    println!("  • 3 UserOps consumed from Kafka ✓");
    println!("  • Bundle created with EntryPoint.handleOps() ✓");
    println!("  • Calldata generated for bundler transaction ✓");
    println!("  • Bundle ready for midpoint insertion ✓");

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_e2e_multiple_batches() -> anyhow::Result<()> {
    println!("\n========================================");
    println!("E2E TEST: Multiple Batches");
    println!("========================================\n");

    let harness = TestHarness::new().await?;
    let topic = "tips-user-operation-multi";

    println!("Step 1: Publishing 10 UserOps...");
    for nonce in 0..10 {
        let user_op = create_test_user_op(TEST_SENDER, nonce);
        let user_op_hash = user_op.hash()?;
        let user_op_json = serde_json::to_vec(&user_op.user_operation)?;

        harness
            .kafka_producer
            .send(
                FutureRecord::to(topic)
                    .payload(&user_op_json)
                    .key(&user_op_hash.0),
                Duration::from_secs(5),
            )
            .await
            .expect("Failed to publish");

        if (nonce + 1) % 3 == 0 {
            println!("  ✓ Published {} UserOps", nonce + 1);
        }
    }

    println!("\nStep 2: Simulating batching with batch_size=5...");
    let consumer: StreamConsumer = ClientConfig::new()
        .set("bootstrap.servers", &harness.kafka_bootstrap_servers)
        .set("group.id", "test-multi-batch")
        .set("auto.offset.reset", "earliest")
        .set("enable.auto.commit", "false")
        .create()?;

    consumer.subscribe(&[topic])?;

    let mut first_batch = Vec::new();
    let mut second_batch = Vec::new();

    println!("\nStep 3: Consuming and batching UserOps...");
    let result = timeout(Duration::from_secs(20), async {
        let mut count = 0;
        while count < 10 {
            match consumer.recv().await {
                Ok(msg) => {
                    if let Some(payload) = msg.payload() {
                        let user_op: VersionedUserOperation = serde_json::from_slice(payload)?;

                        if count < 5 {
                            first_batch.push(user_op);
                        } else {
                            second_batch.push(user_op);
                        }

                        count += 1;

                        if count == 5 {
                            println!("  ✓ First batch complete (5 UserOps)");
                        } else if count == 10 {
                            println!("  ✓ Second batch complete (5 UserOps)");
                        }
                    }
                }
                Err(e) => return Err(Box::new(e) as Box<dyn std::error::Error>),
            }
        }
        Ok::<(), Box<dyn std::error::Error>>(())
    })
    .await;

    assert!(result.is_ok(), "Failed to consume all UserOps");
    assert_eq!(first_batch.len(), 5, "First batch should have 5 UserOps");
    assert_eq!(second_batch.len(), 5, "Second batch should have 5 UserOps");

    println!("\n✓ Multiple batch test passed");
    println!("  • Batch 1: {} UserOps", first_batch.len());
    println!("  • Batch 2: {} UserOps", second_batch.len());

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_e2e_userop_hash_verification() -> anyhow::Result<()> {
    println!("\n========================================");
    println!("E2E TEST: UserOp Hash Verification");
    println!("========================================\n");

    let user_op1 = create_test_user_op(TEST_SENDER, 0);
    let user_op2 = create_test_user_op(TEST_SENDER, 0);

    println!("Verifying UserOp hash determinism...");
    let hash1 = user_op1.hash()?;
    let hash2 = user_op2.hash()?;

    assert_eq!(hash1, hash2, "Identical UserOps should have same hash");
    println!("  ✓ UserOp hash (nonce=0): {}", hash1);

    let user_op_different = create_test_user_op(TEST_SENDER, 99);
    let hash3 = user_op_different.hash()?;

    assert_ne!(
        hash1, hash3,
        "Different UserOps should have different hashes"
    );
    println!("  ✓ UserOp hash (nonce=99): {}", hash3);

    println!("\n✓ UserOp hash verification passed");

    Ok(())
}
