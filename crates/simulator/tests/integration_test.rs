use tips_simulator::types::SimulationRequest;
use tips_simulator::MempoolSimulatorConfig;
use alloy_primitives::{Bytes, B256};
use alloy_rpc_types_mev::EthSendBundle;
use uuid::Uuid;

// Basic smoke test to ensure the core simulation types work correctly
// Tests both mempool event simulation and ExEx event simulation architectures

#[test]
fn test_simulation_request_creation() {
    let bundle_id = Uuid::new_v4();
    let bundle = EthSendBundle {
        txs: vec![
            Bytes::from_static(&[0x01, 0x02, 0x03]), // Mock transaction data
        ],
        block_number: 18_000_000,
        min_timestamp: Some(1625097600),
        max_timestamp: Some(1625097900),
        reverting_tx_hashes: vec![],
        replacement_uuid: None,
        dropping_tx_hashes: vec![],
        refund_percent: None,
        refund_recipient: None,
        refund_tx_hashes: vec![],
        extra_fields: Default::default(),
    };

    let request = SimulationRequest {
        bundle_id,
        bundle: bundle.clone(),
        block_number: 18_000_000,
        block_hash: B256::ZERO,
    };

    assert_eq!(request.bundle_id, bundle_id);
    assert_eq!(request.bundle.txs.len(), 1);
    assert_eq!(request.block_number, 18_000_000);
}

// Test mempool simulator configuration creation
#[test]
fn test_mempool_simulator_config() {
    let config = MempoolSimulatorConfig {
        kafka_brokers: vec!["localhost:9092".to_string()],
        kafka_topic: "mempool-events".to_string(),
        kafka_group_id: "tips-simulator".to_string(),
        database_url: "postgresql://user:pass@localhost:5432/tips".to_string(),
    };

    assert_eq!(config.kafka_brokers, vec!["localhost:9092"]);
    assert_eq!(config.kafka_topic, "mempool-events");
    assert_eq!(config.kafka_group_id, "tips-simulator");
    assert_eq!(config.database_url, "postgresql://user:pass@localhost:5432/tips");
}

// Future integration tests would test both:
// 1. Mempool event simulation (Kafka-based)
// 2. ExEx event simulation
