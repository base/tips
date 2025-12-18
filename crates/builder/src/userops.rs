use account_abstraction_core::types::{UserOperationRequest, VersionedUserOperation};
use alloy_primitives::{Address, B256};
use rblib::orderpool2::{BundleNonce, OrderpoolOrder};
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserOperationOrder {
    pub request: UserOperationRequest,
    hash: B256,
}

impl UserOperationOrder {
    pub fn new(request: UserOperationRequest) -> anyhow::Result<Self> {
        let hash = request.hash()?;
        Ok(Self { request, hash })
    }

    pub fn sender(&self) -> Address {
        match &self.request.user_operation {
            VersionedUserOperation::UserOperation(op) => op.sender,
            VersionedUserOperation::PackedUserOperation(op) => op.sender,
        }
    }

    pub fn nonce(&self) -> u64 {
        match &self.request.user_operation {
            VersionedUserOperation::UserOperation(op) => op.nonce.to::<u64>(),
            VersionedUserOperation::PackedUserOperation(op) => op.nonce.to::<u64>(),
        }
    }
}

impl OrderpoolOrder for UserOperationOrder {
    type ID = B256;

    fn id(&self) -> Self::ID {
        self.hash
    }

    fn nonces(&self) -> Vec<BundleNonce> {
        vec![BundleNonce {
            address: self.sender(),
            nonce: self.nonce(),
            optional: false,
        }]
    }
}

impl Hash for UserOperationOrder {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hash.hash(state);
    }
}
