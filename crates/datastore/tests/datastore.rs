use alloy_primitives::{
    Address, Bytes, StorageKey, StorageValue, TxHash, U256, address, b256, bytes,
};
use alloy_rpc_types_mev::EthSendBundle;
use sqlx::PgPool;
use std::collections::HashMap;
use testcontainers_modules::{
    postgres,
    testcontainers::{ContainerAsync, runners::AsyncRunner},
};
use tips_datastore::postgres::{BundleFilter, BundleState, StateDiff};
use tips_datastore::{BundleDatastore, PostgresDatastore};
use uuid::Uuid;

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

fn create_test_state_diff() -> StateDiff {
    let mut state_diff = HashMap::new();
    
    // Create test account address
    let account1: Address = "0x742d35cc6635c0532925a3b8d40b33dd33ad7309".parse().unwrap();
    let account2: Address = "0x24ae36512421f1d9f6e074f00ff5b8393f5dd925".parse().unwrap();
    
    // Create storage mappings for account1
    let mut account1_storage = HashMap::new();
    account1_storage.insert(
        StorageKey::ZERO,
        StorageValue::from(U256::from(1)),
    );
    account1_storage.insert(
        StorageKey::from(U256::from(1)),
        StorageValue::from(U256::from(2)),
    );
    
    // Create storage mappings for account2
    let mut account2_storage = HashMap::new();
    account2_storage.insert(
        StorageKey::from(U256::from(3)),
        StorageValue::from(U256::from(4)),
    );
    
    state_diff.insert(account1, account1_storage);
    state_diff.insert(account2, account2_storage);
    
    state_diff
}

