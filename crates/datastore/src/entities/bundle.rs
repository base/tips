use alloy_primitives::hex::{FromHex, ToHexExt};
use alloy_rpc_types_mev::EthSendBundle;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "bundles")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,

    /// A list of signed transactions to execute in an atomic bundle
    pub txs: Vec<String>,

    /// Block number for which this bundle is valid
    pub block_number: u64,

    /// Minimum timestamp for which this bundle is valid, in seconds since unix epoch
    pub min_timestamp: Option<u64>,

    /// Maximum timestamp for which this bundle is valid, in seconds since unix epoch
    pub max_timestamp: Option<u64>,

    /// List of transaction hashes that are allowed to revert
    pub reverting_tx_hashes: Option<Vec<String>>,

    /// UUID that can be used to cancel/replace this bundle
    pub replacement_uuid: Option<String>,

    /// Timestamp when the bundle was created
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Timestamp when the bundle was last updated
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl From<EthSendBundle> for ActiveModel {
    fn from(bundle: EthSendBundle) -> Self {
        use sea_orm::ActiveValue::Set;

        let tx_data = bundle.txs.iter().map(|tx| tx.encode_hex()).collect();

        Self {
            id: sea_orm::ActiveValue::NotSet,
            txs: Set(tx_data),
            block_number: Set(bundle.block_number),
            min_timestamp: Set(bundle.min_timestamp),
            max_timestamp: Set(bundle.max_timestamp),
            reverting_tx_hashes: Set(Some(
                bundle
                    .reverting_tx_hashes
                    .into_iter()
                    .map(|hash| format!("{:#x}", hash))
                    .collect(),
            )),
            replacement_uuid: Set(bundle.replacement_uuid),
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(chrono::Utc::now()),
        }
    }
}

impl TryFrom<Model> for EthSendBundle {
    type Error = anyhow::Error;

    fn try_from(model: Model) -> Result<Self, Self::Error> {
        use alloy_primitives::{Bytes, FixedBytes};

        let txs: Vec<Bytes> = model
            .txs
            .into_iter()
            .map(|tx| Bytes::from_hex(tx.as_str()).unwrap()) // todo
            .collect();

        let reverting_tx_hashes: Result<Vec<FixedBytes<32>>, _> = model
            .reverting_tx_hashes
            .unwrap_or_default()
            .into_iter()
            .map(|hash_str| FixedBytes::from_hex(&hash_str))
            .collect();

        Ok(EthSendBundle {
            txs,
            block_number: model.block_number,
            min_timestamp: model.min_timestamp,
            max_timestamp: model.max_timestamp,
            reverting_tx_hashes: reverting_tx_hashes?,
            replacement_uuid: model.replacement_uuid,

            ..Default::default()
        })
    }
}
