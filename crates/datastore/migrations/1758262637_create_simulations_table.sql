-- Create simulations table
CREATE TABLE IF NOT EXISTS simulations (
    id UUID PRIMARY KEY,
    bundle_id UUID NOT NULL REFERENCES bundles(id) ON DELETE CASCADE,

    block_number BIGINT NOT NULL,
    block_hash CHAR(66) NOT NULL,
    execution_time_us BIGINT,
    gas_used BIGINT,
    
    -- Success tracking
    success BOOLEAN NOT NULL DEFAULT true,
    error_reason TEXT,
    
    -- State diff mapping accounts to storage slots to values
    -- Structure: { "account_address": { "slot": "value", ... }, ... }
    state_diff JSONB NOT NULL,
    
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    
    -- Unique constraint: one simulation per bundle per block hash
    UNIQUE(bundle_id, block_hash)
);

-- Index for efficient bundle lookups
CREATE INDEX IF NOT EXISTS idx_simulations_bundle_id ON simulations(bundle_id);

-- Index for block number queries
CREATE INDEX IF NOT EXISTS idx_simulations_block_number ON simulations(block_number);

-- Index for block hash queries
CREATE INDEX IF NOT EXISTS idx_simulations_block_hash ON simulations(block_hash);

-- Index for success field for efficient querying
CREATE INDEX IF NOT EXISTS idx_simulations_success ON simulations(success);
