use alloy_primitives::{Address, Bytes, TxHash, address, b256, bytes};
use alloy_rpc_types_mev::EthSendBundle;
use sqlx::PgPool;
use testcontainers_modules::{
    postgres,
    testcontainers::{ContainerAsync, runners::AsyncRunner},
};
use tips_datastore::postgres::{BundleFilter, BundleState};
use tips_datastore::{BundleDatastore, PostgresDatastore};

struct TestHarness {
    _postgres_instance: ContainerAsync<postgres::Postgres>,
    data_store: PostgresDatastore,
}

async fn setup_datastore() -> eyre::Result<TestHarness> {
    let postgres_instance = postgres::Postgres::default().start().await?;
    let connection_string = format!(
        "postgres://postgres:postgres@{}:{}/postgres",
        postgres_instance.get_host().await?,
        postgres_instance.get_host_port_ipv4(5432).await?
    );

    let pool = PgPool::connect(&connection_string).await?;
    let data_store = PostgresDatastore::new(pool);

    assert!(data_store.run_migrations().await.is_ok());
    Ok(TestHarness {
        _postgres_instance: postgres_instance,
        data_store,
    })
}

const TX_DATA: Bytes = bytes!(
    "0x02f8bf8221058304f8c782038c83d2a76b833d0900942e85c218afcdeb3d3b3f0f72941b4861f915bbcf80b85102000e0000000bb800001010c78c430a094eb7ae67d41a7cca25cdb9315e63baceb03bf4529e57a6b1b900010001f4000a101010110111101111011011faa7efc8e6aa13b029547eecbf5d370b4e1e52eec080a009fc02a6612877cec7e1223f0a14f9a9507b82ef03af41fcf14bf5018ccf2242a0338b46da29a62d28745c828077327588dc82c03a4b0210e3ee1fd62c608f8fcd"
);
const TX_HASH: TxHash = b256!("0x3ea7e1482485387e61150ee8e5c8cad48a14591789ac02cc2504046d96d0a5f4");
const TX_SENDER: Address = address!("0x24ae36512421f1d9f6e074f00ff5b8393f5dd925");

fn create_test_bundle_with_reverting_tx() -> eyre::Result<EthSendBundle> {
    Ok(EthSendBundle {
        txs: vec![TX_DATA],
        block_number: 12345,
        min_timestamp: Some(1640995200),
        max_timestamp: Some(1640995260),
        reverting_tx_hashes: vec![TX_HASH],
        replacement_uuid: None,
        dropping_tx_hashes: vec![],
        refund_percent: None,
        refund_recipient: None,
        refund_tx_hashes: vec![],
        extra_fields: Default::default(),
    })
}

fn create_test_bundle(
    block_number: u64,
    min_timestamp: Option<u64>,
    max_timestamp: Option<u64>,
) -> eyre::Result<EthSendBundle> {
    Ok(EthSendBundle {
        txs: vec![TX_DATA],
        block_number,
        min_timestamp,
        max_timestamp,
        reverting_tx_hashes: vec![],
        replacement_uuid: None,
        dropping_tx_hashes: vec![],
        refund_percent: None,
        refund_recipient: None,
        refund_tx_hashes: vec![],
        extra_fields: Default::default(),
    })
}

#[tokio::test]
async fn insert_and_get() -> eyre::Result<()> {
    let harness = setup_datastore().await?;
    let test_bundle = create_test_bundle_with_reverting_tx()?;

    let insert_result = harness.data_store.insert_bundle(test_bundle.clone()).await;
    if let Err(ref err) = insert_result {
        eprintln!("Insert failed with error: {err:?}");
    }
    assert!(insert_result.is_ok());
    let bundle_id = insert_result.unwrap();

    let query_result = harness.data_store.get_bundle(bundle_id).await;
    assert!(query_result.is_ok());
    let retrieved_bundle_with_metadata = query_result.unwrap();

    assert!(
        retrieved_bundle_with_metadata.is_some(),
        "Bundle should be found"
    );
    let metadata = retrieved_bundle_with_metadata.unwrap();
    let retrieved_bundle = &metadata.bundle;

    assert!(
        matches!(metadata.state, BundleState::Ready),
        "Bundle should default to Ready state"
    );
    assert_eq!(retrieved_bundle.txs.len(), test_bundle.txs.len());
    assert_eq!(retrieved_bundle.block_number, test_bundle.block_number);
    assert_eq!(retrieved_bundle.min_timestamp, test_bundle.min_timestamp);
    assert_eq!(retrieved_bundle.max_timestamp, test_bundle.max_timestamp);
    assert_eq!(
        retrieved_bundle.reverting_tx_hashes.len(),
        test_bundle.reverting_tx_hashes.len()
    );
    assert_eq!(
        retrieved_bundle.dropping_tx_hashes.len(),
        test_bundle.dropping_tx_hashes.len()
    );

    assert!(
        !metadata.txn_hashes.is_empty(),
        "Transaction hashes should not be empty"
    );
    assert!(!metadata.senders.is_empty(), "Senders should not be empty");
    assert_eq!(
        metadata.txn_hashes.len(),
        1,
        "Should have one transaction hash"
    );
    assert_eq!(metadata.senders.len(), 1, "Should have one sender");
    assert!(
        metadata.min_base_fee >= 0,
        "Min base fee should be non-negative"
    );

    assert_eq!(
        metadata.txn_hashes[0], TX_HASH,
        "Transaction hash should match"
    );
    assert_eq!(metadata.senders[0], TX_SENDER, "Sender should match");

    // Test that state_changed_at is set (should be recent)
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let state_changed_timestamp = metadata.state_changed_at.timestamp();
    assert!(
        (now - state_changed_timestamp).abs() < 10,
        "state_changed_at should be recent (within 10 seconds)"
    );

    Ok(())
}

