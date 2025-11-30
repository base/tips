use alloy_primitives::{Address, U256, keccak256};
use alloy_provider::{Provider, RootProvider};
use anyhow::{Context, Result};
use op_alloy_network::Optimism;
use tips_system_tests::client::TipsRpcClient;
use tips_system_tests::fixtures::{
    create_funded_signer, create_optimism_provider, create_signed_transaction,
};
use tokio::time::{Duration, sleep};

/// Get the URL for integration tests against the production ingress service
/// This requires the full SETUP.md infrastructure to be running:
/// - TIPS ingress service (running on port 8080 via `just start-all`)
/// - builder-playground (on danyal/base-overlay branch, provides L2 node on port 8547)
/// - op-rbuilder (running on port 4444)
/// - Kafka (on port 9092)
fn get_integration_test_url() -> String {
    std::env::var("INGRESS_URL").unwrap_or_else(|_| "http://localhost:8080".to_string())
}

/// Poll the sequencer for a transaction receipt, retrying until found or timeout
async fn wait_for_receipt(
    sequencer_provider: &RootProvider<Optimism>,
    tx_hash: alloy_primitives::TxHash,
    timeout_secs: u64,
) -> Result<()> {
    let start = tokio::time::Instant::now();
    let timeout = Duration::from_secs(timeout_secs);

    loop {
        if start.elapsed() > timeout {
            anyhow::bail!(
                "Timeout waiting for transaction receipt after {}s",
                timeout_secs
            );
        }

        match sequencer_provider.get_transaction_receipt(tx_hash).await {
            Ok(Some(_receipt)) => {
                return Ok(());
            }
            Ok(None) => {
                sleep(Duration::from_millis(500)).await;
            }
            Err(_) => {
                sleep(Duration::from_millis(500)).await;
            }
        }
    }
}

#[tokio::test]
async fn test_rpc_client_instantiation() -> Result<()> {
    if std::env::var("INTEGRATION_TESTS").is_err() {
        eprintln!(
            "Skipping integration tests (set INTEGRATION_TESTS=1 and ensure SETUP.md infrastructure is running)"
        );
        return Ok(());
    }

    let url = get_integration_test_url();
    let provider = create_optimism_provider(&url)?;
    let _client = TipsRpcClient::new(provider);
    Ok(())
}

#[tokio::test]
async fn test_send_valid_transaction() -> Result<()> {
    if std::env::var("INTEGRATION_TESTS").is_err() {
        eprintln!(
            "Skipping integration tests (set INTEGRATION_TESTS=1 and ensure SETUP.md infrastructure is running)"
        );
        return Ok(());
    }

    let url = get_integration_test_url();
    let provider = create_optimism_provider(&url)?;
    let client = TipsRpcClient::new(provider);
    let signer = create_funded_signer();

    let sequencer_url =
        std::env::var("SEQUENCER_URL").unwrap_or_else(|_| "http://localhost:8547".to_string());
    let sequencer_provider = create_optimism_provider(&sequencer_url)?;
    let nonce = sequencer_provider
        .get_transaction_count(signer.address())
        .await?;

    let to = Address::from([0x11; 20]);
    let value = U256::from(1000);
    let gas_limit = 21000;
    let gas_price = 1_000_000_000;

    let signed_tx = create_signed_transaction(&signer, to, value, nonce, gas_limit, gas_price)?;

    let tx_hash = client
        .send_raw_transaction(signed_tx)
        .await
        .context("Failed to send transaction to TIPS")?;

    wait_for_receipt(&sequencer_provider, tx_hash, 60)
        .await
        .context("Transaction was not included in a block")?;

    Ok(())
}

