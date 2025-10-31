use alloy_consensus::Transaction;
use alloy_consensus::transaction::SignerRecoverable;
use alloy_primitives::{Address, B256, Bytes, TxHash, keccak256};
use alloy_provider::network::eip2718::{Decodable2718, Encodable2718};
use op_alloy_consensus::OpTxEnvelope;
use op_alloy_flz::tx_estimated_size_fjord_bytes;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Block time in microseconds
pub const BLOCK_TIME: u128 = 2_000_000;

pub struct BundleTransactions(Vec<Bytes>);

impl From<Vec<Bytes>> for BundleTransactions {
    fn from(txs: Vec<Bytes>) -> Self {
        BundleTransactions(txs)
    }
}

impl BundleTransactions {
    pub fn bundle_hash(&self) -> B256 {
        let mut concatenated = Vec::new();
        for tx in self.0.iter() {
            concatenated.extend_from_slice(tx);
        }
        keccak256(&concatenated)
    }

    /// Get transaction hashes for all transactions in the bundle
    pub fn txn_hashes(&self) -> Result<Vec<TxHash>, String> {
        self.transactions()?
            .iter()
            .map(|t| Ok(t.tx_hash()))
            .collect()
    }

    /// Get sender addresses for all transactions in the bundle
    pub fn senders(&self) -> Result<Vec<Address>, String> {
        self.transactions()?
            .iter()
            .map(|t| {
                t.recover_signer()
                    .map_err(|e| format!("failed to recover signer: {e}"))
            })
            .collect()
    }

    /// Get total gas limit for all transactions in the bundle
    pub fn gas_limit(&self) -> Result<u64, String> {
        Ok(self.transactions()?.iter().map(|t| t.gas_limit()).sum())
    }

    /// Get total data availability size for all transactions in the bundle
    pub fn da_size(&self) -> Result<u64, String> {
        Ok(self
            .transactions()?
            .iter()
            .map(|t| tx_estimated_size_fjord_bytes(&t.encoded_2718()))
            .sum())
    }

    /// Decode all transactions from bytes to OpTxEnvelope
    pub fn transactions(&self) -> Result<Vec<OpTxEnvelope>, String> {
        self.0
            .iter()
            .map(|b| {
                OpTxEnvelope::decode_2718_exact(b)
                    .map_err(|e| format!("failed to decode transaction: {e}"))
            })
            .collect()
    }
}

