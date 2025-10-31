use alloy_primitives::{Address, Bytes, U256, keccak256};
use anyhow::Result;
use jsonrpsee::server::Server;
use rdkafka::ClientConfig;
use tips_audit::publisher::LoggingBundleEventPublisher;
use tips_e2e_tests::client::TipsRpcClient;
use tips_e2e_tests::fixtures::{create_signed_transaction, create_test_signer};
use tips_e2e_tests::mock_provider::MockProvider;
use tips_ingress_rpc::queue::KafkaQueuePublisher;
use tips_ingress_rpc::service::{IngressApiServer, IngressService};

/// Start a test server with mock provider and return its URL
async fn start_test_server() -> Result<(String, tokio::task::JoinHandle<()>)> {
    let mock_provider = MockProvider::new();

    let kafka_config = ClientConfig::new()
        .set("bootstrap.servers", "localhost:9092")
        .create()?;
    let queue = KafkaQueuePublisher::new(kafka_config, "test-topic".to_string());
    let audit_publisher = LoggingBundleEventPublisher::new();

    let service: IngressService<KafkaQueuePublisher, LoggingBundleEventPublisher, MockProvider> =
        IngressService::new(mock_provider, false, queue, audit_publisher, 10800);

    let server = Server::builder().build("127.0.0.1:0").await?;
    let addr = server.local_addr()?;
    let url = format!("http://{}", addr);

    let handle = tokio::spawn(async move {
        server.start(service.into_rpc()).stopped().await;
    });

    Ok((url, handle))
}

#[tokio::test]
async fn test_rpc_client_instantiation() -> Result<()> {
    let (url, _handle) = start_test_server().await?;
    let _client = TipsRpcClient::new(&url);
    Ok(())
}

#[tokio::test]
async fn test_send_raw_transaction_rejects_empty() -> Result<()> {
    let (url, _handle) = start_test_server().await?;
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
    let (url, _handle) = start_test_server().await?;
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
    if std::env::var("KAFKA_QUEUE_TESTS").is_err() {
        eprintln!(
            "Skipping Kafka queue tests (set KAFKA_QUEUE_TESTS=1 to run, and make sure the KafkaQueuePublisher is running)"
        );
        return Ok(());
    }

    let (url, _handle) = start_test_server().await?;
    let client = TipsRpcClient::new(&url);
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

    let (url, _handle) = start_test_server().await?;
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
    if std::env::var("KAFKA_QUEUE_TESTS").is_err() {
        eprintln!(
            "Skipping Kafka queue tests (set KAFKA_QUEUE_TESTS=1 to run, and make sure the KafkaQueuePublisher is running)"
        );
        return Ok(());
    }

    use tips_core::Bundle;

    let (url, _handle) = start_test_server().await?;
    let client = TipsRpcClient::new(&url);
    let signer = create_test_signer();

    // Create a valid signed transaction
    let to = Address::from([0x11; 20]);
    let value = U256::from(1000);
    let nonce = 0;
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
    if std::env::var("KAFKA_QUEUE_TESTS").is_err() {
        eprintln!(
            "Skipping Kafka queue tests (set KAFKA_QUEUE_TESTS=1 to run, and make sure the KafkaQueuePublisher is running)"
        );
        return Ok(());
    }

    use tips_core::Bundle;
    use uuid::Uuid;

    let (url, _handle) = start_test_server().await?;
    let client = TipsRpcClient::new(&url);
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
    if std::env::var("KAFKA_QUEUE_TESTS").is_err() {
        eprintln!(
            "Skipping Kafka queue tests (set KAFKA_QUEUE_TESTS=1 to run, and make sure the KafkaQueuePublisher is running)"
        );
        return Ok(());
    }

    use tips_core::Bundle;

    let (url, _handle) = start_test_server().await?;
    let client = TipsRpcClient::new(&url);
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
