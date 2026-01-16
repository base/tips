use crate::domain::types::WrappedUserOperation;
use alloy_primitives::{Address, B256};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", content = "data")]
pub enum MempoolEvent {
    UserOpAdded {
        user_op: WrappedUserOperation,
        entry_point: Address,
    },
    UserOpIncluded {
        user_op: WrappedUserOperation,
        block_number: u64,
        tx_hash: B256,
    },
    UserOpDropped {
        user_op: WrappedUserOperation,
        reason: String,
    },
}
