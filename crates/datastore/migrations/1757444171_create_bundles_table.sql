DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'bundle_state') THEN
        CREATE TYPE bundle_state AS ENUM (
            'Ready',
            'IncludedInFlashblock',
            'IncludedInBlock'
        );
    END IF;
END$$;

-- Create bundles table
CREATE TABLE IF NOT EXISTS bundles (
    id UUID PRIMARY KEY,
    bundle_state bundle_state NOT NULL,
    state_changed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- todo: bundle_hash, key cannot insert the same bundle, on conflict return existing UUID
    -- todo: bundle_type (single, bundle)
    -- todo: single_key: (address, nonce)
    -- on bundle_type: single allow to upsert by (address, nonce)

    txn_hashes CHAR(66)[],
    senders CHAR(42)[],
    minimum_base_fee BIGINT, -- todo find a larger type

    txs TEXT[] NOT NULL,
    reverting_tx_hashes CHAR(66)[],
    dropping_tx_hashes CHAR(66)[],

    block_number BIGINT,
    min_timestamp BIGINT,
    max_timestamp BIGINT,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);

