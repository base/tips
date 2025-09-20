use crate::types::{SimulationError, SimulationRequest, SimulationResult};
use alloy_consensus::transaction::{SignerRecoverable, Transaction};
use alloy_primitives::{Address, B256, U256};
use alloy_eips::eip2718::Decodable2718;
use alloy_rpc_types::BlockNumberOrTag;
use eyre::Result;
use async_trait::async_trait;
use op_alloy_consensus::OpTxEnvelope;
use reth_provider::{StateProvider, StateProviderFactory};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::time::Duration;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Create state provider from ExEx context
///
/// This function prepares the necessary components for EVM simulation:
/// 1. Creates a StateProvider at a specific block using the Provider from ExEx context
/// 2. Validates that the block exists and retrieves its hash
/// 3. Returns the state provider that can be used for EVM database initialization
///
/// # Arguments
/// * `provider` - The state provider factory from the ExEx context (e.g., ctx.provider)
/// * `block_number` - The block number to create the state at
///
/// # Returns
/// A tuple of (StateProvider, block_hash) ready for EVM initialization
///
/// # Usage in ExEx
/// When implementing an ExEx that needs to simulate transactions, you can use this
/// function to get a state provider that implements the Client interface. This state
/// provider can then be used with reth's EvmConfig to create an EVM instance.
///
/// The typical flow is:
/// 1. Get the provider from ExExContext: `ctx.provider`
/// 2. Call this function to get a state provider at a specific block
/// 3. Use the state provider with reth_revm::database::StateProviderDatabase
/// 4. Configure the EVM with the appropriate EvmConfig from your node
pub fn prepare_evm_state<P>(
    provider: Arc<P>,
    block_number: u64,
) -> Result<(Box<dyn StateProvider>, B256)>
where
    P: StateProviderFactory,
{
    // Get the state provider at the specified block
    let state_provider = provider
        .state_by_block_number_or_tag(BlockNumberOrTag::Number(block_number))
        .map_err(|e| eyre::eyre!("Failed to get state provider at block {}: {}", block_number, e))?;
    
    // Get the block hash
    let block_hash = state_provider
        .block_hash(block_number)
        .map_err(|e| eyre::eyre!("Failed to get block hash: {}", e))?
        .ok_or_else(|| eyre::eyre!("Block {} not found", block_number))?;
    
    Ok((state_provider, block_hash))
}

/// Example usage within an ExEx:
/// ```ignore
/// // In your ExEx implementation
/// use reth_exex::ExExContext;
/// use reth_revm::database::StateProviderDatabase;
/// use revm::Evm;
/// 
/// // Get provider from ExEx context
/// let provider = ctx.provider.clone();
/// 
/// // Prepare EVM state
/// let (state_provider, block_hash) = prepare_evm_state::<Node>(
///     provider.clone(),
///     block_number,
/// )?;
/// 
/// // Create state database
/// let db = StateProviderDatabase::new(state_provider);
/// 
/// // Build EVM with the database
/// // Note: You would configure the EVM with proper environment settings
/// // based on your chain's requirements (gas limits, fork settings, etc.)
/// let evm = Evm::builder()
///     .with_db(db)
///     .build();
/// ```

#[async_trait]
pub trait SimulationEngine: Send + Sync {
    /// Simulate a bundle execution
    async fn simulate_bundle<S>(
        &self,
        request: SimulationRequest,
        state_provider: &S,
    ) -> Result<SimulationResult>
    where
        S: StateProvider + Send + Sync;
}

#[derive(Clone)]
pub struct RethSimulationEngine {
    timeout: Duration,
}

/// Represents the execution context for a bundle simulation
#[derive(Debug)]
struct ExecutionContext {
    /// Block number for simulation
    block_number: u64,
    /// Initial balances of involved accounts
    initial_balances: HashMap<Address, U256>,
    /// Initial nonces of involved accounts
    initial_nonces: HashMap<Address, u64>,
    /// Storage changes during simulation
    storage_changes: HashMap<Address, HashMap<U256, U256>>,
    /// Gas used so far
    gas_used: u64,
}

