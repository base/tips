//! Integration tests for TIPS ingress service RPC endpoints.
//!
//! ## What These Tests Verify
//!
//! 1. **RPC Connectivity** - Can connect to TIPS ingress service
//! 2. **Bundle Acceptance** - Valid bundles are accepted and return hashes
//! 3. **Transaction Acceptance** - Valid raw transactions are accepted
//! 4. **Error Handling** - Invalid requests return appropriate errors
//!
//! ## What Is NOT Tested Here
//!
//! - **Validation Logic** - Unit tested in `ingress-rpc/src/validation.rs`
//! - **Transaction Inclusion** - Builder responsibility, not TIPS
//! - **Bundle Updates/Cancellation** - Not supported (bundle-pool removed)
//! - **Bundle State Tracking** - No bundle pool to track state
//!
//! ## Test Environment Requirements
//!
//! Set `INTEGRATION_TESTS=1` and ensure infrastructure is running:
//! - TIPS ingress service (port 8080)
//! - Simulation provider with base_meterBundle support

#[path = "common/mod.rs"]
mod common;

use alloy_primitives::{Address, TxHash, U256, keccak256};
use alloy_provider::{Provider, RootProvider};
use anyhow::{Context, Result, bail};
use common::{
    kafka::{wait_for_audit_event, wait_for_ingress_bundle},
    s3::wait_for_bundle_history_event,
};
use op_alloy_network::Optimism;
use tips_audit::{storage::BundleHistoryEvent, types::BundleEvent};
use tips_core::{BundleExtensions, CancelBundle};
use tips_system_tests::client::TipsRpcClient;
use tips_system_tests::fixtures::{
    create_funded_signer, create_optimism_provider, create_signed_transaction,
};
use tokio::time::{Duration, Instant, sleep};

/// Get the URL for integration tests against the TIPS ingress service
fn get_integration_test_url() -> String {
    std::env::var("INGRESS_URL").unwrap_or_else(|_| "http://localhost:8080".to_string())
}

/// Get the URL for the sequencer (for fetching nonces)
fn get_sequencer_url() -> String {
    std::env::var("SEQUENCER_URL").unwrap_or_else(|_| "http://localhost:8547".to_string())
}

fn configured_tx_submission_modes() -> Vec<String> {
    std::env::var("TIPS_TEST_TX_SUBMISSION_METHOD")
        .or_else(|_| std::env::var("TIPS_INGRESS_TX_SUBMISSION_METHOD"))
        .unwrap_or_else(|_| "mempool".to_string())
        .split(',')
        .map(|mode| mode.trim().to_ascii_lowercase())
        .filter(|mode| !mode.is_empty())
        .collect()
}

fn tx_submission_includes_kafka() -> bool {
    configured_tx_submission_modes()
        .iter()
        .any(|mode| mode == "kafka")
}

fn tx_submission_includes_mempool() -> bool {
    configured_tx_submission_modes()
        .iter()
        .any(|mode| mode == "mempool")
}

async fn wait_for_transaction_seen(
    provider: &RootProvider<Optimism>,
    tx_hash: TxHash,
    timeout_secs: u64,
) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    loop {
        if Instant::now() >= deadline {
            bail!(
                "Timed out waiting for transaction {} to appear on the sequencer",
                tx_hash
            );
        }

        if provider
            .get_transaction_by_hash(tx_hash.into())
            .await?
            .is_some()
        {
            return Ok(());
        }

        sleep(Duration::from_millis(500)).await;
    }
}

#[tokio::test]
async fn test_client_can_connect_to_tips() -> Result<()> {
    if std::env::var("INTEGRATION_TESTS").is_err() {
        eprintln!(
            "Skipping integration tests (set INTEGRATION_TESTS=1 and ensure TIPS infrastructure is running)"
        );
        return Ok(());
    }

    let url = get_integration_test_url();
    let provider = create_optimism_provider(&url)?;
    let _client = TipsRpcClient::new(provider);
    Ok(())
}

