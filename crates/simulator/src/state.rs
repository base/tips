use alloy_primitives::{Address, B256, U256};
use anyhow::Result;
use async_trait::async_trait;
use reth_provider::{StateProvider as RethStateProvider, StateProviderFactory};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, warn};

/// Provides access to blockchain state for simulation
#[async_trait]
pub trait StateProvider: Send + Sync {
    /// Get the current block number
    async fn get_block_number(&self) -> Result<u64>;
    
    /// Get block hash for a given block number
    async fn get_block_hash(&self, block_number: u64) -> Result<B256>;
    
    /// Get account balance at a specific block
    async fn get_balance(&self, address: Address, block_number: u64) -> Result<U256>;
    
    /// Get account nonce at a specific block
    async fn get_nonce(&self, address: Address, block_number: u64) -> Result<u64>;
    
    /// Get storage value at a specific slot and block
    async fn get_storage(&self, address: Address, slot: U256, block_number: u64) -> Result<U256>;
    
    /// Get account code at a specific block
    async fn get_code(&self, address: Address, block_number: u64) -> Result<Vec<u8>>;
    
    /// Get multiple storage slots efficiently
    async fn get_storage_batch(
        &self,
        requests: Vec<(Address, Vec<U256>)>,
        block_number: u64,
    ) -> Result<HashMap<Address, HashMap<U256, U256>>>;
}

/// Direct reth state provider that accesses state without RPC
pub struct DirectStateProvider<SF> {
    state_provider_factory: Arc<SF>,
    /// Current block number for state queries
    current_block_number: u64,
}

impl<SF> DirectStateProvider<SF> 
where
    SF: StateProviderFactory,
{
    pub fn new(state_provider_factory: Arc<SF>, current_block_number: u64) -> Self {
        Self {
            state_provider_factory,
            current_block_number,
        }
    }

    /// Update the current block number for state queries
    pub fn set_current_block(&mut self, block_number: u64) {
        self.current_block_number = block_number;
    }

    /// Get a state provider for the current block
    fn get_state_provider(&self) -> Result<Box<dyn RethStateProvider>> {
        self.state_provider_factory
            .state_by_block_number(self.current_block_number)
            .map_err(|e| anyhow::anyhow!("Failed to get state provider: {}", e))
    }
}

#[async_trait]
impl<SF> StateProvider for DirectStateProvider<SF>
where
    SF: StateProviderFactory + Send + Sync,
{
    async fn get_block_number(&self) -> Result<u64> {
        Ok(self.current_block_number)
    }
    
    async fn get_block_hash(&self, block_number: u64) -> Result<B256> {
        let state_provider = self.get_state_provider()?;
        
        // Get block hash from state provider
        // Note: This would need to be implemented based on reth's state provider API
        // For now, we'll use a placeholder
        debug!(block_number = block_number, "Getting block hash from direct state");
        
        // TODO: Implement proper block hash retrieval from reth state provider
        Ok(B256::ZERO) // Placeholder
    }
    
    async fn get_balance(&self, address: Address, _block_number: u64) -> Result<U256> {
        let state_provider = self.get_state_provider()?;
        
        match state_provider.account_balance(address) {
            Ok(Some(balance)) => {
                debug!(
                    address = %address,
                    block_number = self.current_block_number,
                    balance = %balance,
                    "Retrieved balance from direct state"
                );
                Ok(balance)
            }
            Ok(None) => {
                debug!(
                    address = %address,
                    block_number = self.current_block_number,
                    "Account not found, returning zero balance"
                );
                Ok(U256::ZERO)
            }
            Err(e) => {
                error!(
                    error = %e,
                    address = %address,
                    block_number = self.current_block_number,
                    "Failed to get balance from direct state"
                );
                Err(anyhow::anyhow!("State provider error: {}", e))
            }
        }
    }
    
    async fn get_nonce(&self, address: Address, _block_number: u64) -> Result<u64> {
        let state_provider = self.get_state_provider()?;
        
        match state_provider.account_nonce(address) {
            Ok(Some(nonce)) => {
                debug!(
                    address = %address,
                    block_number = self.current_block_number,
                    nonce = nonce,
                    "Retrieved nonce from direct state"
                );
                Ok(nonce)
            }
            Ok(None) => {
                debug!(
                    address = %address,
                    block_number = self.current_block_number,
                    "Account not found, returning zero nonce"
                );
                Ok(0)
            }
            Err(e) => {
                error!(
                    error = %e,
                    address = %address,
                    block_number = self.current_block_number,
                    "Failed to get nonce from direct state"
                );
                Err(anyhow::anyhow!("State provider error: {}", e))
            }
        }
    }
    
    async fn get_storage(&self, address: Address, slot: U256, _block_number: u64) -> Result<U256> {
        let state_provider = self.get_state_provider()?;
        
        match state_provider.storage(address, reth_primitives::StorageKey::from(slot)) {
            Ok(Some(value)) => {
                debug!(
                    address = %address,
                    slot = %slot,
                    block_number = self.current_block_number,
                    value = %value,
                    "Retrieved storage from direct state"
                );
                Ok(U256::from(value))
            }
            Ok(None) => {
                debug!(
                    address = %address,
                    slot = %slot,
                    block_number = self.current_block_number,
                    "Storage slot not found, returning zero"
                );
                Ok(U256::ZERO)
            }
            Err(e) => {
                error!(
                    error = %e,
                    address = %address,
                    slot = %slot,
                    block_number = self.current_block_number,
                    "Failed to get storage from direct state"
                );
                Err(anyhow::anyhow!("State provider error: {}", e))
            }
        }
    }
    
    async fn get_code(&self, address: Address, _block_number: u64) -> Result<Vec<u8>> {
        let state_provider = self.get_state_provider()?;
        
        match state_provider.account_code(address) {
            Ok(Some(code)) => {
                debug!(
                    address = %address,
                    block_number = self.current_block_number,
                    code_len = code.len(),
                    "Retrieved code from direct state"
                );
                Ok(code.original_bytes())
            }
            Ok(None) => {
                debug!(
                    address = %address,
                    block_number = self.current_block_number,
                    "Account has no code"
                );
                Ok(vec![])
            }
            Err(e) => {
                error!(
                    error = %e,
                    address = %address,
                    block_number = self.current_block_number,
                    "Failed to get code from direct state"
                );
                Err(anyhow::anyhow!("State provider error: {}", e))
            }
        }
    }
    
    async fn get_storage_batch(
        &self,
        requests: Vec<(Address, Vec<U256>)>,
        block_number: u64,
    ) -> Result<HashMap<Address, HashMap<U256, U256>>> {
        let mut result = HashMap::new();
        
        // Process each address
        for (address, slots) in requests {
            let mut address_storage = HashMap::new();
            
            for slot in slots {
                match self.get_storage(address, slot, block_number).await {
                    Ok(value) => {
                        address_storage.insert(slot, value);
                    }
                    Err(e) => {
                        warn!(
                            error = %e,
                            address = %address,
                            slot = %slot,
                            "Failed to get storage in batch request"
                        );
                    }
                }
            }
            
            if !address_storage.is_empty() {
                result.insert(address, address_storage);
            }
        }
        
        Ok(result)
    }
}

/// Create a direct state provider using reth's state provider factory
pub fn create_direct_state_provider<SF>(
    state_provider_factory: Arc<SF>,
    current_block_number: u64,
) -> impl StateProvider
where
    SF: StateProviderFactory + Send + Sync + 'static,
{
    DirectStateProvider::new(state_provider_factory, current_block_number)
}