impl RethSimulationEngine {
    pub fn new(timeout_ms: u64) -> Self {
        Self {
            timeout: Duration::from_millis(timeout_ms),
        }
    }

    /// Extract transaction details from raw transaction bytes
    fn decode_transaction(&self, tx_bytes: &[u8]) -> Result<OpTxEnvelope> {
        OpTxEnvelope::decode_2718_exact(tx_bytes)
            .map_err(|e| eyre::eyre!("Failed to decode transaction: {}", e))
    }

    /// Validate that a transaction can be executed in the current context
    fn validate_transaction(
        &self,
        tx: &OpTxEnvelope,
        context: &ExecutionContext,
    ) -> Result<(), SimulationError> {
        let sender = tx.recover_signer()
            .map_err(|_| SimulationError::Unknown { 
                message: "Failed to recover transaction sender".to_string() 
            })?;

        // Check nonce
        let expected_nonce = context.initial_nonces.get(&sender)
            .copied()
            .unwrap_or(0);
        let tx_nonce = tx.nonce();

        if tx_nonce != expected_nonce {
            return Err(SimulationError::InvalidNonce {
                tx_index: 0, // TODO: Pass actual tx index
                expected: expected_nonce,
                actual: tx_nonce,
            });
        }

        // Check balance for gas payment
        let gas_fee = U256::from(tx.gas_limit()) * U256::from(tx.max_fee_per_gas());
        let available_balance = context.initial_balances.get(&sender)
            .copied()
            .unwrap_or(U256::ZERO);

        if available_balance < gas_fee {
            return Err(SimulationError::InsufficientBalance {
                tx_index: 0, // TODO: Pass actual tx index
                required: gas_fee,
                available: available_balance,
            });
        }

        Ok(())
    }

    /// Simulate a single transaction execution
    fn simulate_transaction(
        &self,
        tx: &OpTxEnvelope,
        context: &mut ExecutionContext,
        tx_index: usize,
    ) -> Result<(), SimulationError> {
        // For now, this is a placeholder implementation
        // In a full implementation, this would:
        // 1. Create an EVM instance with the current state
        // 2. Execute the transaction
        // 3. Track gas usage and state changes
        // 4. Handle reverts appropriately

        debug!(
            tx_index = tx_index,
            tx_hash = ?tx.hash(),
            gas_limit = tx.gas_limit(),
            "Simulating transaction"
        );

        // Validate the transaction first
        self.validate_transaction(tx, context)?;

        // Simulate gas usage (placeholder logic)
        let estimated_gas = std::cmp::min(tx.gas_limit(), 100_000); // Simple estimation
        context.gas_used += estimated_gas;

        // Simulate some state changes (placeholder)
        if let Some(to) = tx.to() {
            let storage_slot = U256::from(tx_index);
            let new_value = U256::from(context.gas_used);
            
            context.storage_changes
                .entry(Address::from(*to))
                .or_insert_with(HashMap::new)
                .insert(storage_slot, new_value);
        }

        // Update nonce for sender
        let sender = tx.recover_signer()
            .map_err(|_| SimulationError::Unknown { 
                message: "Failed to recover sender".to_string() 
            })?;
        
        if let Some(nonce) = context.initial_nonces.get_mut(&sender) {
            *nonce += 1;
        }

        debug!(
            tx_index = tx_index,
            gas_used = estimated_gas,
            total_gas = context.gas_used,
            "Transaction simulation completed"
        );

        Ok(())
    }

