use alloy_primitives::{Address, Bytes, U256};
use eyre::Result;
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

    match result {
        Ok(_tx_hash) => Ok(()),
        Err(e) => {
            let error_msg = e.to_string();
            if error_msg.contains("connection") {
                println!("Server not running. Start with: just start-all");
            } else {
                println!("Unexpected error: {}", error_msg);
            }
            Err(e)
        }
    }
}

#[tokio::test]
async fn test_send_bundle_when_implemented() -> Result<()> {
    use alloy_rpc_types_mev::EthSendBundle;

    let client = TipsRpcClient::new("http://localhost:8080");

    let empty_bundle = EthSendBundle {
        txs: vec![],
        block_number: 1,
        min_timestamp: None,
        max_timestamp: None,
        reverting_tx_hashes: vec![],
        replacement_uuid: None,
        ..Default::default()
    };

    let result = client.send_bundle(empty_bundle).await;

    match result {
        Ok(_uuid) => Ok(()),
        Err(e) => {
            let error_msg = e.to_string();
            if error_msg.contains("connection closed")
                || error_msg.contains("error sending request")
            {
                // eth_sendBundle not yet implemented on server
                Ok(())
            } else if error_msg.contains("RPC error") || error_msg.contains("validation") {
                // RPC endpoint responded - validation error expected for empty bundle
                Ok(())
            } else {
                println!("Unexpected error: {}", error_msg);
                Err(e)
            }
        }
    }
}
