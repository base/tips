/// Test data builders for creating complex test scenarios
use alloy_primitives::{Address, Bytes, B256, U256};
use alloy_rpc_types_mev::EthSendBundle;
use std::collections::HashMap;
use tips_simulator::types::{SimulationError, SimulationRequest, SimulationResult};
use uuid::Uuid;

/// Builder for creating test bundles with various configurations
pub struct TestBundleBuilder {
    txs: Vec<Bytes>,
    block_number: u64,
    min_timestamp: Option<u64>,
    max_timestamp: Option<u64>,
    reverting_tx_hashes: Vec<B256>,
    replacement_uuid: Option<String>,
}

impl TestBundleBuilder {
    pub fn new() -> Self {
        Self {
            txs: vec![],
            block_number: 18_000_000,
            min_timestamp: None,
            max_timestamp: None,
            reverting_tx_hashes: vec![],
            replacement_uuid: None,
        }
    }

    pub fn with_transaction(mut self, tx: Bytes) -> Self {
        self.txs.push(tx);
        self
    }

    pub fn with_simple_transaction(mut self, data: &[u8]) -> Self {
        self.txs.push(Bytes::from(data.to_vec()));
        self
    }

    pub fn with_block_number(mut self, block_number: u64) -> Self {
        self.block_number = block_number;
        self
    }

    pub fn with_timestamps(mut self, min: u64, max: u64) -> Self {
        self.min_timestamp = Some(min);
        self.max_timestamp = Some(max);
        self
    }



    pub fn build(self) -> EthSendBundle {
        EthSendBundle {
            txs: self.txs,
            block_number: self.block_number,
            min_timestamp: self.min_timestamp,
            max_timestamp: self.max_timestamp,
            reverting_tx_hashes: self.reverting_tx_hashes,
            replacement_uuid: self.replacement_uuid,
            dropping_tx_hashes: vec![],
            refund_percent: None,
            refund_recipient: None,
            refund_tx_hashes: vec![],
            extra_fields: Default::default(),
        }
    }
}

/// Builder for creating simulation requests
pub struct SimulationRequestBuilder {
    bundle_id: Option<Uuid>,
    bundle: Option<EthSendBundle>,
    block_number: u64,
    block_hash: Option<B256>,
}

impl SimulationRequestBuilder {
    pub fn new() -> Self {
        Self {
            bundle_id: None,
            bundle: None,
            block_number: 18_000_000,
            block_hash: None,
        }
    }

    pub fn with_bundle_id(mut self, id: Uuid) -> Self {
        self.bundle_id = Some(id);
        self
    }

    pub fn with_bundle(mut self, bundle: EthSendBundle) -> Self {
        self.bundle = Some(bundle);
        self
    }

    pub fn with_block(mut self, number: u64, hash: B256) -> Self {
        self.block_number = number;
        self.block_hash = Some(hash);
        self
    }

    pub fn build(self) -> SimulationRequest {
        SimulationRequest {
            bundle_id: self.bundle_id.unwrap_or_else(Uuid::new_v4),
            bundle: self.bundle.unwrap_or_else(|| {
                TestBundleBuilder::new()
                    .with_simple_transaction(&[0x01, 0x02, 0x03])
                    .build()
            }),
            block_number: self.block_number,
            block_hash: self.block_hash.unwrap_or_else(B256::random),
        }
    }
}

/// Builder for creating simulation results with specific characteristics
pub struct SimulationResultBuilder {
    id: Option<Uuid>,
    bundle_id: Option<Uuid>,
    block_number: u64,
    block_hash: Option<B256>,
    success: bool,
    gas_used: Option<u64>,
    execution_time_us: u128,
    state_diff: HashMap<Address, HashMap<U256, U256>>,
    error: Option<SimulationError>,
}

impl SimulationResultBuilder {
    pub fn new() -> Self {
        Self {
            id: None,
            bundle_id: None,
            block_number: 18_000_000,
            block_hash: None,
            success: true,
            gas_used: Some(150_000),
            execution_time_us: 1500,
            state_diff: HashMap::new(),
            error: None,
        }
    }

    pub fn successful() -> Self {
        Self::new()
    }

    pub fn failed() -> Self {
        let mut builder = Self::new();
        builder.success = false;
        builder.gas_used = None;
        builder.error = Some(SimulationError::Unknown {
            message: "Test failure".to_string(),
        });
        builder
    }

    pub fn with_ids(mut self, simulation_id: Uuid, bundle_id: Uuid) -> Self {
        self.id = Some(simulation_id);
        self.bundle_id = Some(bundle_id);
        self
    }

