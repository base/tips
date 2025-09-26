/// Common test utilities and infrastructure for simulator testing
pub mod builders;
pub mod fixtures;
pub mod mocks;
pub mod mock_bundle_simulator;

use alloy_primitives::{Address, Bytes, B256, U256};
use alloy_rpc_types_mev::EthSendBundle;
use std::collections::HashMap;
use tips_simulator::types::{SimulationRequest, SimulationResult};
use uuid::Uuid;

/// Test configuration that can be shared across tests
pub struct TestConfig {
    pub default_block_number: u64,
    pub default_gas_limit: u64,
    pub simulation_timeout_ms: u64,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            default_block_number: 18_000_000,
            default_gas_limit: 30_000_000,
            simulation_timeout_ms: 5000,
        }
    }
}

/// Helper to create a simple test bundle
pub fn create_test_bundle(num_txs: usize, block_number: u64) -> EthSendBundle {
    let mut txs = Vec::new();
    for i in 0..num_txs {
        // Create simple transaction bytes (not valid transactions, but good for testing)
        let tx_bytes = vec![0x01, 0x02, 0x03, i as u8];
        txs.push(Bytes::from(tx_bytes));
    }

    EthSendBundle {
        txs,
        block_number,
        min_timestamp: Some(1625097600),
        max_timestamp: Some(1625097900),
        reverting_tx_hashes: vec![],
        replacement_uuid: None,
        dropping_tx_hashes: vec![],
        refund_percent: None,
        refund_recipient: None,
        refund_tx_hashes: vec![],
        extra_fields: Default::default(),
    }
}

/// Helper to create a test simulation request
pub fn create_test_request(bundle: EthSendBundle) -> SimulationRequest {
    SimulationRequest {
        bundle_id: Uuid::new_v4(),
        bundle,
        block_number: 18_000_000,
        block_hash: B256::random(),
    }
}

/// Helper to create a successful simulation result
pub fn create_success_result(
    bundle_id: Uuid,
    gas_used: u64,
) -> SimulationResult {
    let mut state_diff = HashMap::new();
    let address = Address::random();
    let mut storage = HashMap::new();
    storage.insert(U256::from(1), U256::from(100));
    state_diff.insert(address, storage);

    SimulationResult::success(
        Uuid::new_v4(),
        bundle_id,
        18_000_000,
        B256::random(),
        gas_used,
        1500, // execution time in microseconds
        state_diff,
    )
}

/// Test assertion helpers
pub mod assertions {
    use super::*;

    /// Assert that a simulation result is successful
    pub fn assert_simulation_success(result: &SimulationResult) {
        assert!(result.success, "Expected successful simulation");
        assert!(result.gas_used.is_some(), "Successful simulation should have gas_used");
        assert!(result.error_reason.is_none(), "Successful simulation should not have error");
    }

    /// Assert that a simulation result is a failure
    pub fn assert_simulation_failure(result: &SimulationResult) {
        assert!(!result.success, "Expected failed simulation");
        assert!(result.gas_used.is_none(), "Failed simulation should not have gas_used");
        assert!(result.error_reason.is_some(), "Failed simulation should have error reason");
    }

    /// Assert state diff contains expected changes
    pub fn assert_state_diff_contains(
        result: &SimulationResult,
        address: Address,
        slot: U256,
        expected_value: U256,
    ) {
        let storage = result.state_diff.get(&address)
            .expect("Address not found in state diff");
        let value = storage.get(&slot)
            .expect("Storage slot not found");
        assert_eq!(*value, expected_value, "Unexpected storage value");
    }
}

/// Test timing utilities
pub mod timing {
    use std::time::{Duration, Instant};

    /// Measure execution time of an async operation
    pub async fn measure_async<F, T>(f: F) -> (T, Duration)
    where
        F: std::future::Future<Output = T>,
    {
        let start = Instant::now();
        let result = f.await;
        (result, start.elapsed())
    }

    /// Assert that an operation completes within a timeout
    pub async fn assert_completes_within<F, T>(
        f: F,
        timeout: Duration,
    ) -> T
    where
        F: std::future::Future<Output = T>,
    {
        tokio::time::timeout(timeout, f)
            .await
            .expect("Operation timed out")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_test_bundle() {
        let bundle = create_test_bundle(3, 18_000_000);
        assert_eq!(bundle.txs.len(), 3);
        assert_eq!(bundle.block_number, 18_000_000);
    }

    #[test]
    fn test_create_success_result() {
        let bundle_id = Uuid::new_v4();
        let result = create_success_result(bundle_id, 150_000);
        
        assertions::assert_simulation_success(&result);
        assert_eq!(result.bundle_id, bundle_id);
        assert_eq!(result.gas_used, Some(150_000));
    }
}
