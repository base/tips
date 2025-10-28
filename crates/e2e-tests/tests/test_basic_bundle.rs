use alloy_primitives::{Address, Bytes, U256};
use anyhow::Result;
use tips_e2e_tests::client::TipsRpcClient;
use tips_e2e_tests::fixtures::{create_signed_transaction, create_test_signer};

#[tokio::test]
async fn test_rpc_client_instantiation() -> Result<()> {
    let _client = TipsRpcClient::new("http://localhost:8080");
    Ok(())
}

#[tokio::test]
async fn test_send_raw_transaction_rejects_empty() -> Result<()> {
    let client = TipsRpcClient::new("http://localhost:8080");

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
    let client = TipsRpcClient::new("http://localhost:8080");

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
    if std::env::var("RUN_NODE_TESTS").is_err() {
        eprintln!("skipping test_send_valid_transaction (set RUN_NODE_TESTS=1 to run)");
        return Ok(());
    }
    let client = TipsRpcClient::new("http://localhost:8080");
    let signer = create_test_signer();

    let to = Address::from([0x11; 20]);
    let value = U256::from(1000);
    let nonce = 0;
    let gas_limit = 21000;
    let gas_price = 1_000_000_000;

    let signed_tx =
        create_signed_transaction(&signer, to, value, nonce, gas_limit, gas_price).await?;

    let result = client.send_raw_transaction(signed_tx).await;

    result.map(|_tx_hash| ())
}

#[tokio::test]
async fn test_send_bundle_rejects_empty() -> Result<()> {
    use tips_core::Bundle;

    let client = TipsRpcClient::new("http://localhost:8080");

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
        error_msg.contains("RPC error") || error_msg.contains("empty") || error_msg.contains("validation"),
        "Error should mention validation failure, got: {}",
        error_msg
    );

    Ok(())
}

#[tokio::test]
async fn test_send_bundle_with_valid_transaction() -> Result<()> {
    if std::env::var("RUN_NODE_TESTS").is_err() {
        eprintln!("skipping test_send_bundle_with_valid_transaction (set RUN_NODE_TESTS=1 to run)");
        return Ok(());
    }

    use tips_core::Bundle;

    let client = TipsRpcClient::new("http://localhost:8080");
    let signer = create_test_signer();

    // Create a valid signed transaction
    let to = Address::from([0x11; 20]);
    let value = U256::from(1000);
    let nonce = 0;
    let gas_limit = 21000;
    let gas_price = 1_000_000_000;

    let signed_tx =
        create_signed_transaction(&signer, to, value, nonce, gas_limit, gas_price).await?;

    let bundle = Bundle {
        txs: vec![signed_tx],
        block_number: 1,
        min_timestamp: None,
        max_timestamp: None,
        reverting_tx_hashes: vec![],
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
    if std::env::var("RUN_NODE_TESTS").is_err() {
        eprintln!("skipping test_send_bundle_with_replacement_uuid (set RUN_NODE_TESTS=1 to run)");
        return Ok(());
    }

    use tips_core::Bundle;
    use uuid::Uuid;

    let client = TipsRpcClient::new("http://localhost:8080");
    let signer = create_test_signer();

    let signed_tx = create_signed_transaction(
        &signer,
        Address::from([0x22; 20]),
        U256::from(2000),
        0,
        21000,
        1_000_000_000,
    )
    .await?;

    let replacement_uuid = Uuid::new_v4();

    let bundle = Bundle {
        txs: vec![signed_tx],
        block_number: 1,
        replacement_uuid: Some(replacement_uuid.to_string()),
        min_timestamp: None,
        max_timestamp: None,
        reverting_tx_hashes: vec![],
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
    if std::env::var("RUN_NODE_TESTS").is_err() {
        eprintln!(
            "skipping test_send_bundle_with_multiple_transactions (set RUN_NODE_TESTS=1 to run)"
        );
        return Ok(());
    }

    use tips_core::Bundle;

    let client = TipsRpcClient::new("http://localhost:8080");
    let signer = create_test_signer();

    // Create multiple signed transactions with different nonces
    let tx1 = create_signed_transaction(
        &signer,
        Address::from([0x33; 20]),
        U256::from(1000),
        0,
        21000,
        1_000_000_000,
    )
    .await?;

    let tx2 = create_signed_transaction(
        &signer,
        Address::from([0x44; 20]),
        U256::from(2000),
        1,
        21000,
        1_000_000_000,
    )
    .await?;

    let tx3 = create_signed_transaction(
        &signer,
        Address::from([0x55; 20]),
        U256::from(3000),
        2,
        21000,
        1_000_000_000,
    )
    .await?;

    let bundle = Bundle {
        txs: vec![tx1, tx2, tx3],
        block_number: 1,
        min_timestamp: None,
        max_timestamp: None,
        reverting_tx_hashes: vec![],
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
