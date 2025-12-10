use alloy_consensus::transaction::{SignerRecoverable, Transaction as ConsensusTransaction};
use alloy_primitives::{Address, B256, TxHash, U256};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use tips_core::AcceptedBundle;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TransactionId {
    pub sender: Address,
    pub nonce: U256,
    pub hash: TxHash,
}

pub type BundleId = Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DropReason {
    TimedOut,
    Reverted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: TransactionId,
    pub data: Bytes,
}

pub type UserOpHash = B256;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UserOpDropReason {
    Invalid(String),
    Expired,
    ReplacedByHigherFee,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", content = "data")]
pub enum BundleEvent {
    Received {
        bundle_id: BundleId,
        bundle: Box<AcceptedBundle>,
        timestamp_ms: i64,
    },
    Cancelled {
        bundle_id: BundleId,
        timestamp_ms: i64,
    },
    BuilderIncluded {
        bundle_id: BundleId,
        builder: String,
        block_number: u64,
        flashblock_index: u64,
        timestamp_ms: i64,
    },
    BlockIncluded {
        bundle_id: BundleId,
        block_number: u64,
        block_hash: TxHash,
        timestamp_ms: i64,
    },
    Dropped {
        bundle_id: BundleId,
        reason: DropReason,
        timestamp_ms: i64,
    },
    /// Transaction received by ingress-rpc (start of send_raw_transaction)
    TransactionReceived {
        bundle_id: BundleId,
        bundle: Box<AcceptedBundle>,
        timestamp_ms: i64,
    },
    /// Transaction processing complete in ingress-rpc (end of send_raw_transaction)
    TransactionSent {
        bundle_id: BundleId,
        tx_hash: TxHash,
        timestamp_ms: i64,
    },
    /// Backrun bundle received by ingress-rpc (start of send_backrun_bundle)
    BackrunReceived {
        bundle_id: BundleId,
        bundle: Box<AcceptedBundle>,
        timestamp_ms: i64,
    },
    /// Backrun bundle sent to builder (end of send_backrun_bundle)
    BackrunSent {
        bundle_id: BundleId,
        target_tx_hash: TxHash,
        timestamp_ms: i64,
    },
    /// Backrun bundle inserted into builder store
    BackrunInserted {
        bundle_id: BundleId,
        target_tx_hash: TxHash,
        backrun_tx_hashes: Vec<TxHash>,
        timestamp_ms: i64,
    },
    /// Transaction selected from mempool, about to start executing
    StartExecuting {
        bundle_id: Option<BundleId>,
        tx_hash: TxHash,
        block_number: u64,
        timestamp_ms: i64,
    },
    /// Transaction successfully executed and committed
    Executed {
        bundle_id: Option<BundleId>,
        tx_hash: TxHash,
        block_number: u64,
        gas_used: u64,
        timestamp_ms: i64,
    },
    /// Backrun bundle transaction executed (success or reverted)
    BackrunBundleExecuted {
        bundle_id: BundleId,
        target_tx_hash: TxHash,
        backrun_tx_hash: TxHash,
        block_number: u64,
        gas_used: u64,
        success: bool,
        timestamp_ms: i64,
    },
}

impl BundleEvent {
    /// Returns a human-readable name for the event type
    pub fn event_name(&self) -> &'static str {
        match self {
            BundleEvent::Received { .. } => "Received",
            BundleEvent::Cancelled { .. } => "Cancelled",
            BundleEvent::BuilderIncluded { .. } => "BuilderIncluded",
            BundleEvent::BlockIncluded { .. } => "BlockIncluded",
            BundleEvent::Dropped { .. } => "Dropped",
            BundleEvent::TransactionReceived { .. } => "TransactionReceived",
            BundleEvent::TransactionSent { .. } => "TransactionSent",
            BundleEvent::BackrunReceived { .. } => "BackrunReceived",
            BundleEvent::BackrunSent { .. } => "BackrunSent",
            BundleEvent::BackrunInserted { .. } => "BackrunInserted",
            BundleEvent::StartExecuting { .. } => "StartExecuting",
            BundleEvent::Executed { .. } => "Executed",
            BundleEvent::BackrunBundleExecuted { .. } => "BackrunBundleExecuted",
        }
    }

    /// Returns the bundle_id for events that have one
    pub fn bundle_id(&self) -> Option<BundleId> {
        match self {
            BundleEvent::Received { bundle_id, .. } => Some(*bundle_id),
            BundleEvent::Cancelled { bundle_id, .. } => Some(*bundle_id),
            BundleEvent::BuilderIncluded { bundle_id, .. } => Some(*bundle_id),
            BundleEvent::BlockIncluded { bundle_id, .. } => Some(*bundle_id),
            BundleEvent::Dropped { bundle_id, .. } => Some(*bundle_id),
            // Transaction events
            BundleEvent::TransactionReceived { bundle_id, .. } => Some(*bundle_id),
            BundleEvent::TransactionSent { bundle_id, .. } => Some(*bundle_id),
            // Backrun events with bundle_id
            BundleEvent::BackrunReceived { bundle_id, .. } => Some(*bundle_id),
            BundleEvent::BackrunSent { bundle_id, .. } => Some(*bundle_id),
            BundleEvent::BackrunInserted { bundle_id, .. } => Some(*bundle_id),
            BundleEvent::BackrunBundleExecuted { bundle_id, .. } => Some(*bundle_id),
            // These events have optional bundle_id (looked up from tx hash)
            BundleEvent::StartExecuting { bundle_id, .. } => *bundle_id,
            BundleEvent::Executed { bundle_id, .. } => *bundle_id,
        }
    }

    /// Returns the tx_hash for events that track individual transactions
    pub fn tx_hash(&self) -> Option<TxHash> {
        match self {
            BundleEvent::TransactionReceived { bundle, .. } => {
                bundle.txs.first().map(|tx| tx.tx_hash())
            }
            BundleEvent::TransactionSent { tx_hash, .. } => Some(*tx_hash),
            BundleEvent::BackrunReceived { bundle, .. } => {
                bundle.txs.first().map(|tx| tx.tx_hash())
            }
            BundleEvent::BackrunSent { target_tx_hash, .. } => Some(*target_tx_hash),
            BundleEvent::BackrunInserted { target_tx_hash, .. } => Some(*target_tx_hash),
            BundleEvent::StartExecuting { tx_hash, .. } => Some(*tx_hash),
            BundleEvent::Executed { tx_hash, .. } => Some(*tx_hash),
            BundleEvent::BackrunBundleExecuted { target_tx_hash, .. } => Some(*target_tx_hash),
            // Standard bundle events don't have a tx_hash
            _ => None,
        }
    }

    pub fn transaction_ids(&self) -> Vec<TransactionId> {
        match self {
            BundleEvent::Received { bundle, .. } => {
                bundle
                    .txs
                    .iter()
                    .filter_map(|envelope| {
                        match envelope.recover_signer() {
                            Ok(sender) => Some(TransactionId {
                                sender,
                                nonce: U256::from(envelope.nonce()),
                                hash: *envelope.hash(),
                            }),
                            Err(_) => None, // Skip invalid transactions
                        }
                    })
                    .collect()
            }
            BundleEvent::Cancelled { .. } => vec![],
            BundleEvent::BuilderIncluded { .. } => vec![],
            BundleEvent::BlockIncluded { .. } => vec![],
            BundleEvent::Dropped { .. } => vec![],
            // TransactionReceived has full bundle, extract transaction IDs
            BundleEvent::TransactionReceived { bundle, .. } => bundle
                .txs
                .iter()
                .filter_map(|envelope| match envelope.recover_signer() {
                    Ok(sender) => Some(TransactionId {
                        sender,
                        nonce: U256::from(envelope.nonce()),
                        hash: *envelope.hash(),
                    }),
                    Err(_) => None,
                })
                .collect(),
            // BackrunReceived has full bundle, extract transaction IDs
            BundleEvent::BackrunReceived { bundle, .. } => bundle
                .txs
                .iter()
                .filter_map(|envelope| match envelope.recover_signer() {
                    Ok(sender) => Some(TransactionId {
                        sender,
                        nonce: U256::from(envelope.nonce()),
                        hash: *envelope.hash(),
                    }),
                    Err(_) => None,
                })
                .collect(),
            // These events don't track transaction IDs this way
            BundleEvent::TransactionSent { .. } => vec![],
            BundleEvent::BackrunSent { .. } => vec![],
            BundleEvent::BackrunInserted { .. } => vec![],
            BundleEvent::StartExecuting { .. } => vec![],
            BundleEvent::Executed { .. } => vec![],
            BundleEvent::BackrunBundleExecuted { .. } => vec![],
        }
    }

    /// Returns the timestamp_ms for any event
    pub fn timestamp_ms(&self) -> i64 {
        match self {
            BundleEvent::Received { timestamp_ms, .. } => *timestamp_ms,
            BundleEvent::Cancelled { timestamp_ms, .. } => *timestamp_ms,
            BundleEvent::BuilderIncluded { timestamp_ms, .. } => *timestamp_ms,
            BundleEvent::BlockIncluded { timestamp_ms, .. } => *timestamp_ms,
            BundleEvent::Dropped { timestamp_ms, .. } => *timestamp_ms,
            BundleEvent::TransactionReceived { timestamp_ms, .. } => *timestamp_ms,
            BundleEvent::TransactionSent { timestamp_ms, .. } => *timestamp_ms,
            BundleEvent::BackrunReceived { timestamp_ms, .. } => *timestamp_ms,
            BundleEvent::BackrunSent { timestamp_ms, .. } => *timestamp_ms,
            BundleEvent::BackrunInserted { timestamp_ms, .. } => *timestamp_ms,
            BundleEvent::StartExecuting { timestamp_ms, .. } => *timestamp_ms,
            BundleEvent::Executed { timestamp_ms, .. } => *timestamp_ms,
            BundleEvent::BackrunBundleExecuted { timestamp_ms, .. } => *timestamp_ms,
        }
    }

    pub fn generate_event_key(&self) -> String {
        match self {
            BundleEvent::BlockIncluded {
                bundle_id,
                block_hash,
                ..
            } => {
                format!("{bundle_id}-{block_hash}")
            }
            // Transaction events
            BundleEvent::TransactionReceived { bundle_id, .. } => {
                format!("transaction-received-{bundle_id}")
            }
            BundleEvent::TransactionSent { bundle_id, .. } => {
                format!("transaction-sent-{bundle_id}")
            }
            // Backrun events use bundle_id
            BundleEvent::BackrunReceived { bundle_id, .. } => {
                format!("backrun-received-{bundle_id}")
            }
            BundleEvent::BackrunSent { bundle_id, .. } => {
                format!("backrun-sent-{bundle_id}")
            }
            BundleEvent::BackrunInserted { bundle_id, .. } => {
                format!("backrun-inserted-{bundle_id}")
            }
            BundleEvent::StartExecuting {
                bundle_id,
                tx_hash,
                block_number,
                ..
            } => match bundle_id {
                Some(id) => format!("start-executing-{id}-{block_number}"),
                None => format!("start-executing-{tx_hash}-{block_number}"),
            },
            BundleEvent::Executed {
                bundle_id,
                tx_hash,
                block_number,
                ..
            } => match bundle_id {
                Some(id) => format!("executed-{id}-{block_number}"),
                None => format!("executed-{tx_hash}-{block_number}"),
            },
            BundleEvent::BackrunBundleExecuted {
                bundle_id,
                backrun_tx_hash,
                block_number,
                ..
            } => {
                format!("backrun-bundle-executed-{bundle_id}-{backrun_tx_hash}-{block_number}")
            }
            _ => {
                format!(
                    "{}-{}",
                    self.bundle_id().unwrap_or_default(),
                    Uuid::new_v4()
                )
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", content = "data")]
pub enum UserOpEvent {
    AddedToMempool {
        user_op_hash: UserOpHash,
        sender: Address,
        entry_point: Address,
        nonce: U256,
    },
    Dropped {
        user_op_hash: UserOpHash,
        reason: UserOpDropReason,
    },
    Included {
        user_op_hash: UserOpHash,
        block_number: u64,
        tx_hash: TxHash,
    },
}

impl UserOpEvent {
    pub fn user_op_hash(&self) -> UserOpHash {
        match self {
            UserOpEvent::AddedToMempool { user_op_hash, .. } => *user_op_hash,
            UserOpEvent::Dropped { user_op_hash, .. } => *user_op_hash,
            UserOpEvent::Included { user_op_hash, .. } => *user_op_hash,
        }
    }

    pub fn generate_event_key(&self) -> String {
        match self {
            UserOpEvent::Included {
                user_op_hash,
                tx_hash,
                ..
            } => {
                format!("{user_op_hash}-{tx_hash}")
            }
            _ => {
                format!("{}-{}", self.user_op_hash(), Uuid::new_v4())
            }
        }
    }
}

#[cfg(test)]
mod user_op_event_tests {
    use super::*;
    use alloy_primitives::{address, b256};

    fn create_test_user_op_hash() -> UserOpHash {
        b256!("1111111111111111111111111111111111111111111111111111111111111111")
    }

    #[test]
    fn test_user_op_event_added_to_mempool_serialization() {
        let event = UserOpEvent::AddedToMempool {
            user_op_hash: create_test_user_op_hash(),
            sender: address!("2222222222222222222222222222222222222222"),
            entry_point: address!("0000000071727De22E5E9d8BAf0edAc6f37da032"),
            nonce: U256::from(1),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"event\":\"AddedToMempool\""));

        let deserialized: UserOpEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event.user_op_hash(), deserialized.user_op_hash());
    }

    #[test]
    fn test_user_op_event_dropped_serialization() {
        let event = UserOpEvent::Dropped {
            user_op_hash: create_test_user_op_hash(),
            reason: UserOpDropReason::Invalid("gas too low".to_string()),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"event\":\"Dropped\""));
        assert!(json.contains("gas too low"));

        let deserialized: UserOpEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event.user_op_hash(), deserialized.user_op_hash());
    }

    #[test]
    fn test_user_op_event_included_serialization() {
        let event = UserOpEvent::Included {
            user_op_hash: create_test_user_op_hash(),
            block_number: 12345,
            tx_hash: b256!("3333333333333333333333333333333333333333333333333333333333333333"),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"event\":\"Included\""));
        assert!(json.contains("\"block_number\":12345"));

        let deserialized: UserOpEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event.user_op_hash(), deserialized.user_op_hash());
    }

    #[test]
    fn test_user_op_hash_accessor() {
        let hash = create_test_user_op_hash();

        let added = UserOpEvent::AddedToMempool {
            user_op_hash: hash,
            sender: address!("2222222222222222222222222222222222222222"),
            entry_point: address!("0000000071727De22E5E9d8BAf0edAc6f37da032"),
            nonce: U256::from(1),
        };
        assert_eq!(added.user_op_hash(), hash);

        let dropped = UserOpEvent::Dropped {
            user_op_hash: hash,
            reason: UserOpDropReason::Expired,
        };
        assert_eq!(dropped.user_op_hash(), hash);

        let included = UserOpEvent::Included {
            user_op_hash: hash,
            block_number: 100,
            tx_hash: b256!("4444444444444444444444444444444444444444444444444444444444444444"),
        };
        assert_eq!(included.user_op_hash(), hash);
    }

    #[test]
    fn test_generate_event_key_included() {
        let user_op_hash =
            b256!("1111111111111111111111111111111111111111111111111111111111111111");
        let tx_hash = b256!("2222222222222222222222222222222222222222222222222222222222222222");

        let event = UserOpEvent::Included {
            user_op_hash,
            block_number: 100,
            tx_hash,
        };

        let key = event.generate_event_key();
        assert!(key.contains(&format!("{user_op_hash}")));
        assert!(key.contains(&format!("{tx_hash}")));
    }

    #[test]
    fn test_user_op_drop_reason_variants() {
        let invalid = UserOpDropReason::Invalid("test reason".to_string());
        let json = serde_json::to_string(&invalid).unwrap();
        assert!(json.contains("Invalid"));
        assert!(json.contains("test reason"));

        let expired = UserOpDropReason::Expired;
        let json = serde_json::to_string(&expired).unwrap();
        assert!(json.contains("Expired"));

        let replaced = UserOpDropReason::ReplacedByHigherFee;
        let json = serde_json::to_string(&replaced).unwrap();
        assert!(json.contains("ReplacedByHigherFee"));
    }
}
