use alloy_primitives::{Address, TxHash, U256};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TransactionId {
    pub sender: Address,
    pub nonce: U256,
    pub hash: TxHash,
}

pub type BundleId = Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleWithMetadata {
    pub id: BundleId,
    pub transactions: Vec<Transaction>,
    pub metadata: serde_json::Value,
}

pub type Bundle = BundleWithMetadata;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: TransactionId,
    pub data: Bytes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum MempoolEvent {
    ReceivedBundle {
        bundle_id: BundleId,
        transactions: Vec<Transaction>,
    },
    CancelledBundle {
        bundle_id: BundleId,
        transaction_ids: Vec<TransactionId>,
    },
    BuilderMined {
        bundle_id: BundleId,
        transaction_ids: Vec<TransactionId>,
        block_number: u64,
        flashblock_index: u64,
    },
    FlashblockInclusion {
        bundle_id: BundleId,
        transaction_ids: Vec<TransactionId>,
        block_number: u64,
        flashblock_index: u64,
    },
    BlockInclusion {
        bundle_id: BundleId,
        transaction_ids: Vec<TransactionId>,
        block_hash: TxHash,
        block_number: u64,
        flashblock_index: u64,
    },
}

impl MempoolEvent {
    pub fn bundle_id(&self) -> BundleId {
        match self {
            MempoolEvent::ReceivedBundle { bundle_id, .. } => *bundle_id,
            MempoolEvent::CancelledBundle { bundle_id, .. } => *bundle_id,
            MempoolEvent::BuilderMined { bundle_id, .. } => *bundle_id,
            MempoolEvent::FlashblockInclusion { bundle_id, .. } => *bundle_id,
            MempoolEvent::BlockInclusion { bundle_id, .. } => *bundle_id,
        }
    }

    pub fn transaction_ids(&self) -> Vec<TransactionId> {
        match self {
            MempoolEvent::ReceivedBundle { transactions, .. } => {
                transactions.iter().map(|t| t.id.clone()).collect()
            }
            MempoolEvent::CancelledBundle {
                transaction_ids, ..
            } => transaction_ids.clone(),
            MempoolEvent::BuilderMined {
                transaction_ids, ..
            } => transaction_ids.clone(),
            MempoolEvent::FlashblockInclusion {
                transaction_ids, ..
            } => transaction_ids.clone(),
            MempoolEvent::BlockInclusion {
                transaction_ids, ..
            } => transaction_ids.clone(),
        }
    }
}
