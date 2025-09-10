use crate::traits::BundleDatastore;
use alloy_primitives::TxHash;
use alloy_primitives::hex::{FromHex, ToHexExt};
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
    async fn insert_bundle(&self, bundle: EthSendBundle) -> Result<Uuid> {
        let id = Uuid::new_v4();

        let txs: Vec<String> = bundle
            .txs
            .iter()
            .map(|tx| tx.encode_hex_upper_with_prefix())
            .collect();
        let reverting_tx_hashes: Vec<String> = bundle
            .reverting_tx_hashes
            .iter()
            .map(|h| h.encode_hex_with_prefix())
            .collect();
        let dropping_tx_hashes: Vec<String> = bundle
            .dropping_tx_hashes
            .iter()
            .map(|h| h.encode_hex_with_prefix())
            .collect();

        sqlx::query!(
            r#"
            INSERT INTO bundles (
                id, txs, reverting_tx_hashes, dropping_tx_hashes, 
                block_number, min_timestamp, max_timestamp,
                created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW())
            "#,
            id,
            &txs,
            &reverting_tx_hashes,
            &dropping_tx_hashes,
            bundle.block_number as i64,
            bundle.min_timestamp.map(|t| t as i64),
            bundle.max_timestamp.map(|t| t as i64),
        )
        .execute(&self.pool)
        .await?;

        Ok(id)
    }

    async fn get_bundle(&self, id: Uuid) -> Result<Option<EthSendBundle>> {
        let result = sqlx::query!(
            r#"
            SELECT txs, reverting_tx_hashes, dropping_tx_hashes, 
                   block_number, min_timestamp, max_timestamp
            FROM bundles 
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;

        match result {
            Some(row) => {
                let txs: Result<Vec<alloy_primitives::Bytes>, _> =
                    row.txs.into_iter().map(|tx_hex| tx_hex.parse()).collect();

                let reverting_tx_hashes: Result<Vec<TxHash>, _> = row
                    .reverting_tx_hashes
                    .unwrap_or_default()
                    .into_iter()
                    .map(TxHash::from_hex)
                    .collect();

                let dropping_tx_hashes: Result<Vec<TxHash>, _> = row
                    .dropping_tx_hashes
                    .unwrap_or_default()
                    .into_iter()
                    .map(TxHash::from_hex)
                    .collect();

                Ok(Some(EthSendBundle {
                    txs: txs?,
                    block_number: row.block_number.unwrap_or(0) as u64,
                    min_timestamp: row.min_timestamp.map(|t| t as u64),
                    max_timestamp: row.max_timestamp.map(|t| t as u64),
                    reverting_tx_hashes: reverting_tx_hashes?,
                    replacement_uuid: None,
                    dropping_tx_hashes: dropping_tx_hashes?,
                    refund_percent: None,
                    refund_recipient: None,
                    refund_tx_hashes: Vec::new(),
                    extra_fields: Default::default(),
                }))
            }
            None => Ok(None),
        }
    }

    async fn cancel_bundle(&self, _id: Uuid) -> Result<()> {
        todo!()
    }

    async fn select_bundles(&self) -> Result<Vec<EthSendBundle>> {
        todo!()
    }
}
