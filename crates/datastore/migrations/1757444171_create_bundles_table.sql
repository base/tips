-- Create bundles table
CREATE TABLE IF NOT EXISTS bundles (
    id UUID PRIMARY KEY,
    txs TEXT[] NOT NULL,
    block_number BIGINT NOT NULL,
    min_timestamp BIGINT,
    max_timestamp BIGINT,
    reverting_tx_hashes TEXT[],
    replacement_uuid TEXT,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);