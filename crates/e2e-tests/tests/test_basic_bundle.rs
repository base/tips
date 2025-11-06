use alloy_primitives::{Address, Bytes, U256, keccak256};
use alloy_provider::{Provider, ProviderBuilder, RootProvider};
use anyhow::Result;
use op_alloy_network::Optimism;
use tips_e2e_tests::client::TipsRpcClient;
use tips_e2e_tests::fixtures::{create_funded_signer, create_signed_transaction};

/// Get the URL for integration tests against the production ingress service
/// This requires the full SETUP.md infrastructure to be running:
/// - TIPS ingress service (running on port 8080 via `just start-all`)
/// - builder-playground (on danyal/base-overlay branch, provides L2 node on port 8547)
/// - op-rbuilder (running on port 4444)
/// - Kafka (on port 9092)
fn get_integration_test_url() -> String {
    std::env::var("INGRESS_URL").unwrap_or_else(|_| "http://localhost:8080".to_string())
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
    let _client = TipsRpcClient::new(&url);
    Ok(())
}

#[tokio::test]
async fn test_send_raw_transaction_rejects_empty() -> Result<()> {
    if std::env::var("INTEGRATION_TESTS").is_err() {
        eprintln!(
            "Skipping integration tests (set INTEGRATION_TESTS=1 and ensure SETUP.md infrastructure is running)"
        );
        return Ok(());
    }

    let url = get_integration_test_url();
    let client = TipsRpcClient::new(&url);

    let empty_tx = Bytes::new();
    let result = client.send_raw_transaction(empty_tx).await;

    assert!(result.is_err(), "Empty transaction should be rejected");

    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("RPC error") || error_msg.contains("empty"),
        "Error should mention empty data or be an RPC error, got: {}",
        error_msg
    );

    Ok(())
}

#[tokio::test]
async fn test_send_raw_transaction_rejects_invalid() -> Result<()> {
    if std::env::var("INTEGRATION_TESTS").is_err() {
        eprintln!(
            "Skipping integration tests (set INTEGRATION_TESTS=1 and ensure SETUP.md infrastructure is running)"
        );
        return Ok(());
    }

    let url = get_integration_test_url();
    let client = TipsRpcClient::new(&url);

    let invalid_tx = Bytes::from(vec![0x01, 0x02, 0x03]);
    let result = client.send_raw_transaction(invalid_tx).await;

    assert!(result.is_err(), "Invalid transaction should be rejected");

    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("RPC error")
            || error_msg.contains("decode")
            || error_msg.contains("Failed"),
        "Error should mention decoding failure, got: {}",
        error_msg
    );

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
    let client = TipsRpcClient::new(&url);
    let signer = create_funded_signer();

    // Fetch current nonce from L2 node
    let sequencer_url =
        std::env::var("SEQUENCER_URL").unwrap_or_else(|_| "http://localhost:8547".to_string());
    let sequencer_provider: RootProvider<Optimism> = ProviderBuilder::new()
        .disable_recommended_fillers()
        .network::<Optimism>()
        .connect_http(sequencer_url.parse()?);
    let nonce = sequencer_provider
        .get_transaction_count(signer.address())
        .await?;

    let to = Address::from([0x11; 20]);
    let value = U256::from(1000);
    let gas_limit = 21000;
    let gas_price = 1_000_000_000;

    let signed_tx =
        create_signed_transaction(&signer, to, value, nonce, gas_limit, gas_price).await?;

    let result = client.send_raw_transaction(signed_tx).await;

    result.map(|_tx_hash| ())
}

