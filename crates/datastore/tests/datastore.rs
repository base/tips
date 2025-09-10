use alloy_primitives::{Address, Bytes, TxHash};
use alloy_rpc_types_mev::EthSendBundle;
use datastore::{PostgresDatastore, traits::BundleDatastore};
use sqlx::PgPool;
use testcontainers_modules::{
    postgres,
    testcontainers::{ContainerAsync, runners::AsyncRunner},
};

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

#[tokio::test]
async fn insert_and_get() -> eyre::Result<()> {
    let harness = setup_datastore().await?;
    let test_bundle = EthSendBundle {
        txs: vec![
            "0x02f8bf8221058304f8c782038c83d2a76b833d0900942e85c218afcdeb3d3b3f0f72941b4861f915bbcf80b85102000e0000000bb800001010c78c430a094eb7ae67d41a7cca25cdb9315e63baceb03bf4529e57a6b1b900010001f4000a101010110111101111011011faa7efc8e6aa13b029547eecbf5d370b4e1e52eec080a009fc02a6612877cec7e1223f0a14f9a9507b82ef03af41fcf14bf5018ccf2242a0338b46da29a62d28745c828077327588dc82c03a4b0210e3ee1fd62c608f8fcd".parse::<Bytes>()?,
        ],
        block_number: 12345,
        min_timestamp: Some(1640995200),
        max_timestamp: Some(1640995260),
        reverting_tx_hashes: vec![
            "0x3ea7e1482485387e61150ee8e5c8cad48a14591789ac02cc2504046d96d0a5f4".parse::<TxHash>()?,
        ],
        replacement_uuid: None,
        dropping_tx_hashes: vec![],
        refund_percent: None,
        refund_recipient: None,
        refund_tx_hashes: vec![],
        extra_fields: Default::default(),
    };

    let insert_result = harness.data_store.insert_bundle(test_bundle.clone()).await;
    if let Err(ref err) = insert_result {
        eprintln!("Insert failed with error: {:?}", err);
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

    let expected_hash: TxHash =
        "0x3ea7e1482485387e61150ee8e5c8cad48a14591789ac02cc2504046d96d0a5f4".parse()?;
    let expected_sender: Address = "0x24ae36512421f1d9f6e074f00ff5b8393f5dd925".parse()?;

    assert_eq!(
        metadata.txn_hashes[0], expected_hash,
        "Transaction hash should match"
    );
    assert_eq!(metadata.senders[0], expected_sender, "Sender should match");

    Ok(())
}
