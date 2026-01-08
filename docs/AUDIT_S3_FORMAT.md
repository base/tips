# Audit S3 Storage Format

The audit system archives bundle and UserOp lifecycle events to S3 for long-term storage and lookup.

## Storage Paths

| Path | Description |
|------|-------------|
| `/bundles/<uuid>` | Bundle lifecycle history |
| `/transactions/by_hash/<hash>` | Transaction hash to bundle mapping |
| `/userops/<hash>` | UserOperation lifecycle history |

## Bundle History

**Path:** `/bundles/<uuid>`

Stores the complete lifecycle of a bundle as a series of events.

```json
{
  "history": [
    {
      "event": "Created",
      "timestamp": 1234567890,
      "key": "<bundle_id>-<uuid>",
      "data": {
        "bundle": { /* EthSendBundle object */ }
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

See [Bundle States](./BUNDLE_STATES.md) for event type definitions.

## Transaction Lookup

**Path:** `/transactions/by_hash/<hash>`

Maps transaction hashes to bundle UUIDs for efficient lookups.

```json
{
  "bundle_ids": [
    "550e8400-e29b-41d4-a716-446655440000",
    "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
  ]
}
```

## UserOperation History

**Path:** `/userops/<hash>`

Stores ERC-4337 UserOperation lifecycle events. Events are written after validation passes.

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

### UserOp Events

| Event | Description | Key Fields |
|-------|-------------|------------|
| `AddedToMempool` | UserOp passed validation and entered mempool | sender, entry_point, nonce |
| `Included` | UserOp included in a block | block_number, tx_hash |
| `Dropped` | UserOp removed from mempool | reason |

### Drop Reasons

| Reason | Description |
|--------|-------------|
| `Invalid(String)` | Validation failed with error message |
| `Expired` | TTL exceeded |
| `ReplacedByHigherFee` | Replaced by another UserOp with higher fee |