    pub fn with_gas_used(mut self, gas: u64) -> Self {
        self.gas_used = Some(gas);
        self
    }

    pub fn with_execution_time_us(mut self, time: u128) -> Self {
        self.execution_time_us = time;
        self
    }

    pub fn with_state_change(mut self, address: Address, slot: U256, value: U256) -> Self {
        self.state_diff
            .entry(address)
            .or_insert_with(HashMap::new)
            .insert(slot, value);
        self
    }

    pub fn with_error(mut self, error: SimulationError) -> Self {
        self.success = false;
        self.gas_used = None;
        self.error = Some(error);
        self
    }

    pub fn with_revert(self, reason: String) -> Self {
        self.with_error(SimulationError::Revert { reason })
    }

    pub fn with_out_of_gas(self) -> Self {
        self.with_error(SimulationError::OutOfGas)
    }

    pub fn with_invalid_nonce(self, tx_index: usize, expected: u64, actual: u64) -> Self {
        self.with_error(SimulationError::InvalidNonce {
            tx_index,
            expected,
            actual,
        })
    }

    pub fn build(self) -> SimulationResult {
        if self.success {
            SimulationResult::success(
                self.id.unwrap_or_else(Uuid::new_v4),
                self.bundle_id.unwrap_or_else(Uuid::new_v4),
                self.block_number,
                self.block_hash.unwrap_or_else(B256::random),
                self.gas_used.unwrap_or(150_000),
                self.execution_time_us,
                self.state_diff,
            )
        } else {
            SimulationResult::failure(
                self.id.unwrap_or_else(Uuid::new_v4),
                self.bundle_id.unwrap_or_else(Uuid::new_v4),
                self.block_number,
                self.block_hash.unwrap_or_else(B256::random),
                self.execution_time_us,
                self.error.unwrap_or(SimulationError::Unknown {
                    message: "Unknown error".to_string(),
                }),
            )
        }
    }
}

/// Builder for creating test scenarios with multiple bundles
pub struct ScenarioBuilder {
    bundles: Vec<EthSendBundle>,
    block_number: u64,
    block_hash: B256,
}

impl ScenarioBuilder {
    pub fn new() -> Self {
        Self {
            bundles: vec![],
            block_number: 18_000_000,
            block_hash: B256::random(),
        }
    }

    pub fn with_block(mut self, number: u64, hash: B256) -> Self {
        self.block_number = number;
        self.block_hash = hash;
        self
    }


    pub fn add_simple_bundle(mut self, num_txs: usize) -> Self {
        let mut builder = TestBundleBuilder::new().with_block_number(self.block_number);

        for i in 0..num_txs {
            builder = builder.with_simple_transaction(&[i as u8, 0x01, 0x02]);
        }

        self.bundles.push(builder.build());
        self
    }

    pub fn build_requests(self) -> Vec<SimulationRequest> {
        self.bundles
            .into_iter()
            .map(|bundle| {
                SimulationRequestBuilder::new()
                    .with_bundle(bundle)
                    .with_block(self.block_number, self.block_hash)
                    .build()
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bundle_builder() {
        let bundle = TestBundleBuilder::new()
            .with_simple_transaction(&[0x01, 0x02])
            .with_simple_transaction(&[0x03, 0x04])
            .with_block_number(18_500_000)
            .with_timestamps(1000, 2000)
            .build();

        assert_eq!(bundle.txs.len(), 2);
        assert_eq!(bundle.block_number, 18_500_000);
        assert_eq!(bundle.min_timestamp, Some(1000));
        assert_eq!(bundle.max_timestamp, Some(2000));
    }

    #[test]
    fn test_result_builder() {
        let bundle_id = Uuid::new_v4();
        let result = SimulationResultBuilder::successful()
            .with_ids(Uuid::new_v4(), bundle_id)
            .with_gas_used(200_000)
            .with_state_change(Address::random(), U256::from(1), U256::from(100))
            .build();

        assert!(result.success);
        assert_eq!(result.bundle_id, bundle_id);
        assert_eq!(result.gas_used, Some(200_000));
        assert!(!result.state_diff.is_empty());
    }

    #[test]
    fn test_scenario_builder() {
        let requests = ScenarioBuilder::new()
            .with_block(19_000_000, B256::random())
            .add_simple_bundle(2)
            .add_simple_bundle(3)
            .build_requests();

        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].block_number, 19_000_000);
        assert_eq!(requests[0].bundle.txs.len(), 2);
        assert_eq!(requests[1].bundle.txs.len(), 3);
    }
}
