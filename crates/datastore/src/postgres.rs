use crate::traits::BundleDatastore;
use alloy_consensus::Transaction;
use alloy_consensus::private::alloy_eips::Decodable2718;
use alloy_consensus::transaction::SignerRecoverable;
use alloy_primitives::hex::{FromHex, ToHexExt};
use alloy_primitives::{Address, TxHash};
use alloy_rpc_types_mev::EthSendBundle;
use anyhow::Result;
use op_alloy_consensus::OpTxEnvelope;
use sqlx::PgPool;
use tracing::info;
use uuid::Uuid;

/// Extended bundle data that includes the original bundle plus extracted metadata
#[derive(Debug, Clone)]
pub struct BundleWithMetadata {
    pub bundle: EthSendBundle,
    pub txn_hashes: Vec<TxHash>,
    pub senders: Vec<Address>,
    pub min_base_fee: i64,
}

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

impl PostgresDatastore {
    fn extract_bundle_metadata(
        &self,
        bundle: &EthSendBundle,
    ) -> Result<(Vec<String>, i64, Vec<String>)> {
        let mut senders = Vec::new();
        let mut txn_hashes = Vec::new();

        let mut min_base_fee = i64::MAX;

        for tx_bytes in &bundle.txs {
            let envelope = OpTxEnvelope::decode_2718_exact(tx_bytes)?;
            txn_hashes.push(envelope.hash().encode_hex_with_prefix());

            let sender = match envelope.recover_signer() {
                Ok(signer) => signer,
                Err(err) => return Err(err.into()),
            };

            senders.push(sender.encode_hex_with_prefix());
            min_base_fee = min_base_fee.min(envelope.max_fee_per_gas() as i64); // todo type and todo not right
        }

        let minimum_base_fee = if min_base_fee == i64::MAX {
            0
        } else {
            min_base_fee
        };

        Ok((senders, minimum_base_fee, txn_hashes))
    }
}

#[async_trait::async_trait]
impl BundleDatastore for PostgresDatastore {
    async fn insert_bundle(&self, bundle: EthSendBundle) -> Result<Uuid> {
        let id = Uuid::new_v4();

        let (senders, minimum_base_fee, txn_hashes) = self.extract_bundle_metadata(&bundle)?;

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
                id, senders, minimum_base_fee, txn_hashes, 
                txs, reverting_tx_hashes, dropping_tx_hashes, 
                block_number, min_timestamp, max_timestamp,
                created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NOW(), NOW())
            "#,
            id,
            &senders,
            minimum_base_fee,
            &txn_hashes,
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

    async fn get_bundle(&self, id: Uuid) -> Result<Option<BundleWithMetadata>> {
        let result = sqlx::query!(
            r#"
            SELECT senders, minimum_base_fee, txn_hashes, txs, reverting_tx_hashes, 
                   dropping_tx_hashes, block_number, min_timestamp, max_timestamp
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

                let bundle = EthSendBundle {
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
                };

                let txn_hashes: Result<Vec<TxHash>, _> = row
                    .txn_hashes
                    .unwrap_or_default()
                    .into_iter()
                    .map(TxHash::from_hex)
                    .collect();

                let senders: Result<Vec<Address>, _> = row
                    .senders
                    .unwrap_or_default()
                    .into_iter()
                    .map(Address::from_hex)
                    .collect();

                Ok(Some(BundleWithMetadata {
                    bundle,
                    txn_hashes: txn_hashes?,
                    senders: senders?,
                    min_base_fee: row.minimum_base_fee.unwrap_or(0),
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