#[derive(Default, Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Bundle {
    pub txs: Vec<Bytes>,

    #[serde(with = "alloy_serde::quantity")]
    pub block_number: u64,

    #[serde(
        default,
        deserialize_with = "alloy_serde::quantity::opt::deserialize",
        skip_serializing_if = "Option::is_none"
    )]
    pub flashblock_number_min: Option<u64>,

    #[serde(
        default,
        deserialize_with = "alloy_serde::quantity::opt::deserialize",
        skip_serializing_if = "Option::is_none"
    )]
    pub flashblock_number_max: Option<u64>,

    #[serde(
        default,
        deserialize_with = "alloy_serde::quantity::opt::deserialize",
        skip_serializing_if = "Option::is_none"
    )]
    pub min_timestamp: Option<u64>,

    #[serde(
        default,
        deserialize_with = "alloy_serde::quantity::opt::deserialize",
        skip_serializing_if = "Option::is_none"
    )]
    pub max_timestamp: Option<u64>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reverting_tx_hashes: Vec<TxHash>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replacement_uuid: Option<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dropping_tx_hashes: Vec<TxHash>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleHash {
    pub bundle_hash: B256,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelBundle {
    pub replacement_uuid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleWithMetadata {
    bundle: Bundle,
    uuid: Uuid,
    meter_bundle_response: MeterBundleResponse,
}

impl BundleWithMetadata {
    pub fn load(
        mut bundle: Bundle,
        meter_bundle_response: MeterBundleResponse,
    ) -> Result<Self, String> {
        let uuid = bundle
            .replacement_uuid
            .clone()
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        let uuid = Uuid::parse_str(uuid.as_str()).map_err(|_| format!("Invalid UUID: {uuid}"))?;

        bundle.replacement_uuid = Some(uuid.to_string());

        Ok(BundleWithMetadata {
            bundle,
            uuid,
            meter_bundle_response,
        })
    }

    pub fn uuid(&self) -> &Uuid {
        &self.uuid
    }

    pub fn bundle(&self) -> &Bundle {
        &self.bundle
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionResult {
    pub coinbase_diff: String,
    pub eth_sent_to_coinbase: String,
    pub from_address: Address,
    pub gas_fees: String,
    pub gas_price: String,
    pub gas_used: u64,
    pub to_address: Option<Address>,
    pub tx_hash: TxHash,
    pub value: String,
    pub execution_time_us: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeterBundleResponse {
    pub bundle_gas_price: String,
    pub bundle_hash: B256,
    pub coinbase_diff: String,
    pub eth_sent_to_coinbase: String,
    pub gas_fees: String,
    pub results: Vec<TransactionResult>,
    pub state_block_number: u64,
    #[serde(
        default,
        deserialize_with = "alloy_serde::quantity::opt::deserialize",
        skip_serializing_if = "Option::is_none"
    )]
    pub state_flashblock_index: Option<u64>,
    pub total_gas_used: u64,
    pub total_execution_time_us: u128,
    pub state_root_time_us: u128,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{create_test_meter_bundle_response, create_transaction};
    use alloy_primitives::Keccak256;
    use alloy_provider::network::eip2718::Encodable2718;
    use alloy_signer_local::PrivateKeySigner;

    #[test]
    fn test_bundle_types() {
        let alice = PrivateKeySigner::random();
        let bob = PrivateKeySigner::random();

        let tx1 = create_transaction(alice.clone(), 1, bob.address());
        let tx2 = create_transaction(alice.clone(), 2, bob.address());

        let tx1_bytes = tx1.encoded_2718();
        let tx2_bytes = tx2.encoded_2718();

        let bundle = BundleWithMetadata::load(
            Bundle {
                replacement_uuid: None,
                txs: vec![tx1_bytes.clone().into()],
                block_number: 1,
                ..Default::default()
            },
            create_test_meter_bundle_response(),
        )
        .unwrap();

        assert!(!bundle.uuid().is_nil());
        assert_eq!(
            bundle.bundle.replacement_uuid,
            Some(bundle.uuid().to_string())
        );
        let bundle_txs: BundleTransactions = bundle.bundle().txs.clone().into();
        assert_eq!(bundle_txs.txn_hashes().unwrap().len(), 1);
        assert_eq!(bundle_txs.txn_hashes().unwrap()[0], tx1.tx_hash());
        assert_eq!(bundle_txs.senders().unwrap().len(), 1);
        assert_eq!(bundle_txs.senders().unwrap()[0], alice.address());

        // Bundle hashes are keccack256(...txnHashes)
        let expected_bundle_hash_single = {
            let mut hasher = Keccak256::default();
            hasher.update(keccak256(&tx1_bytes));
            hasher.finalize()
        };

        assert_eq!(bundle_txs.bundle_hash(), expected_bundle_hash_single);

        let uuid = Uuid::new_v4();
        let bundle = BundleWithMetadata::load(
            Bundle {
                replacement_uuid: Some(uuid.to_string()),
                txs: vec![tx1_bytes.clone().into(), tx2_bytes.clone().into()],
                block_number: 1,
                ..Default::default()
            },
            create_test_meter_bundle_response(),
        )
        .unwrap();

        assert_eq!(*bundle.uuid(), uuid);
        assert_eq!(bundle.bundle.replacement_uuid, Some(uuid.to_string()));
        let bundle_txs2: BundleTransactions = bundle.bundle().txs.clone().into();
        assert_eq!(bundle_txs2.txn_hashes().unwrap().len(), 2);
        assert_eq!(bundle_txs2.txn_hashes().unwrap()[0], tx1.tx_hash());
        assert_eq!(bundle_txs2.txn_hashes().unwrap()[1], tx2.tx_hash());
        assert_eq!(bundle_txs2.senders().unwrap().len(), 2);
        assert_eq!(bundle_txs2.senders().unwrap()[0], alice.address());
        assert_eq!(bundle_txs2.senders().unwrap()[1], alice.address());

        let expected_bundle_hash_double = {
            let mut hasher = Keccak256::default();
            hasher.update(keccak256(&tx1_bytes));
            hasher.update(keccak256(&tx2_bytes));
            hasher.finalize()
        };

        assert_eq!(bundle_txs2.bundle_hash(), expected_bundle_hash_double);
    }

    #[test]
    fn test_meter_bundle_response_serialization() {
        let response = MeterBundleResponse {
            bundle_gas_price: "1000000000".to_string(),
            bundle_hash: B256::default(),
            coinbase_diff: "100".to_string(),
            eth_sent_to_coinbase: "0".to_string(),
            gas_fees: "100".to_string(),
            results: vec![],
            state_block_number: 12345,
            state_flashblock_index: Some(42),
            total_gas_used: 21000,
            total_execution_time_us: 1000,
            state_root_time_us: 500,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"stateFlashblockIndex\":42"));
        assert!(json.contains("\"stateBlockNumber\":12345"));

        let deserialized: MeterBundleResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.state_flashblock_index, Some(42));
        assert_eq!(deserialized.state_block_number, 12345);
    }

    #[test]
    fn test_meter_bundle_response_without_flashblock_index() {
        let response = MeterBundleResponse {
            bundle_gas_price: "1000000000".to_string(),
            bundle_hash: B256::default(),
            coinbase_diff: "100".to_string(),
            eth_sent_to_coinbase: "0".to_string(),
            gas_fees: "100".to_string(),
            results: vec![],
            state_block_number: 12345,
            state_flashblock_index: None,
            total_gas_used: 21000,
            total_execution_time_us: 1000,
            state_root_time_us: 500,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(!json.contains("stateFlashblockIndex"));
        assert!(json.contains("\"stateBlockNumber\":12345"));

        let deserialized: MeterBundleResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.state_flashblock_index, None);
        assert_eq!(deserialized.state_block_number, 12345);
    }

    #[test]
    fn test_meter_bundle_response_deserialization_without_flashblock() {
        let json = r#"{
            "bundleGasPrice": "1000000000",
            "bundleHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
            "coinbaseDiff": "100",
            "ethSentToCoinbase": "0",
            "gasFees": "100",
            "results": [],
            "stateBlockNumber": 12345,
            "totalGasUsed": 21000,
            "totalExecutionTimeUs": 1000,
            "stateRootTimeUs": 500
        }"#;

        let deserialized: MeterBundleResponse = serde_json::from_str(json).unwrap();
        assert_eq!(deserialized.state_flashblock_index, None);
        assert_eq!(deserialized.state_block_number, 12345);
        assert_eq!(deserialized.total_gas_used, 21000);
    }
}
