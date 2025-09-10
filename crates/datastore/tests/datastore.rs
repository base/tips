use alloy_primitives::{Bytes, TxHash};
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
            "0x02f86f0102843b9aca0085029e7822d68298f094d2c8e0b2e8f2a8e8f2a8e8f2a8e8f2a8e8f2a880b844a9059cbb000000000000000000000000d2c8e0b2e8f2a8e8f2a8e8f2a8e8f2a8e8f2a80000000000000000000000000000000000000000000000000de0b6b3a7640000c080a0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0a0fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210".parse::<Bytes>()?,
        ],
        block_number: 12345,
        min_timestamp: Some(1640995200),
        max_timestamp: Some(1640995260),
        reverting_tx_hashes: vec![
            "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".parse::<TxHash>()?,
        ],
        replacement_uuid: None,
        dropping_tx_hashes: vec![
            "0xfedcba0987654321fedcba0987654321fedcba0987654321fedcba0987654321".parse::<TxHash>()?,
        ],
        refund_percent: None,
        refund_recipient: None,
        refund_tx_hashes: vec![],
        extra_fields: Default::default(),
    };

    let insert_result = harness.data_store.insert_bundle(test_bundle.clone()).await;
    assert!(insert_result.is_ok());
    let bundle_id = insert_result.unwrap();

    let query_result = harness.data_store.get_bundle(bundle_id).await;
    assert!(query_result.is_ok());
    let retrieved_bundle = query_result.unwrap();

    assert!(retrieved_bundle.is_some(), "Bundle should be found");
    let retrieved_bundle = retrieved_bundle.unwrap();
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

    Ok(())
}
