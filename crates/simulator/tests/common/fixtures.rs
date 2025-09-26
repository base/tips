/// Test fixtures and pre-configured test data
use alloy_primitives::Bytes;
use alloy_rpc_types_mev::EthSendBundle;
use std::sync::LazyLock;

/// Common test addresses
pub mod addresses {
    use alloy_primitives::Address;
    use std::sync::LazyLock;
    
    pub static ALICE: LazyLock<Address> = LazyLock::new(|| "0x0000000000000000000000000000000000000001".parse().unwrap());
    pub static BOB: LazyLock<Address> = LazyLock::new(|| "0x0000000000000000000000000000000000000002".parse().unwrap());
    pub static CHARLIE: LazyLock<Address> = LazyLock::new(|| "0x0000000000000000000000000000000000000003".parse().unwrap());
    pub static CONTRACT_A: LazyLock<Address> = LazyLock::new(|| "0x1000000000000000000000000000000000000001".parse().unwrap());
    pub static CONTRACT_B: LazyLock<Address> = LazyLock::new(|| "0x1000000000000000000000000000000000000002".parse().unwrap());
}

/// Common test block hashes and numbers
pub mod blocks {
    use alloy_primitives::B256;
    use std::sync::LazyLock;
    
    pub const BLOCK_18M: u64 = 18_000_000;
    pub const BLOCK_18M_PLUS_1: u64 = 18_000_001;
    pub const BLOCK_18M_PLUS_2: u64 = 18_000_002;
    
    pub static HASH_18M: LazyLock<B256> = LazyLock::new(|| B256::from_slice(&[1u8; 32]));
    pub static HASH_18M_PLUS_1: LazyLock<B256> = LazyLock::new(|| B256::from_slice(&[2u8; 32]));
    pub static HASH_18M_PLUS_2: LazyLock<B256> = LazyLock::new(|| B256::from_slice(&[3u8; 32]));
}

/// Pre-built transaction fixtures
pub mod transactions {
    use alloy_primitives::Bytes;
    
    /// Simple transfer transaction (mock data)
    pub fn simple_transfer() -> Bytes {
        Bytes::from(vec![
            0x02, // EIP-1559 tx type
            0x01, 0x02, 0x03, 0x04, // Mock transaction data
            0x05, 0x06, 0x07, 0x08,
        ])
    }
    
    /// Contract call transaction (mock data)
    pub fn contract_call() -> Bytes {
        Bytes::from(vec![
            0x02, // EIP-1559 tx type
            0x10, 0x20, 0x30, 0x40, // Mock contract call data
            0x50, 0x60, 0x70, 0x80,
        ])
    }
    
    /// Transaction that will revert (mock data)
    pub fn reverting_tx() -> Bytes {
        Bytes::from(vec![
            0x02, // EIP-1559 tx type
            0xFF, 0xFF, 0xFF, 0xFF, // Mock reverting transaction
        ])
    }
}


/// Pre-configured bundles for testing
pub mod bundles {
    use super::*;
    use crate::common::builders::TestBundleBuilder;
    
    /// Simple single transaction bundle
    pub fn single_tx_bundle() -> EthSendBundle {
        TestBundleBuilder::new()
            .with_transaction(transactions::simple_transfer())
            .with_block_number(blocks::BLOCK_18M)
            .build()
    }
    
    /// Bundle with multiple transactions
    pub fn multi_tx_bundle() -> EthSendBundle {
        TestBundleBuilder::new()
            .with_transaction(transactions::simple_transfer())
            .with_transaction(transactions::contract_call())
            .with_transaction(transactions::simple_transfer())
            .with_block_number(blocks::BLOCK_18M)
            .build()
    }
    
    /// Bundle with reverting transaction
    pub fn reverting_bundle() -> EthSendBundle {
        TestBundleBuilder::new()
            .with_transaction(transactions::simple_transfer())
            .with_transaction(transactions::reverting_tx())
            .with_block_number(blocks::BLOCK_18M)
            .build()
    }
    
    /// Large bundle for stress testing
    pub fn large_bundle(num_txs: usize) -> EthSendBundle {
        let mut builder = TestBundleBuilder::new()
            .with_block_number(blocks::BLOCK_18M);
        
        for i in 0..num_txs {
            let tx_data = vec![0x02, i as u8, 0x01, 0x02, 0x03];
            builder = builder.with_transaction(Bytes::from(tx_data));
        }
        
        builder.build()
    }
    
    /// Bundle with specific timing constraints
    pub fn time_constrained_bundle() -> EthSendBundle {
        TestBundleBuilder::new()
            .with_transaction(transactions::simple_transfer())
            .with_block_number(blocks::BLOCK_18M)
            .with_timestamps(1625097600, 1625097700) // 100 second window
            .build()
    }
}

/// Test scenarios combining multiple fixtures
pub mod scenarios {
    use super::*;
    use tips_simulator::types::SimulationRequest;
    use uuid::Uuid;
    
    /// Create a basic simulation scenario
    pub fn basic_simulation() -> SimulationRequest {
        let bundle = bundles::single_tx_bundle();
        SimulationRequest {
            bundle_id: Uuid::new_v4(),
            bundle,
            block_number: blocks::BLOCK_18M,
            block_hash: *blocks::HASH_18M,
        }
    }
    
    /// Create a contract interaction scenario
    pub fn contract_interaction() -> SimulationRequest {
        let bundle = bundles::multi_tx_bundle();
        SimulationRequest {
            bundle_id: Uuid::new_v4(),
            bundle,
            block_number: blocks::BLOCK_18M,
            block_hash: *blocks::HASH_18M,
        }
    }
    
    /// Create a large bundle scenario
    pub fn large_bundle_scenario() -> SimulationRequest {
        let bundle = bundles::large_bundle(100);
        SimulationRequest {
            bundle_id: Uuid::new_v4(),
            bundle,
            block_number: blocks::BLOCK_18M,
            block_hash: *blocks::HASH_18M,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_fixture_addresses() {
        assert_ne!(*addresses::ALICE, *addresses::BOB);
        assert_ne!(*addresses::CONTRACT_A, *addresses::CONTRACT_B);
    }
    
    #[test]
    fn test_fixture_bundles() {
        let single = bundles::single_tx_bundle();
        assert_eq!(single.txs.len(), 1);
        
        let multi = bundles::multi_tx_bundle();
        assert_eq!(multi.txs.len(), 3);
        
        let large = bundles::large_bundle(100);
        assert_eq!(large.txs.len(), 100);
    }
    
    #[test]
    fn test_fixture_scenarios() {
        let request = scenarios::basic_simulation();
        assert_eq!(request.block_number, blocks::BLOCK_18M);
        assert_eq!(request.bundle.txs.len(), 1);
        
        let interaction = scenarios::contract_interaction();
        assert_eq!(interaction.bundle.txs.len(), 3);
        
        let large_scenario = scenarios::large_bundle_scenario();
        assert_eq!(large_scenario.bundle.txs.len(), 100);
    }
}
