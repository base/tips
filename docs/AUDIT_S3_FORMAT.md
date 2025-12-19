# S3 Storage Format

This document describes the S3 storage format used by the audit system for archiving bundle and UserOp lifecycle events and transaction lookups.

## Storage Structure

### Bundle History: `/bundles/<UUID>`

Each bundle is stored as a JSON object containing its complete lifecycle history. The history events in this object are 
dervied from the events defined in the [bundle states](./BUNDLE_STATES.md).

```json
{
   "history": [
        {
            "event": "Created",          // Event type
            "timestamp": 1234567890,     // timestamp event was written to kafka
            "key": "<bundle_id>-<uuid>", // used to dedup events
            "data": {
              "bundle": {
                // EthSendBundle object
              }
            }
        },
        {
            "event": "BuilderIncluded",
            "timestamp": 1234567893,
            "key": "<bundle_id>-<uuid>",
            "data": {
              "blockNumber": 12345,
              "flashblockIndex": 1,
              "builderId": "builder-id"
            }
        },
        {
            "event": "BlockIncluded",
            "timestamp": 1234567895,
            "key": "<bundle_id>-<uuid>",
            "data": {
              "blockNumber": 12345,
              "blockHash": "0x..."
            }
        },
        {
            "event": "Dropped",
            "timestamp": 1234567896,
            "key": "<bundle_id>-<uuid>",
            "data": {
              "reason": "TIMEOUT"
            }
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

### UserOp History: `/userops/<hash>`

Each UserOperation (ERC-4337) is stored as a JSON object containing its complete lifecycle history. Events are written after validation passes.

```json
{
    "history": [
        {
            "event": "AddedToMempool",
            "data": {
                "key": "<user_op_hash>-<uuid>",
                "timestamp": 1234567890,
                "sender": "0x1234567890abcdef1234567890abcdef12345678",
                "entry_point": "0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789",
                "nonce": "0x1"
            }
        },
        {
            "event": "Included",
            "data": {
                "key": "<user_op_hash>-<tx_hash>",
                "timestamp": 1234567895,
                "block_number": 12345678,
                "tx_hash": "0xabcdef..."
            }
        },
        {
            "event": "Dropped",
            "data": {
                "key": "<user_op_hash>-<uuid>",
                "timestamp": 1234567896,
                "reason": {
                    "Invalid": "AA21 didn't pay prefund"
                }
            }
        }
    ]
}
```

#### UserOp Event Types

| Event | When | Key Fields |
|-------|------|------------|
| `AddedToMempool` | UserOp passes validation and enters the mempool | sender, entry_point, nonce |
| `Dropped` | UserOp removed from mempool | reason (Invalid, Expired, ReplacedByHigherFee) |
| `Included` | UserOp included in a block | block_number, tx_hash |

#### Drop Reasons

- `Invalid(String)` - Validation failed with error message (e.g., revert reason)
- `Expired` - TTL exceeded
- `ReplacedByHigherFee` - Replaced by another UserOp with higher fee
