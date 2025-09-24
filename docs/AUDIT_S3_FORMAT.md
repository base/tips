# S3 Storage Format

This document describes the S3 storage format used by the audit system for archiving bundle lifecycle events and transaction lookups.

## Storage Structure

### Bundle History: `/bundles/<UUID>`

Each bundle is stored as a JSON object containing its complete lifecycle history. The history events in this object are 
dervied from the events defined in the [bundle states](./BUNDLE_STATES.md).

```json
{
   "history": [
        {
            "event": "Created",          // Event type (see ./BUNDLE_STATES.md)
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
            "event": "FlashblockIncluded",
            "timestamp": 1234567894,
            "key": "<bundle_id>-<uuid>",
            "data": {
              "blockNumber": 12345,
              "flashblockIndex": 1
            },
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
