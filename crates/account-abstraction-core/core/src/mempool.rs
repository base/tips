 use std::time::Duration;

use alloy_primitives::{Address, Bytes};
use anyhow::Error;
use async_trait::async_trait;
use rundler_pool::{PoolInner, PoolInnerConfig, PoolEvent};            
use rundler_provider::{BlockHashOrNumber, DAGasOracle, DAGasOracleSync, ProviderResult};
use rundler_types::chain::ChainSpec;
use rundler_types::da::{DAGasBlockData, DAGasData};
use rundler_utils::emit::WithEntryPoint;
use tokio::sync::broadcast;

use crate::VersionedUserOperation;

#[allow(dead_code)]
struct NoopDAGasOracle;

#[async_trait]
impl DAGasOracle for NoopDAGasOracle {
    async fn estimate_da_gas(
        &self,
        _data: Bytes,
        _to: Address,
        _block: BlockHashOrNumber,
        _gas_price: u128,
        _extra_data_len: usize,
    ) -> ProviderResult<(u128, DAGasData, DAGasBlockData)> {
        Ok((0, DAGasData::Empty, DAGasBlockData::Empty))
    }
}

#[async_trait]
impl DAGasOracleSync for NoopDAGasOracle {
    async fn da_block_data(&self, _block: BlockHashOrNumber) -> ProviderResult<DAGasBlockData> {
        Ok(DAGasBlockData::Empty)
    }

    async fn da_gas_data(
        &self,
        _gas_data: Bytes,
        _to: Address,
        _block: BlockHashOrNumber,
    ) -> ProviderResult<DAGasData> {
        Ok(DAGasData::Empty)
    }

    fn calc_da_gas_sync(
        &self,
        _gas_data: &DAGasData,
        _block_data: &DAGasBlockData,
        _gas_price: u128,
        _extra_data_len: usize,
    ) -> u128 {
        0
    }
}


#[allow(dead_code)]
 fn create_pool() -> PoolInner<NoopDAGasOracle>{

    let config = PoolInnerConfig {
        chain_spec: ChainSpec::default(),
        entry_point: Address::ZERO,
        max_size_of_pool_bytes: 10 * 1024 * 1024,
        min_replacement_fee_increase_percentage: 10,
        throttled_entity_mempool_count: 4,
        throttled_entity_live_blocks: 10,
        da_gas_tracking_enabled: false,
        max_time_in_pool: Some(Duration::from_secs(3600)),
        verification_gas_limit_efficiency_reject_threshold: 0.5,
    };
      // 2. Create event channel
      let (event_sender, _event_receiver) =
          broadcast::channel::<WithEntryPoint<PoolEvent>>(1000);

     let da_gas_oracle: NoopDAGasOracle = NoopDAGasOracle;
     let pool = PoolInner::new(config, Some(da_gas_oracle), event_sender);
    return pool;
    
 }

  trait MempoolTrait {
    fn add_operation(&self, operation: VersionedUserOperation) -> Result<(), Error>;
    // fn remove_operation(&self, operation: VersionedUserOperation) -> Result<(), Error>;
    // fn get_top_n_operations(&self, n: usize) -> Result<Vec<VersionedUserOperation>, Error>;
    // fn mine_operation(&self, operation: VersionedUserOperation) -> Result<(), Error>;
 }

 struct Mempool {
    pool: PoolInner<NoopDAGasOracle>,
 }

 impl Mempool {
    pub fn new() -> Self {
        Self { pool: create_pool() }
    }
 }

 impl MempoolTrait for Mempool {
    fn add_operation(&self, operation: VersionedUserOperation) -> Result<(), Error> {
        self.pool.add_operation(operation, 0, 0);
        Ok(())
    }
 }