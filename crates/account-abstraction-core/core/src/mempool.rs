use alloy_primitives::{Address, B256, U256};
use anyhow::Result;
use rundler_pool::{LocalPoolBuilder, LocalPoolHandle, PoolConfig, PoolTask, PoolTaskArgs};
use rundler_sim::PrecheckSettings;
use rundler_sim::simulation::Settings as SimulationSettings;
use rundler_types::{
    EntryPointVersion, PriorityFeeMode, UserOperationId, UserOperationVariant, chain::ChainSpec,
};
use rundler_types::pool::Pool;
use std::collections::HashMap;
use tokio::sync::broadcast;
use tracing::info;

pub struct MempoolConfig {
    pub rpc_url: String,
    pub chain_id: u64,
    pub entry_point_address: Address,
    pub entry_point_version: EntryPointVersion,
}

impl Default for MempoolConfig {
    fn default() -> Self {
        Self {
            rpc_url: "http://localhost:8545".to_string(),
            chain_id: 1,
            entry_point_address: Address::ZERO,
            entry_point_version: EntryPointVersion::V0_7,
        }
    }
}

pub struct SimpleMempool {
    config: MempoolConfig,
    pool_handle: Option<LocalPoolHandle>,
}

impl SimpleMempool {
    pub fn new(config: MempoolConfig) -> Self {
        Self {
            config,
            pool_handle: None,
        }
    }

    pub async fn initialize(&mut self) -> Result<()> {
        info!(
            "Initializing mempool with chain_id: {}",
            self.config.chain_id
        );

        let chain_spec = ChainSpec {
            id: self.config.chain_id,
            ..Default::default()
        };

        let pool_config = PoolConfig {
            chain_spec: chain_spec.clone(),
            entry_point: self.config.entry_point_address,
            entry_point_version: self.config.entry_point_version,
            same_sender_mempool_count: 4,
            min_replacement_fee_increase_percentage: 10,
            max_size_of_pool_bytes: 500_000_000,
            blocklist: None,
            allowlist: None,
            precheck_settings: PrecheckSettings {
                max_verification_gas: 5_000_000,
                max_bundle_execution_gas: 1_000_000_000,
                bundle_priority_fee_overhead_percent: 0,
                base_fee_accept_percent: 50,
                max_uo_cost: U256::from(10u128.pow(18)),
                priority_fee_mode: PriorityFeeMode::BaseFeePercent(0),
                pre_verification_gas_accept_percent: 100,
                verification_gas_limit_efficiency_reject_threshold: 10.0,
            },
            sim_settings: SimulationSettings {
                min_stake_value: U256::from(10u128.pow(18)),
                min_unstake_delay: 84600,
                tracer_timeout: "5s".to_string(),
                enable_unsafe_fallback: false,
            },
            mempool_channel_configs: HashMap::from([(B256::ZERO, Default::default())]),
            reputation_tracking_enabled: true,
            paymaster_tracking_enabled: true,
            da_gas_tracking_enabled: false,
            drop_min_num_blocks: 10,
            throttled_entity_mempool_count: 4,
            throttled_entity_live_blocks: 10,
            paymaster_cache_length: 10000,
            max_expected_storage_slots: 100,
            execution_gas_limit_efficiency_reject_threshold: 10.0,
            verification_gas_limit_efficiency_reject_threshold: 10.0,
            max_time_in_pool: Some(std::time::Duration::from_secs(3600)),
        };

        let (event_tx, _event_rx) = broadcast::channel(1000);
        let pool_builder = LocalPoolBuilder::new(100);

        let pool_task_args = PoolTaskArgs {
            chain_spec: chain_spec.clone(),
            http_url: self.config.rpc_url.clone(),
            chain_max_sync_retries: 3,
            chain_poll_interval: std::time::Duration::from_secs(1),
            pool_configs: vec![pool_config],
            remote_address: None,
            chain_update_channel_capacity: 1000,
            unsafe_mode: false,
        };

        let _pool_task = PoolTask::new(pool_task_args, event_tx.clone(), pool_builder, event_tx);

        let pool_builder = LocalPoolBuilder::new(100);
        let handle = pool_builder.get_handle();

        self.pool_handle = Some(handle);

        info!("Mempool initialized successfully");

        Ok(())
    }

    pub async fn add_operation(&self, op: UserOperationVariant) -> Result<B256> {
        let handle = self
            .pool_handle
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Pool not initialized"))?;

        // Default permissions: no special limits or sponsorship.
        let perms = rundler_types::UserOperationPermissions::default();

        let hash = handle
            .add_op(op, perms)
            .await
            .map_err(anyhow::Error::from)?;

        Ok(hash)
    }

    pub async fn get_operations(&self, _max_ops: u64) -> Result<Vec<UserOperationVariant>> {
        let _pool_builder = self
            .pool_handle
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Pool not initialized"))?;

        anyhow::bail!(
            "get_operations not yet fully implemented - need to determine correct LocalPoolHandle API"
        )
    }

    pub async fn remove_operation(&self, _op_id: UserOperationId) -> Result<()> {
        let _pool_builder = self
            .pool_handle
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Pool not initialized"))?;

        anyhow::bail!(
            "remove_operation not yet fully implemented - need to determine correct LocalPoolHandle API"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mempool_creation() {
        let config = MempoolConfig::default();
        let mempool = SimpleMempool::new(config);
        assert!(mempool.pool_handle.is_some());
    }
}