fn create_empty_state_diff() -> StateDiff {
    HashMap::new()
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
async fn insert_and_get_simulation() -> eyre::Result<()> {
    let harness = setup_datastore().await?;
    
    // First create a bundle to link the simulation to
    let test_bundle = create_test_bundle(12345, Some(1640995200), Some(1640995260))?;
    let bundle_id = harness.data_store.insert_bundle(test_bundle).await
        .map_err(|e| eyre::eyre!(e))?;
    
    // Create simulation data
    let block_number = 18500000u64;
    let block_hash = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string();
    let execution_time_us = 250000u64;
    let gas_used = 21000u64;
    let state_diff = create_test_state_diff();
    
    // Insert simulation
    let simulation_id = harness.data_store.insert_simulation(
        bundle_id,
        block_number,
        block_hash.clone(),
        execution_time_us,
        gas_used,
        state_diff.clone(),
    ).await.map_err(|e| eyre::eyre!(e))?;
    
    // Retrieve simulation
    let retrieved_simulation = harness.data_store.get_simulation(simulation_id).await
        .map_err(|e| eyre::eyre!(e))?;
    assert!(retrieved_simulation.is_some(), "Simulation should be found");
    
    let simulation = retrieved_simulation.unwrap();
    assert_eq!(simulation.id, simulation_id);
    assert_eq!(simulation.bundle_id, bundle_id);
    assert_eq!(simulation.block_number, block_number);
    assert_eq!(simulation.block_hash, block_hash);
    assert_eq!(simulation.execution_time_us, execution_time_us);
    assert_eq!(simulation.gas_used, gas_used);
    assert_eq!(simulation.state_diff.len(), state_diff.len());
    
    // Verify state diff content
    for (account, expected_storage) in &state_diff {
        let actual_storage = simulation.state_diff.get(account)
            .expect("Account should exist in state diff");
        assert_eq!(actual_storage.len(), expected_storage.len());
        for (slot, expected_value) in expected_storage {
            let actual_value = actual_storage.get(slot)
                .expect("Storage slot should exist");
            assert_eq!(actual_value, expected_value);
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn simulation_with_empty_state_diff() -> eyre::Result<()> {
    let harness = setup_datastore().await?;
    
    // Create a bundle
    let test_bundle = create_test_bundle(12345, None, None)?;
    let bundle_id = harness.data_store.insert_bundle(test_bundle).await
        .map_err(|e| eyre::eyre!(e))?;
    
    // Create simulation with empty state diff
    let simulation_id = harness.data_store.insert_simulation(
        bundle_id,
        18500000,
        "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890".to_string(),
        100000,
        15000,
        create_empty_state_diff(),
    ).await.map_err(|e| eyre::eyre!(e))?;
    
    // Retrieve and verify
    let simulation = harness.data_store.get_simulation(simulation_id).await
        .map_err(|e| eyre::eyre!(e))?
        .expect("Simulation should exist");
    
    assert!(simulation.state_diff.is_empty(), "State diff should be empty");
    
    Ok(())
}

#[tokio::test]
async fn multiple_simulations_latest_selection() -> eyre::Result<()> {
    let harness = setup_datastore().await?;
    
    // Create a single bundle
    let test_bundle = create_test_bundle(12345, Some(1000), Some(2000))?;
    let bundle_id = harness.data_store.insert_bundle(test_bundle).await
        .map_err(|e| eyre::eyre!(e))?;
    
    // Insert multiple simulations with sequential block numbers
    let base_block = 18500000u64;
    let mut simulation_ids = Vec::new();
    
    for i in 0..5 {
        let block_number = base_block + i;
        let block_hash = format!("0x{:064x}", block_number); // Create unique block hash
        let execution_time = 100000 + (i * 10000); // Varying execution times
        let gas_used = 21000 + (i * 1000); // Varying gas usage
        
        let simulation_id = harness.data_store.insert_simulation(
            bundle_id,
            block_number,
            block_hash,
            execution_time,
            gas_used,
            if i % 2 == 0 { create_test_state_diff() } else { create_empty_state_diff() },
        ).await.map_err(|e| eyre::eyre!(e))?;
        
        simulation_ids.push((simulation_id, block_number, execution_time, gas_used));
    }
    
    // Query for bundles with latest simulation
    let results = harness.data_store.select_bundles_with_latest_simulation(
        BundleFilter::new()
    ).await.map_err(|e| eyre::eyre!(e))?;
    
    // Should return exactly one bundle
    assert_eq!(results.len(), 1, "Should return exactly one bundle");
    
    let bundle_with_sim = &results[0];
    let latest_sim = &bundle_with_sim.latest_simulation;
    
    // Verify it's the latest simulation (highest block number)
    let expected_latest_block = base_block + 4; // Last iteration was i=4
    assert_eq!(latest_sim.block_number, expected_latest_block, "Should return simulation with highest block number");
    assert_eq!(latest_sim.bundle_id, bundle_id, "Should reference correct bundle");
    
    // Verify the execution time and gas used match the latest simulation
    let expected_execution_time = 100000 + (4 * 10000); // i=4
    let expected_gas_used = 21000 + (4 * 1000); // i=4
    assert_eq!(latest_sim.execution_time_us, expected_execution_time, "Execution time should match latest simulation");
    assert_eq!(latest_sim.gas_used, expected_gas_used, "Gas used should match latest simulation");
    
    // Verify the latest simulation has the expected state diff (should be non-empty since i=4 is even)
    assert!(!latest_sim.state_diff.is_empty(), "Latest simulation should have non-empty state diff");
    
    // Verify that we can still retrieve all individual simulations
    for (sim_id, block_num, exec_time, gas) in &simulation_ids {
        let individual_sim = harness.data_store.get_simulation(*sim_id).await
            .map_err(|e| eyre::eyre!(e))?
            .expect("Individual simulation should exist");
        
        assert_eq!(individual_sim.block_number, *block_num);
        assert_eq!(individual_sim.execution_time_us, *exec_time);
        assert_eq!(individual_sim.gas_used, *gas);
    }
    
    Ok(())
}

#[tokio::test]
async fn select_bundles_with_latest_simulation() -> eyre::Result<()> {
    let harness = setup_datastore().await?;
    
    // Create three bundles
    let bundle1 = create_test_bundle(100, Some(1000), Some(2000))?;
    let bundle2 = create_test_bundle(200, Some(1500), Some(2500))?;
    let bundle3 = create_test_bundle(300, None, None)?;
    
    let bundle1_id = harness.data_store.insert_bundle(bundle1).await
        .map_err(|e| eyre::eyre!(e))?;
    let bundle2_id = harness.data_store.insert_bundle(bundle2).await
        .map_err(|e| eyre::eyre!(e))?;
    let _bundle3_id = harness.data_store.insert_bundle(bundle3).await
        .map_err(|e| eyre::eyre!(e))?;
    
    // Add multiple simulations for bundle1 (to test "latest" logic)
    harness.data_store.insert_simulation(
        bundle1_id,
        18500000,
        "0x1111111111111111111111111111111111111111111111111111111111111111".to_string(),
        100000,
        21000,
        create_test_state_diff(),
    ).await.map_err(|e| eyre::eyre!(e))?;
    
    let latest_sim1_id = harness.data_store.insert_simulation(
        bundle1_id,
        18500001, // Higher block number = later
        "0x2222222222222222222222222222222222222222222222222222222222222222".to_string(),
        120000,
        22000,
        create_empty_state_diff(),
    ).await.map_err(|e| eyre::eyre!(e))?;
    
    // Add one simulation for bundle2
    let sim2_id = harness.data_store.insert_simulation(
        bundle2_id,
        18500002,
        "0x3333333333333333333333333333333333333333333333333333333333333333".to_string(),
        90000,
        19000,
        create_test_state_diff(),
    ).await.map_err(|e| eyre::eyre!(e))?;
    
    // Bundle3 has no simulations
    
    // Query bundles with latest simulation (no filter)
    let results = harness.data_store.select_bundles_with_latest_simulation(
        BundleFilter::new()
    ).await.map_err(|e| eyre::eyre!(e))?;
    
    // Should return 2 bundles (bundle1 and bundle2), sorted by minimum_base_fee DESC
    assert_eq!(results.len(), 2, "Should return 2 bundles that have simulations");
    
    // Verify the results contain the correct bundles and latest simulations
    let bundle1_result = results.iter().find(|r| r.bundle_with_metadata.bundle.block_number == 100);
    let bundle2_result = results.iter().find(|r| r.bundle_with_metadata.bundle.block_number == 200);
    
    assert!(bundle1_result.is_some(), "Bundle1 should be in results");
    assert!(bundle2_result.is_some(), "Bundle2 should be in results");
    
    let bundle1_result = bundle1_result.unwrap();
    let bundle2_result = bundle2_result.unwrap();
    
    // Check that bundle1 has the latest simulation (block 18500001)
    assert_eq!(bundle1_result.latest_simulation.id, latest_sim1_id);
    assert_eq!(bundle1_result.latest_simulation.block_number, 18500001);
    assert_eq!(bundle1_result.latest_simulation.gas_used, 22000);
    
    // Check that bundle2 has its simulation
    assert_eq!(bundle2_result.latest_simulation.id, sim2_id);
    assert_eq!(bundle2_result.latest_simulation.block_number, 18500002);
    assert_eq!(bundle2_result.latest_simulation.gas_used, 19000);
    
    Ok(())
}

#[tokio::test]
async fn select_bundles_with_latest_simulation_filtered() -> eyre::Result<()> {
    let harness = setup_datastore().await?;
    
    // Create bundles with different criteria
    let bundle1 = create_test_bundle(100, Some(1000), Some(2000))?; // Valid for block 100, timestamp 1000-2000
    let bundle2 = create_test_bundle(200, Some(1500), Some(2500))?; // Valid for block 200, timestamp 1500-2500
    
    let bundle1_id = harness.data_store.insert_bundle(bundle1).await
        .map_err(|e| eyre::eyre!(e))?;
    let bundle2_id = harness.data_store.insert_bundle(bundle2).await
        .map_err(|e| eyre::eyre!(e))?;
    
    // Add simulations to both bundles
    harness.data_store.insert_simulation(
        bundle1_id,
        18500000,
        "0x1111111111111111111111111111111111111111111111111111111111111111".to_string(),
        100000,
        21000,
        create_test_state_diff(),
    ).await.map_err(|e| eyre::eyre!(e))?;
    
    harness.data_store.insert_simulation(
        bundle2_id,
        18500001,
        "0x2222222222222222222222222222222222222222222222222222222222222222".to_string(),
        120000,
        22000,
        create_empty_state_diff(),
    ).await.map_err(|e| eyre::eyre!(e))?;
    
    // Test filtering by block number
    let block_filter = BundleFilter::new().valid_for_block(200);
    let filtered_results = harness.data_store.select_bundles_with_latest_simulation(block_filter).await
        .map_err(|e| eyre::eyre!(e))?;
    
    assert_eq!(filtered_results.len(), 1, "Should return 1 bundle valid for block 200");
    assert_eq!(filtered_results[0].bundle_with_metadata.bundle.block_number, 200);
    
    // Test filtering by timestamp
    let timestamp_filter = BundleFilter::new().valid_for_timestamp(1200);
    let timestamp_results = harness.data_store.select_bundles_with_latest_simulation(timestamp_filter).await
        .map_err(|e| eyre::eyre!(e))?;
    
    assert_eq!(timestamp_results.len(), 1, "Should return 1 bundle valid for timestamp 1200");
    assert_eq!(timestamp_results[0].bundle_with_metadata.bundle.block_number, 100);
    
    Ok(())
}

#[tokio::test]
async fn get_nonexistent_simulation() -> eyre::Result<()> {
    let harness = setup_datastore().await?;
    
    // Try to get simulation that doesn't exist
    let fake_id = Uuid::new_v4();
    let result = harness.data_store.get_simulation(fake_id).await
        .map_err(|e| eyre::eyre!(e))?;
    
    assert!(result.is_none(), "Should return None for non-existent simulation");
    
    Ok(())
}
