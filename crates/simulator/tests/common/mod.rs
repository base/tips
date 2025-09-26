#![allow(dead_code)]

/// Common test utilities and infrastructure for simulator testing

pub mod builders;
pub mod fixtures;
pub mod mock_bundle_simulator;
pub mod mocks;

use alloy_primitives::{Address, Bytes, B256, U256};
use alloy_rpc_types_mev::EthSendBundle;
use std::collections::HashMap;
use tips_simulator::types::{SimulationRequest, SimulationResult};
use uuid::Uuid;


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
pub fn create_success_result(bundle_id: Uuid, gas_used: u64) -> SimulationResult {
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
        assert!(
            result.gas_used.is_some(),
            "Successful simulation should have gas_used"
        );
        assert!(
            result.error_reason.is_none(),
            "Successful simulation should not have error"
        );
    }


}

