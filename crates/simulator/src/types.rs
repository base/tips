use alloy_primitives::{Address, B256, U256};
use alloy_rpc_types_mev::EthSendBundle;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Result of simulating a complete bundle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
    /// Unique identifier for this simulation
    pub id: Uuid,
    /// Bundle that was simulated
    pub bundle_id: Uuid,
    /// Block number at which simulation was performed
    pub block_number: u64,
    /// Block hash at which simulation was performed
    pub block_hash: B256,
    /// Whether the bundle simulation was successful
    pub success: bool,
    /// Total gas used by all transactions in the bundle
    pub gas_used: Option<u64>,
    /// Time taken to execute the simulation in microseconds
    pub execution_time_us: u128,
    /// State changes produced by the bundle simulation
    /// Map of account address -> (storage slot -> new value)
    pub state_diff: HashMap<Address, HashMap<U256, U256>>,
    /// Error message if simulation failed
    pub error_reason: Option<String>,
    /// When this simulation was created
    pub created_at: DateTime<Utc>,
}

/// Configuration for ExEx-based simulation
#[derive(Debug, Clone)]
pub struct ExExSimulationConfig {
    /// PostgreSQL database connection URL
    pub database_url: String,
    /// Maximum number of concurrent simulations
    pub max_concurrent_simulations: usize,
    /// Timeout for individual simulations in milliseconds
    pub simulation_timeout_ms: u64,
}

/// Errors that can occur during simulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SimulationError {
    /// Bundle execution reverted
    Revert { reason: String },
    /// Bundle ran out of gas
    OutOfGas,
    /// Invalid nonce in one of the transactions
    InvalidNonce { tx_index: usize, expected: u64, actual: u64 },
    /// Insufficient balance for gas payment
    InsufficientBalance { tx_index: usize, required: U256, available: U256 },
    /// State access error (RPC failure, etc.)
    StateAccessError { message: String },
    /// Simulation timeout
    Timeout,
    /// Unknown error
    Unknown { message: String },
}

impl std::fmt::Display for SimulationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SimulationError::Revert { reason } => write!(f, "Bundle reverted: {}", reason),
            SimulationError::OutOfGas => write!(f, "Bundle ran out of gas"),
            SimulationError::InvalidNonce { tx_index, expected, actual } => {
                write!(f, "Invalid nonce in tx {}: expected {}, got {}", tx_index, expected, actual)
            }
            SimulationError::InsufficientBalance { tx_index, required, available } => {
                write!(f, "Insufficient balance in tx {}: required {}, available {}", tx_index, required, available)
            }
            SimulationError::StateAccessError { message } => write!(f, "State access error: {}", message),
            SimulationError::Timeout => write!(f, "Simulation timed out"),
            SimulationError::Unknown { message } => write!(f, "Unknown error: {}", message),
        }
    }
}

impl std::error::Error for SimulationError {}

/// A request to simulate a bundle
#[derive(Debug, Clone)]
pub struct SimulationRequest {
    pub bundle_id: Uuid,
    pub bundle: EthSendBundle,
    pub block_number: u64,
    pub block_hash: B256,
}

impl SimulationResult {
    /// Create a new successful simulation result
    pub fn success(
        id: Uuid,
        bundle_id: Uuid,
        block_number: u64,
        block_hash: B256,
        gas_used: u64,
        execution_time_us: u128,
        state_diff: HashMap<Address, HashMap<U256, U256>>,
    ) -> Self {
        Self {
            id,
            bundle_id,
            block_number,
            block_hash,
            success: true,
            gas_used: Some(gas_used),
            execution_time_us,
            state_diff,
            error_reason: None,
            created_at: Utc::now(),
        }
    }

    /// Create a new failed simulation result
    pub fn failure(
        id: Uuid,
        bundle_id: Uuid,
        block_number: u64,
        block_hash: B256,
        execution_time_us: u128,
        error: SimulationError,
    ) -> Self {
        Self {
            id,
            bundle_id,
            block_number,
            block_hash,
            success: false,
            gas_used: None,
            execution_time_us,
            state_diff: HashMap::new(),
            error_reason: Some(error.to_string()),
            created_at: Utc::now(),
        }
    }
}
