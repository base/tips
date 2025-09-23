use crate::types::{SimulationError, SimulationRequest, SimulationResult};
use alloy_consensus::{transaction::SignerRecoverable, BlockHeader};
use alloy_primitives::B256;
use alloy_eips::eip2718::Decodable2718;
use alloy_rpc_types::BlockNumberOrTag;
use eyre::Result;
use async_trait::async_trait;
use reth_node_api::FullNodeComponents;
use reth_provider::{StateProvider, StateProviderFactory, HeaderProvider};
use reth_revm::{database::StateProviderDatabase, db::State};
use reth_evm::ConfigureEvm;
use reth_evm::NextBlockEnvAttributes;
use reth_evm::execute::BlockBuilder;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tracing::{error, info};
use uuid::Uuid;

// FIXME: The block time should be retrieved from the reth node.
const BLOCK_TIME: u64 = 2;

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
        request: &SimulationRequest,
        state_provider: &S,
    ) -> Result<SimulationResult>
    where
        S: StateProvider + Send + Sync;
}

#[derive(Clone)]
pub struct RethSimulationEngine<Node>
where
    Node: FullNodeComponents,
{
    provider: Arc<Node::Provider>,
    evm_config: Node::Evm,
}


impl<Node> RethSimulationEngine<Node>
where
    Node: FullNodeComponents,
{
    pub fn new(provider: Arc<Node::Provider>, evm_config: Node::Evm) -> Self {
        Self {
            provider,
            evm_config,
        }
    }

}

#[async_trait]
impl<Node> SimulationEngine for RethSimulationEngine<Node>
where
    Node: FullNodeComponents,
    <Node as FullNodeComponents>::Evm: ConfigureEvm<NextBlockEnvCtx = NextBlockEnvAttributes>,
{
    async fn simulate_bundle<S>(
        &self,
        request: &SimulationRequest,
        _state_provider: &S,
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

        // Get the parent header for building the next block
        let header = self
            .provider
            .sealed_header_by_hash(request.block_hash)
            .map_err(|e| eyre::eyre!("Failed to get parent header: {}", e))?
            .ok_or_else(|| eyre::eyre!("Parent block {} not found", request.block_hash))?;

        // Create the state database and builder for next block
        let state_provider = self.provider.state_by_block_hash(request.block_hash)?;
        let state_db = StateProviderDatabase::new(state_provider);
        let mut db = State::builder().with_database(state_db).with_bundle_update().build();
        let attributes = NextBlockEnvAttributes {
            timestamp: header.timestamp() + BLOCK_TIME,  // Optimism 2-second block time
            suggested_fee_recipient: header.beneficiary(),
            prev_randao: B256::random(),
            gas_limit: header.gas_limit(),
            parent_beacon_block_root: header.parent_beacon_block_root(),
            withdrawals: None,
        };

        // Variables to track bundle execution
        let mut total_gas_used = 0u64;
        let mut failed = false;
        let mut failure_reason = None;

        // Apply pre-execution changes and simulate transactions in a scope
        // to ensure builder is dropped before we call take_bundle()
        {
            // NOTE: We use the reth block builder here, which diverges from op-rbuilder. It's
            // not yet clear which builder we want to simulate with, so we're using reth because
            // it's easy.
            let mut builder = self
                .evm_config
                .builder_for_next_block(&mut db, &header, attributes)
                .map_err(|e| eyre::eyre!("Failed to init block builder: {}", e))?;
            builder.apply_pre_execution_changes().map_err(|e| eyre::eyre!("Failed pre-exec: {}", e))?;

            // Simulate each transaction in the bundle
            for (tx_index, tx_bytes) in request.bundle.txs.iter().enumerate() {
                // Decode bytes into the node's SignedTx type and recover the signer for execution
                type NodeSignedTxTy<Node> = 
                    <<<Node as reth_node_api::FullNodeTypes>::Types as reth_node_api::NodeTypes>::Primitives as reth_node_api::NodePrimitives>::SignedTx;
                let mut reader = tx_bytes.iter().as_slice();
                let signed: NodeSignedTxTy<Node> = Decodable2718::decode_2718(&mut reader)
                    .map_err(|e| eyre::eyre!("Failed to decode tx {tx_index}: {e}"))?;
                let recovered = signed
                    .try_into_recovered()
                    .map_err(|e| eyre::eyre!("Failed to recover tx {tx_index}: {e}"))?;

                match builder.execute_transaction(recovered) {
                    Ok(gas_used) => {
                        total_gas_used = total_gas_used.saturating_add(gas_used);
                    }
                    Err(e) => {
                        failed = true;
                        failure_reason = Some(SimulationError::Unknown { message: format!("Execution failed: {}", e) });
                        break;
                    }
                }
            }
        }

        let execution_time = start_time.elapsed().as_micros();

        if failed {
            error!(
                bundle_id = %request.bundle_id,
                simulation_id = %simulation_id,
                error = ?failure_reason,
                "Bundle simulation failed"
            );

            Ok(SimulationResult::failure(
                simulation_id,
                request.bundle_id,
                request.block_number,
                request.block_hash,
                execution_time,
                failure_reason.unwrap_or(SimulationError::Unknown {
                    message: "Unknown failure".to_string(),
                }),
            ))
        } else {
            // Collect the state diff
            let bundle = db.take_bundle();
            
            // Extract storage changes from the bundle
            let mut modified_storage_slots = HashMap::new();
            for (address, account) in bundle.state() {
                let mut storage_changes = HashMap::new();
                for (slot, slot_value) in account.storage.iter() {
                    // Only include modified slots (non-zero values or explicitly set to zero)
                    if slot_value.present_value != slot_value.original_value() {
                        storage_changes.insert(*slot, slot_value.present_value);
                    }
                }
                if !storage_changes.is_empty() {
                    modified_storage_slots.insert(*address, storage_changes);
                }
            }

            info!(
                bundle_id = %request.bundle_id,
                simulation_id = %simulation_id,
                gas_used = total_gas_used,
                execution_time_us = execution_time,
                storage_changes = modified_storage_slots.len(),
                "Bundle simulation completed successfully"
            );

            Ok(SimulationResult::success(
                simulation_id,
                request.bundle_id,
                request.block_number,
                request.block_hash,
                total_gas_used,
                execution_time,
                modified_storage_slots,
            ))
        }
    }
}