#[tokio::test]
async fn select_bundles_comprehensive() -> eyre::Result<()> {
    let harness = setup_datastore().await?;

    let bundle1 = create_test_bundle(100, Some(1000), Some(2000))?;
    let bundle2 = create_test_bundle(200, Some(1500), Some(2500))?;
    let bundle3 = create_test_bundle(300, None, None)?; // valid for all times
    let bundle4 = create_test_bundle(0, Some(500), Some(3000))?; // valid for all blocks

    harness
        .data_store
        .insert_bundle(bundle1)
        .await
        .expect("Failed to insert bundle1");
    harness
        .data_store
        .insert_bundle(bundle2)
        .await
        .expect("Failed to insert bundle2");
    harness
        .data_store
        .insert_bundle(bundle3)
        .await
        .expect("Failed to insert bundle3");
    harness
        .data_store
        .insert_bundle(bundle4)
        .await
        .expect("Failed to insert bundle4");

    let empty_filter = BundleFilter::new();
    let all_bundles = harness
        .data_store
        .select_bundles(empty_filter)
        .await
        .expect("Failed to select bundles with empty filter");
    assert_eq!(
        all_bundles.len(),
        4,
        "Should return all 4 bundles with empty filter"
    );

    let block_filter = BundleFilter::new().valid_for_block(200);
    let filtered_bundles = harness
        .data_store
        .select_bundles(block_filter)
        .await
        .expect("Failed to select bundles with block filter");
    assert_eq!(
        filtered_bundles.len(),
        2,
        "Should return 2 bundles for block 200 (bundle2 + bundle4 with block 0)"
    );
    assert_eq!(filtered_bundles[0].bundle.block_number, 200);

    let timestamp_filter = BundleFilter::new().valid_for_timestamp(1500);
    let timestamp_filtered = harness
        .data_store
        .select_bundles(timestamp_filter)
        .await
        .expect("Failed to select bundles with timestamp filter");
    assert_eq!(
        timestamp_filtered.len(),
        4,
        "Should return all 4 bundles (all contain timestamp 1500: bundle1[1000-2000], bundle2[1500-2500], bundle3[NULL-NULL], bundle4[500-3000])"
    );

    let combined_filter = BundleFilter::new()
        .valid_for_block(200)
        .valid_for_timestamp(2000);
    let combined_filtered = harness
        .data_store
        .select_bundles(combined_filter)
        .await
        .expect("Failed to select bundles with combined filter");
    assert_eq!(
        combined_filtered.len(),
        2,
        "Should return 2 bundles (bundle2: block=200 and timestamp range 1500-2500 contains 2000; bundle4: block=0 matches all blocks and timestamp range 500-3000 contains 2000)"
    );
    assert_eq!(combined_filtered[0].bundle.block_number, 200);

    let no_match_filter = BundleFilter::new().valid_for_block(999);
    let no_matches = harness
        .data_store
        .select_bundles(no_match_filter)
        .await
        .expect("Failed to select bundles with no match filter");
    assert_eq!(
        no_matches.len(),
        1,
        "Should return 1 bundle for non-existent block (bundle4 with block 0 is valid for all blocks)"
    );

    Ok(())
}