#[tokio::test]
async fn test_send_raw_transaction_accepted() -> Result<()> {
    if std::env::var("INTEGRATION_TESTS").is_err() {
        eprintln!(
            "Skipping integration tests (set INTEGRATION_TESTS=1 and ensure TIPS infrastructure is running)"
        );
        return Ok(());
    }

    let url = get_integration_test_url();
    let provider = create_optimism_provider(&url)?;
    let client = TipsRpcClient::new(provider);
    let signer = create_funded_signer();

    // Get nonce from sequencer
    let sequencer_url = get_sequencer_url();
    let sequencer_provider = create_optimism_provider(&sequencer_url)?;
    let nonce = sequencer_provider
        .get_transaction_count(signer.address())
        .await?;

    let to = Address::from([0x11; 20]);
    let value = U256::from(1000);
    let gas_limit = 21000;
    let gas_price = 1_000_000_000;

    let signed_tx = create_signed_transaction(&signer, to, value, nonce, gas_limit, gas_price)?;

    // Send transaction to TIPS
    let tx_hash = client
        .send_raw_transaction(signed_tx)
        .await
        .context("Failed to send transaction to TIPS")?;

    // Verify TIPS accepted the transaction and returned a hash
    assert!(!tx_hash.is_zero(), "Transaction hash should not be zero");

    // If Kafka submission is enabled, ensure the transaction bundle is enqueued
    if tx_submission_includes_kafka() {
        let mut concatenated = Vec::new();
        concatenated.extend_from_slice(tx_hash.as_slice());
        let expected_bundle_hash = keccak256(&concatenated);

        wait_for_ingress_bundle(&expected_bundle_hash)
            .await
            .context("Failed to observe raw transaction bundle on Kafka")?;
    }

    // If mempool submission is enabled, ensure the sequencer sees the transaction
    if tx_submission_includes_mempool() {
        wait_for_transaction_seen(&sequencer_provider, tx_hash, 30)
            .await
            .context("Transaction never appeared on sequencer")?;
    }

    Ok(())
}

