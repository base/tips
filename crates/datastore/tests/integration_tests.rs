use datastore::PostgresDatastore;
use sqlx::PgPool;
use testcontainers_modules::{postgres, testcontainers::runners::AsyncRunner};

#[tokio::test]
async fn insert_and_query() -> eyre::Result<()> {
    let postgres_instance = postgres::Postgres::default().start().await?;
    let connection_string = format!(
        "postgres://postgres:postgres@{}:{}/postgres",
        postgres_instance.get_host().await?,
        postgres_instance.get_host_port_ipv4(5432).await?
    );

    let pool = PgPool::connect(&connection_string).await?;
    let data_store = PostgresDatastore::new(pool);

    assert!(data_store.run_migrations().await.is_ok());

    Ok(())
}
