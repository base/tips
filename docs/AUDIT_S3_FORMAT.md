# S3 Storage Format

This document describes the S3 storage format used by the audit system for archiving bundle lifecycle events and transaction lookups.

## Storage Structure

### Bundle History: `/bundles/<UUID>`

Each bundle is stored as a JSON object containing its complete lifecycle history:

```json
{
   "history": [
        {
            "event": "Created",
            "timestamp": 1234567890,
            "bundle": {
                // EthSendBundle object
            }
        },
        {
            "event": "Updated",
            "timestamp": 1234567891,
            "bundle": {
                // EthSendBundle object
            }
        },
        {
            "event": "Cancelled",
            "timestamp": 1234567892
        },
        {
            "event": "BuilderIncluded",
            "builder": "builder-id",
            "timestamp": 1234567893,
            "blockNumber": 12345,
            "flashblockIndex": 1
        },
        {
            "event": "FlashblockIncluded",
            "timestamp": 1234567894,
            "blockNumber": 12345,
            "flashblockIndex": 1
        },
        {
            "event": "BlockIncluded",
            "timestamp": 1234567895,
            "blockNumber": 12345,
            "blockHash": "0x..."
        },
        {
            "event": "Dropped",
            "timestamp": 1234567896,
            "reason": "TIMEOUT"
        }
    ]
}
```

### Transaction Lookup: `/transactions/by_hash/<hash>`

Transaction hash to bundle mapping for efficient lookups:

```json
{
    "bundle_ids": [
        "550e8400-e29b-41d4-a716-446655440000",
        "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
    ]
}
```

## Event Types

### Bundle Events

- **Created**: Initial bundle creation with transaction list
- **Updated**: Bundle modification (transaction additions/removals)
- **Cancelled**: Bundle explicitly cancelled
- **BuilderIncluded**: Bundle included by builder in flashblock
- **FlashblockIncluded**: Flashblock containing bundle included in chain
- **BlockIncluded**: Final confirmation in blockchain
- **Dropped**: Bundle dropped from processing

### Drop Reasons

- `TIMEOUT`: Bundle expired without inclusion