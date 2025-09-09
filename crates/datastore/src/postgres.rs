use crate::traits::BundleDatastore;
use alloy_rpc_types_mev::EthSendBundle;
use anyhow::Result;
use sqlx::PgPool;
use tracing::info;
use uuid::Uuid;

/// PostgreSQL implementation of the BundleDatastore trait
pub struct PostgresDatastore {
    pool: PgPool,
}

impl PostgresDatastore {
    pub async fn run_migrations(&self) -> Result<()> {
        info!(message = "running migrations");
        sqlx::migrate!("./migrations").run(&self.pool).await?;
        info!(message = "migrations complete");
        Ok(())
    }
}

impl PostgresDatastore {
    /// Create a new PostgreSQL datastore instance
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl BundleDatastore for PostgresDatastore {
    async fn insert_bundle(&self, _bundle: EthSendBundle) -> Result<Uuid> {
        todo!()
    }

    async fn get_bundle(&self, _id: Uuid) -> Result<Option<EthSendBundle>> {
        todo!()
    }

    async fn cancel_bundle(&self, _id: Uuid) -> Result<()> {
        todo!()
    }

    async fn select_bundles(&self) -> Result<Vec<EthSendBundle>> {
        todo!()
    }
}
