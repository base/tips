use alloy_primitives::{Address, B256, U256};
use alloy_primitives::map::HashMap;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::sync::{Arc, Mutex};
use tips_ingress_rpc::UserOperation;
use tracing::debug;

/// Simple UserOp identifier
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserOpId {
    pub sender: Address,
    pub nonce: U256,
    pub entry_point: Address,
}

/// UserOperation with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserOpPoolItem {
    pub id: UserOpId,
    pub user_op: UserOperation,
    pub entry_point: Address,
    pub hash: B256,
}

impl UserOpPoolItem {
    pub fn new(user_op: UserOperation, entry_point: Address, hash: B256) -> Self {
        Self {
            id: UserOpId {
                sender: user_op.sender(),
                nonce: user_op.nonce(),
                entry_point,
            },
            user_op,
            entry_point,
            hash,
        }
    }
}

/// Simple trait for pool operations
pub trait UserOpStore {
    fn add_user_op(&mut self, item: UserOpPoolItem);
    fn get_user_ops(&self) -> Vec<UserOpPoolItem>;
    fn remove_user_ops(&mut self, ids: Vec<UserOpId>);
}

struct UserOpPoolData {
    user_ops: HashMap<UserOpId, UserOpPoolItem>,
}

#[derive(Clone)]
pub struct InMemoryUserOpPool {
    inner: Arc<Mutex<UserOpPoolData>>,
}

impl Debug for InMemoryUserOpPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InMemoryUserOpPool")
            .field("user_op_count", &self.inner.lock().unwrap().user_ops.len())
            .finish()
    }
}

impl InMemoryUserOpPool {
    pub fn new() -> Self {
        InMemoryUserOpPool {
            inner: Arc::new(Mutex::new(UserOpPoolData {
                user_ops: HashMap::default(),
            })),
        }
    }
}

impl Default for InMemoryUserOpPool {
    fn default() -> Self {
        Self::new()
    }
}

impl UserOpStore for InMemoryUserOpPool {
    fn add_user_op(&mut self, item: UserOpPoolItem) {
        let mut inner = self.inner.lock().unwrap();
        debug!(
            sender = %item.id.sender,
            nonce = %item.id.nonce,
            entry_point = %item.entry_point,
            hash = %item.hash,
            "Added UserOp to pool"
        );
        inner.user_ops.insert(item.id.clone(), item);
    }

    fn get_user_ops(&self) -> Vec<UserOpPoolItem> {
        let inner = self.inner.lock().unwrap();
        inner.user_ops.values().cloned().collect()
    }

    fn remove_user_ops(&mut self, ids: Vec<UserOpId>) {
        let mut inner = self.inner.lock().unwrap();
        for id in ids {
            inner.user_ops.remove(&id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{address, b256};

    fn create_test_user_op_item(sender: Address, nonce: u64) -> UserOpPoolItem {
        use tips_ingress_rpc::UserOperationV06;
        
        let user_op = UserOperation::V06(UserOperationV06 {
            sender,
            nonce: U256::from(nonce),
            init_code: Default::default(),
            call_data: Default::default(),
            call_gas_limit: U256::from(100000),
            verification_gas_limit: U256::from(100000),
            pre_verification_gas: U256::from(21000),
            max_fee_per_gas: U256::from(1000000000),
            max_priority_fee_per_gas: U256::from(1000000000),
            paymaster_and_data: Default::default(),
            signature: Default::default(),
        });

        let entry_point = address!("0000000071727De22E5E9d8BAf0edAc6f37da032");
        let hash = b256!("0000000000000000000000000000000000000000000000000000000000000001");

        UserOpPoolItem::new(user_op, entry_point, hash)
    }

    #[test]
    fn test_add_and_get_user_ops() {
        let mut pool = InMemoryUserOpPool::new();
        let sender = address!("1000000000000000000000000000000000000001");

        let item1 = create_test_user_op_item(sender, 1);
        let item2 = create_test_user_op_item(sender, 2);

        pool.add_user_op(item1.clone());
        pool.add_user_op(item2.clone());

        let user_ops = pool.get_user_ops();
        assert_eq!(user_ops.len(), 2);
    }

    #[test]
    fn test_remove_user_ops() {
        let mut pool = InMemoryUserOpPool::new();
        let sender = address!("1000000000000000000000000000000000000001");

        let item1 = create_test_user_op_item(sender, 1);
        let item2 = create_test_user_op_item(sender, 2);
        let item3 = create_test_user_op_item(sender, 3);

        pool.add_user_op(item1.clone());
        pool.add_user_op(item2.clone());
        pool.add_user_op(item3.clone());

        assert_eq!(pool.get_user_ops().len(), 3);

        pool.remove_user_ops(vec![item1.id.clone(), item2.id.clone()]);

        let user_ops = pool.get_user_ops();
        assert_eq!(user_ops.len(), 1);
        assert_eq!(user_ops[0].id, item3.id);
    }

    #[test]
    fn test_replace_user_op() {
        let mut pool = InMemoryUserOpPool::new();
        let sender = address!("1000000000000000000000000000000000000001");

        let item1 = create_test_user_op_item(sender, 1);
        let item2 = create_test_user_op_item(sender, 1); // Same nonce

        pool.add_user_op(item1);
        pool.add_user_op(item2.clone());

        let user_ops = pool.get_user_ops();
        assert_eq!(user_ops.len(), 1);
        assert_eq!(user_ops[0].hash, item2.hash);
    }
}

