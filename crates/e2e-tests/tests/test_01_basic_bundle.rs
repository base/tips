use alloy_primitives::Bytes;
use eyre::Result;
use tips_e2e_tests::client::TipsRpcClient;

#[tokio::test]
async fn test_rpc_client_instantiation() -> Result<()> {
    let _client = TipsRpcClient::new("http://localhost:8080");
    
    println!("✅ RPC client created successfully");
    println!("   Target: http://localhost:8080");
    
    Ok(())
}

#[tokio::test]
async fn test_send_raw_transaction_rejects_empty() -> Result<()> {
    let client = TipsRpcClient::new("http://localhost:8080");
    
    let empty_tx = Bytes::new();
    let result = client.send_raw_transaction(empty_tx).await;
    
    assert!(result.is_err(), "Empty transaction should be rejected");
    
    let error_msg = result.unwrap_err().to_string();
    println!("✅ Server correctly rejected empty transaction");
    println!("   Error: {}", error_msg);
    
    assert!(
        error_msg.contains("RPC error") || error_msg.contains("empty"),
        "Error should mention empty data or be an RPC error"
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
    println!("✅ Server correctly rejected invalid transaction");
    println!("   Error: {}", error_msg);
    
    assert!(
        error_msg.contains("RPC error") || error_msg.contains("decode") || error_msg.contains("Failed"),
        "Error should mention decoding failure"
    );
    
    Ok(())
}

#[tokio::test]
#[ignore]
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
        Ok(uuid) => {
            println!("✅ Successfully got UUID: {}", uuid);
            Ok(())
        }
        Err(e) => {
            let error_msg = e.to_string();
            if error_msg.contains("RPC error") || error_msg.contains("validation") {
                println!("✅ RPC endpoint responded (validation error expected for empty bundle)");
                println!("   Error: {}", error_msg);
                Ok(())
            } else {
                println!("❌ Unexpected error: {}", error_msg);
                Err(e)
            }
        }
    }
}
