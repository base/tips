use crate::tracker::TransactionTracker;
use alloy_provider::{Provider, RootProvider};
use anyhow::Result;
use op_alloy_network::Optimism;
use std::sync::Arc;
use std::time::Duration;
use tracing::debug;

pub struct ReceiptPoller {
    sequencer: RootProvider<Optimism>,
    tracker: Arc<TransactionTracker>,
    timeout: Duration,
}

impl ReceiptPoller {
    pub fn new(
        sequencer: RootProvider<Optimism>,
        tracker: Arc<TransactionTracker>,
        timeout: Duration,
    ) -> Self {
        Self {
            sequencer,
            tracker,
            timeout,
        }
    }

    pub async fn run(self) -> Result<()> {
        let mut interval = tokio::time::interval(Duration::from_secs(2)); // Block time

        loop {
            interval.tick().await;

            let pending_txs = self.tracker.get_pending();

            for (tx_hash, send_time) in pending_txs {
                let elapsed = send_time.elapsed();

                if elapsed > self.timeout {
                    self.tracker.record_timeout(tx_hash);
                    debug!("Transaction timed out: {:?}", tx_hash);
                    continue;
                }

                match self.sequencer.get_transaction_receipt(tx_hash).await {
                    Ok(Some(_receipt)) => {
                        let inclusion_time = send_time.elapsed();
                        self.tracker.record_included(tx_hash, inclusion_time);
                        debug!(
                            "Transaction included: {:?} in {:?}",
                            tx_hash, inclusion_time
                        );
                    }
                    Ok(None) => {
                        // Transaction not yet included, continue polling
                    }
                    Err(e) => {
                        debug!("Error fetching receipt for {:?}: {}", tx_hash, e);
                        // Don't mark as timeout, might be temporary RPC error
                    }
                }
            }

            // Exit when all transactions resolved and test completed
            if self.tracker.all_resolved() && self.tracker.is_test_completed() {
                break;
            }
        }

        Ok(())
    }
}