#[tokio::test]
async fn test_send_bundle_rejects_empty() -> Result<()> {
    if std::env::var("INTEGRATION_TESTS").is_err() {
        eprintln!(
            "Skipping integration tests (set INTEGRATION_TESTS=1 and ensure SETUP.md infrastructure is running)"
        );
        return Ok(());
    }

    use tips_core::Bundle;

    let url = get_integration_test_url();
    let client = TipsRpcClient::new(&url);

    let empty_bundle = Bundle {
        txs: vec![],
        block_number: 1,
        min_timestamp: None,
        max_timestamp: None,
        reverting_tx_hashes: vec![],
        replacement_uuid: None,
        dropping_tx_hashes: vec![],
        flashblock_number_min: None,
        flashblock_number_max: None,
    };

    let result = client.send_bundle(empty_bundle).await;

    // Empty bundles should be rejected
    assert!(result.is_err(), "Empty bundle should be rejected");

    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("RPC error")
            || error_msg.contains("empty")
            || error_msg.contains("validation"),
        "Error should mention validation failure, got: {}",
        error_msg
    );

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
    let client = TipsRpcClient::new(&url);
    let signer = create_funded_signer();

    // Fetch current nonce from L2 node
    let sequencer_url =
        std::env::var("SEQUENCER_URL").unwrap_or_else(|_| "http://localhost:8547".to_string());
    let sequencer_provider: RootProvider<Optimism> = ProviderBuilder::new()
        .disable_recommended_fillers()
        .network::<Optimism>()
        .connect_http(sequencer_url.parse()?);
    let nonce = sequencer_provider
        .get_transaction_count(signer.address())
        .await?;

    // Create a valid signed transaction
    let to = Address::from([0x11; 20]);
    let value = U256::from(1000);
    let gas_limit = 21000;
    let gas_price = 1_000_000_000;

    let signed_tx =
        create_signed_transaction(&signer, to, value, nonce, gas_limit, gas_price).await?;

    // Compute transaction hash for reverting_tx_hashes
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

    let bundle_hash = client.send_bundle(bundle).await?;

    println!(
        "Bundle submitted successfully! Hash: {:?}",
        bundle_hash.bundle_hash
    );
    assert!(
        !bundle_hash.bundle_hash.is_zero(),
        "Bundle hash should not be zero"
    );

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
    let client = TipsRpcClient::new(&url);
    let signer = create_funded_signer();

    // Fetch current nonce from L2 node
    let sequencer_url =
        std::env::var("SEQUENCER_URL").unwrap_or_else(|_| "http://localhost:8547".to_string());
    let sequencer_provider: RootProvider<Optimism> = ProviderBuilder::new()
        .disable_recommended_fillers()
        .network::<Optimism>()
        .connect_http(sequencer_url.parse()?);
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
    )
    .await?;

    // Compute transaction hash for reverting_tx_hashes
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

    let bundle_hash = client.send_bundle(bundle).await?;

    println!(
        "Bundle with UUID {} submitted! Hash: {:?}",
        replacement_uuid, bundle_hash.bundle_hash
    );

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
    let client = TipsRpcClient::new(&url);
    let signer = create_funded_signer();

    // Fetch current nonce from L2 node
    let sequencer_url =
        std::env::var("SEQUENCER_URL").unwrap_or_else(|_| "http://localhost:8547".to_string());
    let sequencer_provider: RootProvider<Optimism> = ProviderBuilder::new()
        .disable_recommended_fillers()
        .network::<Optimism>()
        .connect_http(sequencer_url.parse()?);
    let nonce = sequencer_provider
        .get_transaction_count(signer.address())
        .await?;

    // Create multiple signed transactions with different nonces
    let tx1 = create_signed_transaction(
        &signer,
        Address::from([0x33; 20]),
        U256::from(1000),
        nonce,
        21000,
        1_000_000_000,
    )
    .await?;

    let tx2 = create_signed_transaction(
        &signer,
        Address::from([0x44; 20]),
        U256::from(2000),
        nonce + 1,
        21000,
        1_000_000_000,
    )
    .await?;

    let tx3 = create_signed_transaction(
        &signer,
        Address::from([0x55; 20]),
        U256::from(3000),
        nonce + 2,
        21000,
        1_000_000_000,
    )
    .await?;

    // Compute transaction hashes for reverting_tx_hashes
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

    let bundle_hash = client.send_bundle(bundle).await?;

    println!(
        "Multi-transaction bundle submitted! Hash: {:?}",
        bundle_hash.bundle_hash
    );
    assert!(!bundle_hash.bundle_hash.is_zero());

    Ok(())
}