#[tokio::test]
async fn cancel_bundle_workflow() -> eyre::Result<()> {
    let harness = setup_datastore().await?;

    let bundle1 = create_test_bundle(100, Some(1000), Some(2000))?;
    let bundle2 = create_test_bundle(200, Some(1500), Some(2500))?;

    let bundle1_id = harness
        .data_store
        .insert_bundle(bundle1)
        .await
        .expect("Failed to insert bundle1");
    let bundle2_id = harness
        .data_store
        .insert_bundle(bundle2)
        .await
        .expect("Failed to insert bundle2");

    let retrieved_bundle1 = harness
        .data_store
        .get_bundle(bundle1_id)
        .await
        .expect("Failed to get bundle1");
    assert!(
        retrieved_bundle1.is_some(),
        "Bundle1 should exist before cancellation"
    );

    let retrieved_bundle2 = harness
        .data_store
        .get_bundle(bundle2_id)
        .await
        .expect("Failed to get bundle2");
    assert!(
        retrieved_bundle2.is_some(),
        "Bundle2 should exist before cancellation"
    );

    harness
        .data_store
        .cancel_bundle(bundle1_id)
        .await
        .expect("Failed to cancel bundle1");

    let cancelled_bundle1 = harness
        .data_store
        .get_bundle(bundle1_id)
        .await
        .expect("Failed to get bundle1 after cancellation");
    assert!(
        cancelled_bundle1.is_none(),
        "Bundle1 should not exist after cancellation"
    );

    let still_exists_bundle2 = harness
        .data_store
        .get_bundle(bundle2_id)
        .await
        .expect("Failed to get bundle2 after bundle1 cancellation");
    assert!(
        still_exists_bundle2.is_some(),
        "Bundle2 should still exist after bundle1 cancellation"
    );

    Ok(())
}

#[tokio::test]
async fn update_bundles_state() -> eyre::Result<()> {
    let harness = setup_datastore().await?;

    let bundle1 = create_test_bundle(100, Some(1000), Some(2000))?;
    let bundle2 = create_test_bundle(200, Some(1500), Some(2500))?;
    let bundle3 = create_test_bundle(300, None, None)?;

    let bundle1_id = harness
        .data_store
        .insert_bundle(bundle1)
        .await
        .expect("Failed to insert bundle1");
    let bundle2_id = harness
        .data_store
        .insert_bundle(bundle2)
        .await
        .expect("Failed to insert bundle2");
    let bundle3_id = harness
        .data_store
        .insert_bundle(bundle3)
        .await
        .expect("Failed to insert bundle3");

    let bundle1_metadata = harness
        .data_store
        .get_bundle(bundle1_id)
        .await
        .expect("Failed to get bundle1")
        .unwrap();
    let bundle2_metadata = harness
        .data_store
        .get_bundle(bundle2_id)
        .await
        .expect("Failed to get bundle2")
        .unwrap();
    let bundle3_metadata = harness
        .data_store
        .get_bundle(bundle3_id)
        .await
        .expect("Failed to get bundle3")
        .unwrap();

    assert!(matches!(bundle1_metadata.state, BundleState::Ready));
    assert!(matches!(bundle2_metadata.state, BundleState::Ready));
    assert!(matches!(bundle3_metadata.state, BundleState::Ready));

    let bundle1_initial_timestamp = bundle1_metadata.state_changed_at;
    let bundle2_initial_timestamp = bundle2_metadata.state_changed_at;

    let uuids = vec![bundle1_id, bundle2_id];
    let updated_uuids = harness
        .data_store
        .update_bundles_state(
            uuids.clone(),
            BundleState::Ready,
            BundleState::IncludedInBlock,
        )
        .await
        .expect("Failed to update bundle states");

    assert_eq!(updated_uuids.len(), 2, "Should update exactly 2 bundles");
    assert!(
        updated_uuids.contains(&bundle1_id),
        "Should contain bundle1_id"
    );
    assert!(
        updated_uuids.contains(&bundle2_id),
        "Should contain bundle2_id"
    );

    let updated_bundle1 = harness
        .data_store
        .get_bundle(bundle1_id)
        .await
        .expect("Failed to get updated bundle1")
        .unwrap();
    let updated_bundle2 = harness
        .data_store
        .get_bundle(bundle2_id)
        .await
        .expect("Failed to get updated bundle2")
        .unwrap();
    let unchanged_bundle3 = harness
        .data_store
        .get_bundle(bundle3_id)
        .await
        .expect("Failed to get unchanged bundle3")
        .unwrap();

    assert!(matches!(
        updated_bundle1.state,
        BundleState::IncludedInBlock
    ));
    assert!(matches!(
        updated_bundle2.state,
        BundleState::IncludedInBlock
    ));
    assert!(matches!(unchanged_bundle3.state, BundleState::Ready));

    assert!(
        updated_bundle1.state_changed_at > bundle1_initial_timestamp,
        "Bundle1 state_changed_at should be updated"
    );
    assert!(
        updated_bundle2.state_changed_at > bundle2_initial_timestamp,
        "Bundle2 state_changed_at should be updated"
    );

    let no_update_uuids = harness
        .data_store
        .update_bundles_state(
            vec![bundle3_id],
            BundleState::IncludedInFlashblock,
            BundleState::IncludedInBlock,
        )
        .await
        .expect("Failed to call update_bundles_state");

    assert_eq!(
        no_update_uuids.len(),
        0,
        "Should not update any bundles when prev_state doesn't match"
    );

    let still_unchanged = harness
        .data_store
        .get_bundle(bundle3_id)
        .await
        .expect("Failed to get bundle1 after no-op update")
        .unwrap();
    assert!(matches!(still_unchanged.state, BundleState::Ready));

    Ok(())
}