#[tokio::test]
async fn test_send_bundle_accepted() -> Result<()> {
    if std::env::var("INTEGRATION_TESTS").is_err() {
        eprintln!(
            "Skipping integration tests (set INTEGRATION_TESTS=1 and ensure TIPS infrastructure is running)"
        );
        return Ok(());
    }

    use tips_core::Bundle;

    let url = get_integration_test_url();
    let provider = create_optimism_provider(&url)?;
    let client = TipsRpcClient::new(provider);
    let signer = create_funded_signer();

    // Get nonce from sequencer
    let sequencer_url = get_sequencer_url();
    let sequencer_provider = create_optimism_provider(&sequencer_url)?;
    let nonce = sequencer_provider
        .get_transaction_count(signer.address())
        .await?;

    let to = Address::from([0x11; 20]);
    let value = U256::from(1000);
    let gas_limit = 21000;
    let gas_price = 1_000_000_000;

    let signed_tx = create_signed_transaction(&signer, to, value, nonce, gas_limit, gas_price)?;
    let tx_hash = keccak256(&signed_tx);

    let bundle = Bundle {
        txs: vec![signed_tx],
        block_number: 1,
        min_timestamp: None,
        max_timestamp: None,
        reverting_tx_hashes: vec![tx_hash],
        replacement_uuid: None,
        dropping_tx_hashes: vec![],
        flashblock_number_min: None,
        flashblock_number_max: None,
    };

    // Send bundle to TIPS
    let bundle_hash = client
        .send_bundle(bundle)
        .await
        .context("Failed to send bundle to TIPS")?;

    // Verify TIPS accepted the bundle and returned a hash
    assert!(
        !bundle_hash.bundle_hash.is_zero(),
        "Bundle hash should not be zero"
    );

    // Verify bundle hash is calculated correctly: keccak256(concat(tx_hashes))
    let mut concatenated = Vec::new();
    concatenated.extend_from_slice(tx_hash.as_slice());
    let expected_bundle_hash = keccak256(&concatenated);
    assert_eq!(
        bundle_hash.bundle_hash, expected_bundle_hash,
        "Bundle hash should match keccak256(tx_hash)"
    );

    // Verify the bundle was published to Kafka and matches expectations
    let accepted_bundle = wait_for_ingress_bundle(&bundle_hash.bundle_hash)
        .await
        .context("Failed to read bundle from Kafka")?;
    assert_eq!(
        accepted_bundle.bundle_hash(),
        bundle_hash.bundle_hash,
        "Kafka bundle hash should match response"
    );

    // Verify audit channel emitted a Received event for this bundle
    let audit_event = wait_for_audit_event(*accepted_bundle.uuid(), |event| {
        matches!(event, BundleEvent::Received { .. })
    })
    .await
    .context("Failed to read audit event from Kafka")?;
    match audit_event {
        BundleEvent::Received { bundle, .. } => {
            assert_eq!(
                bundle.bundle_hash(),
                bundle_hash.bundle_hash,
                "Audit event bundle hash should match response"
            );
        }
        other => panic!("Expected Received audit event, got {:?}", other),
    }

    // Verify bundle history persisted to S3
    let s3_event = wait_for_bundle_history_event(*accepted_bundle.uuid(), |event| {
        matches!(event, BundleHistoryEvent::Received { .. })
    })
    .await
    .context("Failed to read bundle history from S3")?;
    if let BundleHistoryEvent::Received { bundle, .. } = s3_event {
        assert_eq!(
            bundle.bundle_hash(),
            bundle_hash.bundle_hash,
            "S3 history bundle hash should match response"
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_send_bundle_with_three_transactions() -> Result<()> {
    if std::env::var("INTEGRATION_TESTS").is_err() {
        eprintln!(
            "Skipping integration tests (set INTEGRATION_TESTS=1 and ensure TIPS infrastructure is running)"
        );
        return Ok(());
    }

    use tips_core::Bundle;

    let url = get_integration_test_url();
    let provider = create_optimism_provider(&url)?;
    let client = TipsRpcClient::new(provider);
    let signer = create_funded_signer();

    // Get nonce from sequencer
    let sequencer_url = get_sequencer_url();
    let sequencer_provider = create_optimism_provider(&sequencer_url)?;
    let nonce = sequencer_provider
        .get_transaction_count(signer.address())
        .await?;

    // Create 3 transactions (the maximum allowed)
    let tx1 = create_signed_transaction(
        &signer,
        Address::from([0x33; 20]),
        U256::from(1000),
        nonce,
        21000,
        1_000_000_000,
    )?;

    let tx2 = create_signed_transaction(
        &signer,
        Address::from([0x44; 20]),
        U256::from(2000),
        nonce + 1,
        21000,
        1_000_000_000,
    )?;

    let tx3 = create_signed_transaction(
        &signer,
        Address::from([0x55; 20]),
        U256::from(3000),
        nonce + 2,
        21000,
        1_000_000_000,
    )?;

    let tx1_hash = keccak256(&tx1);
    let tx2_hash = keccak256(&tx2);
    let tx3_hash = keccak256(&tx3);

    let bundle = Bundle {
        txs: vec![tx1, tx2, tx3],
        block_number: 1,
        min_timestamp: None,
        max_timestamp: None,
        reverting_tx_hashes: vec![tx1_hash, tx2_hash, tx3_hash],
        replacement_uuid: None,
        dropping_tx_hashes: vec![],
        flashblock_number_min: None,
        flashblock_number_max: None,
    };

    // Send bundle with 3 transactions to TIPS
    let bundle_hash = client
        .send_bundle(bundle)
        .await
        .context("Failed to send multi-transaction bundle to TIPS")?;

    // Verify TIPS accepted the bundle and returned a hash
    assert!(
        !bundle_hash.bundle_hash.is_zero(),
        "Bundle hash should not be zero"
    );

    // Verify bundle hash is calculated correctly: keccak256(concat(all tx_hashes))
    let mut concatenated = Vec::new();
    concatenated.extend_from_slice(tx1_hash.as_slice());
    concatenated.extend_from_slice(tx2_hash.as_slice());
    concatenated.extend_from_slice(tx3_hash.as_slice());
    let expected_bundle_hash = keccak256(&concatenated);
    assert_eq!(
        bundle_hash.bundle_hash, expected_bundle_hash,
        "Bundle hash should match keccak256(concat(tx1_hash, tx2_hash, tx3_hash))"
    );

    // Verify bundle was published to Kafka
    let accepted_bundle = wait_for_ingress_bundle(&bundle_hash.bundle_hash)
        .await
        .context("Failed to read 3-tx bundle from Kafka")?;
    assert_eq!(
        accepted_bundle.bundle_hash(),
        bundle_hash.bundle_hash,
        "Kafka bundle hash should match response"
    );

    // Verify audit channel emitted a Received event
    let audit_event = wait_for_audit_event(*accepted_bundle.uuid(), |event| {
        matches!(event, BundleEvent::Received { .. })
    })
    .await
    .context("Failed to read audit event for 3-tx bundle")?;
    match audit_event {
        BundleEvent::Received { bundle, .. } => {
            assert_eq!(
                bundle.bundle_hash(),
                bundle_hash.bundle_hash,
                "Audit event bundle hash should match response"
            );
        }
        other => panic!("Expected Received audit event, got {:?}", other),
    }

    let s3_event = wait_for_bundle_history_event(*accepted_bundle.uuid(), |event| {
        matches!(event, BundleHistoryEvent::Received { .. })
    })
    .await
    .context("Failed to read 3-tx bundle history from S3")?;
    if let BundleHistoryEvent::Received { bundle, .. } = s3_event {
        assert_eq!(
            bundle.bundle_hash(),
            bundle_hash.bundle_hash,
            "S3 history bundle hash should match response"
        );
    }

    Ok(())
}

#[tokio::test]
#[ignore = "eth_cancelBundle is not yet implemented on the ingress server"]
async fn test_cancel_bundle_endpoint() -> Result<()> {
    if std::env::var("INTEGRATION_TESTS").is_err() {
        eprintln!(
            "Skipping integration tests (set INTEGRATION_TESTS=1 and ensure TIPS infrastructure is running)"
        );
        return Ok(());
    }

    use tips_core::Bundle;

    let url = get_integration_test_url();
    let provider = create_optimism_provider(&url)?;
    let client = TipsRpcClient::new(provider);
    let signer = create_funded_signer();

    // Get nonce from sequencer
    let sequencer_url = get_sequencer_url();
    let sequencer_provider = create_optimism_provider(&sequencer_url)?;
    let nonce = sequencer_provider
        .get_transaction_count(signer.address())
        .await?;

    let signed_tx = create_signed_transaction(
        &signer,
        Address::from([0x77; 20]),
        U256::from(1234),
        nonce,
        21000,
        1_000_000_000,
    )?;
    let tx_hash = keccak256(&signed_tx);

    let bundle = Bundle {
        txs: vec![signed_tx],
        block_number: 1,
        min_timestamp: None,
        max_timestamp: None,
        reverting_tx_hashes: vec![tx_hash],
        replacement_uuid: None,
        dropping_tx_hashes: vec![],
        flashblock_number_min: None,
        flashblock_number_max: None,
    };

    let bundle_hash = client
        .send_bundle(bundle)
        .await
        .context("Failed to send bundle before cancellation")?;

    // Fetch the accepted bundle from Kafka to get the UUID
    let accepted_bundle = wait_for_ingress_bundle(&bundle_hash.bundle_hash)
        .await
        .context("Failed to fetch bundle from Kafka before cancellation")?;
    let bundle_uuid = *accepted_bundle.uuid();

    // Issue cancelBundle RPC
    let cancel_request = CancelBundle {
        replacement_uuid: bundle_uuid.to_string(),
    };
    let _ = client
        .cancel_bundle(cancel_request)
        .await
        .context("Failed to call eth_cancelBundle")?;

    // Verify audit channel records the cancellation
    let audit_event = wait_for_audit_event(bundle_uuid, |event| {
        matches!(event, BundleEvent::Cancelled { .. })
    })
    .await
    .context("Failed to read cancellation audit event")?;

    assert!(
        matches!(audit_event, BundleEvent::Cancelled { .. }),
        "Expected a cancellation audit event"
    );

    let _ = wait_for_bundle_history_event(bundle_uuid, |event| {
        matches!(event, BundleHistoryEvent::Cancelled { .. })
    })
    .await
    .context("Failed to read cancellation bundle history from S3")?;

    Ok(())
}
