use crate::tracker::TransactionTracker;
use crate::wallet::Wallet;
use alloy_network::Network;
use alloy_primitives::{Address, Bytes, keccak256};
use alloy_provider::{Provider, RootProvider};
use anyhow::{Context, Result};
use op_alloy_network::Optimism;
use rand::Rng;
use rand_chacha::ChaCha8Rng;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tips_e2e_tests::client::TipsRpcClient;
use tips_e2e_tests::fixtures::create_load_test_transaction;

pub struct SenderTask<N: Network> {
    wallet: Wallet,
    client: TipsRpcClient<N>,
    sequencer: RootProvider<Optimism>,
    rate_per_wallet: f64,
    duration: Duration,
    tracker: Arc<TransactionTracker>,
    rng: ChaCha8Rng,
}

impl<N: Network> SenderTask<N> {
    pub fn new(
        wallet: Wallet,
        client: TipsRpcClient<N>,
        sequencer: RootProvider<Optimism>,
        rate_per_wallet: f64,
        duration: Duration,
        tracker: Arc<TransactionTracker>,
        rng: ChaCha8Rng,
    ) -> Self {
        Self {
            wallet,
            client,
            sequencer,
            rate_per_wallet,
            duration,
            tracker,
            rng,
        }
    }

    pub async fn run(mut self) -> Result<()> {
        let mut nonce = self
            .sequencer
            .get_transaction_count(self.wallet.address)
            .await
            .context("Failed to get initial nonce")?;

        let interval_duration = Duration::from_secs_f64(1.0 / self.rate_per_wallet);
        let mut ticker = tokio::time::interval(interval_duration);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        let deadline = Instant::now() + self.duration;

        while Instant::now() < deadline {
            ticker.tick().await;

            let recipient = self.random_address();
            let tx_bytes = self.create_transaction(recipient, nonce)?;
            let tx_hash = keccak256(&tx_bytes);
            let send_time = Instant::now();

            match self.client.send_raw_transaction(tx_bytes).await {
                Ok(_) => {
                    self.tracker.record_sent(tx_hash, send_time);
                    nonce += 1;
                }
                Err(_) => {
                    self.tracker.record_send_error();
                    // Don't increment nonce on error, might retry
                }
            }
        }

        Ok(())
    }

    fn create_transaction(&self, to: Address, nonce: u64) -> Result<Bytes> {
        create_load_test_transaction(&self.wallet.signer, to, nonce)
    }

    fn random_address(&mut self) -> Address {
        let mut bytes = [0u8; 20];
        self.rng.fill(&mut bytes);
        Address::from(bytes)
    }
}