#[tokio::test]
async fn test_send_bundle_with_valid_transaction() -> Result<()> {
    if std::env::var("INTEGRATION_TESTS").is_err() {
        eprintln!(
            "Skipping integration tests (set INTEGRATION_TESTS=1 and ensure SETUP.md infrastructure is running)"
        );
        return Ok(());
    }

    use tips_core::Bundle;

    let url = get_integration_test_url();
    let provider = create_optimism_provider(&url)?;
    let client = TipsRpcClient::new(provider);
    let signer = create_funded_signer();

    let sequencer_url =
        std::env::var("SEQUENCER_URL").unwrap_or_else(|_| "http://localhost:8547".to_string());
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

    let bundle_hash = client
        .send_bundle(bundle)
        .await
        .context("Failed to send bundle to TIPS")?;

    assert!(
        !bundle_hash.bundle_hash.is_zero(),
        "Bundle hash should not be zero"
    );

    wait_for_receipt(&sequencer_provider, tx_hash.into(), 60)
        .await
        .context("Bundle transaction was not included in a block")?;

    Ok(())
}

#[tokio::test]
async fn test_send_bundle_with_replacement_uuid() -> Result<()> {
    if std::env::var("INTEGRATION_TESTS").is_err() {
        eprintln!(
            "Skipping integration tests (set INTEGRATION_TESTS=1 and ensure SETUP.md infrastructure is running)"
        );
        return Ok(());
    }

    use tips_core::Bundle;
    use uuid::Uuid;

    let url = get_integration_test_url();
    let provider = create_optimism_provider(&url)?;
    let client = TipsRpcClient::new(provider);
    let signer = create_funded_signer();

    let sequencer_url =
        std::env::var("SEQUENCER_URL").unwrap_or_else(|_| "http://localhost:8547".to_string());
    let sequencer_provider = create_optimism_provider(&sequencer_url)?;
    let nonce = sequencer_provider
        .get_transaction_count(signer.address())
        .await?;

    let signed_tx = create_signed_transaction(
        &signer,
        Address::from([0x22; 20]),
        U256::from(2000),
        nonce,
        21000,
        1_000_000_000,
    )?;

    let tx_hash = keccak256(&signed_tx);

    let replacement_uuid = Uuid::new_v4();

    let bundle = Bundle {
        txs: vec![signed_tx],
        block_number: 1,
        replacement_uuid: Some(replacement_uuid.to_string()),
        min_timestamp: None,
        max_timestamp: None,
        reverting_tx_hashes: vec![tx_hash],
        dropping_tx_hashes: vec![],
        flashblock_number_min: None,
        flashblock_number_max: None,
    };

    let _bundle_hash = client
        .send_bundle(bundle)
        .await
        .context("Failed to send bundle with UUID to TIPS")?;

    wait_for_receipt(&sequencer_provider, tx_hash.into(), 60)
        .await
        .context("Bundle transaction was not included in a block")?;

    Ok(())
}

#[tokio::test]
async fn test_send_bundle_with_multiple_transactions() -> Result<()> {
    if std::env::var("INTEGRATION_TESTS").is_err() {
        eprintln!(
            "Skipping integration tests (set INTEGRATION_TESTS=1 and ensure SETUP.md infrastructure is running)"
        );
        return Ok(());
    }

    use tips_core::Bundle;

    let url = get_integration_test_url();
    let provider = create_optimism_provider(&url)?;
    let client = TipsRpcClient::new(provider);
    let signer = create_funded_signer();

    let sequencer_url =
        std::env::var("SEQUENCER_URL").unwrap_or_else(|_| "http://localhost:8547".to_string());
    let sequencer_provider = create_optimism_provider(&sequencer_url)?;
    let nonce = sequencer_provider
        .get_transaction_count(signer.address())
        .await?;

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

    let bundle_hash = client
        .send_bundle(bundle)
        .await
        .context("Failed to send multi-transaction bundle to TIPS")?;

    assert!(!bundle_hash.bundle_hash.is_zero());

    wait_for_receipt(&sequencer_provider, tx1_hash.into(), 60)
        .await
        .context("First transaction was not included in a block")?;

    wait_for_receipt(&sequencer_provider, tx2_hash.into(), 60)
        .await
        .context("Second transaction was not included in a block")?;

    wait_for_receipt(&sequencer_provider, tx3_hash.into(), 60)
        .await
        .context("Third transaction was not included in a block")?;

    Ok(())
}
