use crate::bundle::UserOpBundle;
use alloy_primitives::Address;
use rblib::{prelude::*, reth::core::primitives::Recovered};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct InsertUserOpBundle {
    pub bundler_address: Address,
    pub bundler_nonce: Arc<Mutex<u64>>,
    pub userops_pool: Arc<Mutex<Vec<UserOpBundle>>>,
}

impl InsertUserOpBundle {
    pub fn new(bundler_address: Address) -> Self {
        Self {
            bundler_address,
            bundler_nonce: Arc::new(Mutex::new(0)),
            userops_pool: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn add_bundle(&self, bundle: UserOpBundle) {
        if let Ok(mut pool) = self.userops_pool.lock() {
            pool.push(bundle);
        }
    }

    pub fn get_next_nonce(&self) -> u64 {
        if let Ok(mut nonce) = self.bundler_nonce.lock() {
            let current = *nonce;
            *nonce += 1;
            current
        } else {
            0
        }
    }

    fn get_pending_bundles(&self) -> Vec<UserOpBundle> {
        if let Ok(mut pool) = self.userops_pool.lock() {
            pool.drain(..).collect()
        } else {
            Vec::new()
        }
    }
}

pub struct TransactionCollector {
    pub transactions: Vec<Recovered<types::Transaction<Optimism>>>,
    pub midpoint_reached: bool,
    pub userops_inserted: bool,
    pub userops_bundle_step: InsertUserOpBundle,
}

impl TransactionCollector {
    pub fn new(userops_step: InsertUserOpBundle) -> Self {
        Self {
            transactions: Vec::new(),
            midpoint_reached: false,
            userops_inserted: false,
            userops_bundle_step: userops_step,
        }
    }

    pub fn collect_transaction(&mut self, tx: Recovered<types::Transaction<Optimism>>) {
        self.transactions.push(tx);
    }

    pub fn maybe_insert_userops_bundle(
        &mut self,
        checkpoint: &Checkpoint<Optimism>,
    ) -> Option<Recovered<types::Transaction<Optimism>>> {
        if self.userops_inserted {
            return None;
        }

        let total_txs = checkpoint.history().into_iter().count();
        let midpoint = total_txs / 2;

        if self.transactions.len() >= midpoint && !self.midpoint_reached {
            self.midpoint_reached = true;

            let bundles = self.userops_bundle_step.get_pending_bundles();
            if bundles.is_empty() {
                return None;
            }

            let merged_bundle = bundles.into_iter().reduce(|mut acc, bundle| {
                acc.user_ops.extend(bundle.user_ops);
                acc
            })?;

            let nonce = self.userops_bundle_step.get_next_nonce();
            let chain_id = checkpoint.block().chainspec().chain().id();
            let base_fee = checkpoint
                .block()
                .parent()
                .base_fee_per_gas
                .unwrap_or(1000000000);

            let bundler_tx = merged_bundle.create_bundle_transaction(
                self.userops_bundle_step.bundler_address,
                nonce,
                chain_id,
                base_fee as u128,
            )?;

            self.userops_inserted = true;
            Some(bundler_tx)
        } else {
            None
        }
    }
}