    /// Initialize execution context by fetching initial state
    fn initialize_context<S>(
        &self,
        request: &SimulationRequest,
        state_provider: &S,
    ) -> Result<ExecutionContext>
    where
        S: StateProvider,
    {
        let mut initial_balances = HashMap::new();
        let mut initial_nonces = HashMap::new();
        
        // Extract all addresses involved in the bundle
        let mut addresses = std::collections::HashSet::new();
        
        for tx_bytes in &request.bundle.txs {
            match self.decode_transaction(tx_bytes) {
                Ok(tx) => {
                    if let Ok(sender) = tx.recover_signer() {
                        addresses.insert(sender);
                    }
                    if let Some(to) = tx.to() {
                        addresses.insert(Address::from(*to));
                    }
                }
                Err(e) => {
                    warn!(error = %e, "Failed to decode transaction in bundle");
                }
            }
        }

        // Fetch initial state for all addresses
        for address in addresses {
            match state_provider.account_balance(&address) {
                Ok(Some(balance)) => {
                    initial_balances.insert(address, balance);
                }
                Ok(None) => {
                    initial_balances.insert(address, U256::ZERO);
                }
                Err(e) => {
                    error!(
                        error = %e,
                        address = %address,
                        "Failed to fetch balance for address"
                    );
                }
            }

            match state_provider.account_nonce(&address) {
                Ok(Some(nonce)) => {
                    initial_nonces.insert(address, nonce);
                }
                Ok(None) => {
                    initial_nonces.insert(address, 0);
                }
                Err(e) => {
                    error!(
                        error = %e,
                        address = %address,
                        "Failed to fetch nonce for address"
                    );
                }
            }
        }

        Ok(ExecutionContext {
            block_number: request.block_number,
            initial_balances,
            initial_nonces,
            storage_changes: HashMap::new(),
            gas_used: 0,
        })
    }
}

#[async_trait]
impl SimulationEngine for RethSimulationEngine {
    async fn simulate_bundle<S>(
        &self,
        request: SimulationRequest,
        state_provider: &S,
    ) -> Result<SimulationResult>
    where
        S: StateProvider + Send + Sync,
    {
        let start_time = Instant::now();
        let simulation_id = Uuid::new_v4();

        info!(
            bundle_id = %request.bundle_id,
            simulation_id = %simulation_id,
            num_transactions = request.bundle.txs.len(),
            block_number = request.block_number,
            "Starting bundle simulation"
        );

        // Initialize execution context
        let mut context = self.initialize_context(&request, state_provider)
            .map_err(|e| eyre::eyre!("Failed to initialize context: {}", e))?;

        // Simulate each transaction in the bundle
        for (tx_index, tx_bytes) in request.bundle.txs.iter().enumerate() {
            let tx = self.decode_transaction(tx_bytes)
                .map_err(|e| SimulationError::Unknown { 
                    message: format!("Failed to decode transaction {}: {}", tx_index, e) 
                })?;

            if let Err(sim_error) = self.simulate_transaction(&tx, &mut context, tx_index) {
                let execution_time = start_time.elapsed().as_micros();
                
                error!(
                    bundle_id = %request.bundle_id,
                    simulation_id = %simulation_id,
                    tx_index = tx_index,
                    error = %sim_error,
                    "Bundle simulation failed"
                );

                return Ok(SimulationResult::failure(
                    simulation_id,
                    request.bundle_id,
                    request.block_number,
                    request.block_hash,
                    execution_time,
                    sim_error,
                ));
            }
        }

        let execution_time = start_time.elapsed().as_micros();

        info!(
            bundle_id = %request.bundle_id,
            simulation_id = %simulation_id,
            gas_used = context.gas_used,
            execution_time_us = execution_time,
            storage_changes = context.storage_changes.len(),
            "Bundle simulation completed successfully"
        );

        Ok(SimulationResult::success(
            simulation_id,
            request.bundle_id,
            request.block_number,
            request.block_hash,
            context.gas_used,
            execution_time,
            context.storage_changes,
        ))
    }
}

/// Create a bundle simulation engine
pub fn create_simulation_engine(timeout_ms: u64) -> RethSimulationEngine {
    RethSimulationEngine::new(timeout_ms)
}
