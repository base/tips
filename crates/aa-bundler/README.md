# AA Bundler Service

## Overview

The AA Bundler service is a bridge between UserOperations and the TIPS bundling pipeline. It:

1. **Consumes** UserOperations from `tips-user-operations` Kafka topic
2. **Validates** entry points and UserOperation format
3. **Converts** UserOperations to EntryPoint `handleOps()` transactions
4. **Creates** bundles from those transactions
5. **Publishes** to `tips-ingress` Kafka topic for processing

## Architecture

```
Kafka: tips-user-operations
  ↓
AA Bundler Service
  ├─ Consumer (receives UserOps)
  ├─ Validator (checks entry points)
  ├─ Converter (UserOp → EntryPoint tx)
  └─ Publisher (creates & publishes bundles)
  ↓
Kafka: tips-ingress
  ↓
Bundle Pool → Builder
```

## Configuration

### Environment Variables

```bash
# Kafka Consumer (UserOperations)
TIPS_AA_BUNDLER_KAFKA_CONSUMER_PROPERTIES_FILE=/app/docker/aa-bundler-consumer-kafka-properties
TIPS_AA_BUNDLER_KAFKA_CONSUMER_TOPIC=tips-user-operations

# Kafka Producer (Bundles)
TIPS_AA_BUNDLER_KAFKA_PRODUCER_PROPERTIES_FILE=/app/docker/aa-bundler-producer-kafka-properties
TIPS_AA_BUNDLER_KAFKA_PRODUCER_TOPIC=tips-ingress

# Kafka Audit
TIPS_AA_BUNDLER_KAFKA_AUDIT_PROPERTIES_FILE=/app/docker/audit-kafka-properties
TIPS_AA_BUNDLER_KAFKA_AUDIT_TOPIC=tips-audit

# Bundler Settings
TIPS_AA_BUNDLER_PRIVATE_KEY=0x...
TIPS_AA_BUNDLER_ENTRY_POINTS=0x0000000071727De22E5E9d8BAf0edAc6f37da032
TIPS_AA_BUNDLER_CHAIN_ID=8453

# RPC URLs
TIPS_AA_BUNDLER_RPC_URL=http://host.docker.internal:8545
TIPS_AA_BUNDLER_SIMULATION_RPC=http://host.docker.internal:8545

# Logging
TIPS_AA_BUNDLER_LOG_LEVEL=info
```

## Running

### With Docker

```bash
# Start all services including bundler
docker-compose -f docker-compose.yml -f docker-compose.tips.yml up -d

# View logs
docker logs -f tips-aa-bundler
```

### Locally

```bash
# Set environment variables
export TIPS_AA_BUNDLER_KAFKA_CONSUMER_PROPERTIES_FILE=docker/aa-bundler-consumer-kafka-properties
export TIPS_AA_BUNDLER_KAFKA_PRODUCER_PROPERTIES_FILE=docker/aa-bundler-producer-kafka-properties
export TIPS_AA_BUNDLER_KAFKA_AUDIT_PROPERTIES_FILE=docker/audit-kafka-properties
export TIPS_AA_BUNDLER_PRIVATE_KEY=0x...
export TIPS_AA_BUNDLER_ENTRY_POINTS=0x0000000071727De22E5E9d8BAf0edAc6f37da032
export TIPS_AA_BUNDLER_RPC_URL=http://localhost:8545
export TIPS_AA_BUNDLER_SIMULATION_RPC=http://localhost:8545

# Run
cargo run --bin tips-aa-bundler
```

## Implementation Status

### ✅ Completed
- Kafka consumer for UserOperations
- UserOperation message parsing
- Entry point validation
- Converter module structure
- Bundle publisher structure
- Audit logging integration

### 🚧 TODO
- **UserOperation to Transaction Conversion** (main remaining work)
  - ABI encoding for v0.6 UserOperations
  - ABI encoding for v0.7 UserOperations (PackedUserOperation)
  - EntryPoint.handleOps() calldata construction
  - Transaction creation and signing
  - Gas estimation
- Bundle creation and publishing
- Error handling and retry logic
- Metrics and monitoring

## Message Flow

### Input (from `tips-user-operations`)

```json
{
  "user_operation": {
    "sender": "0x...",
    "nonce": "0x0",
    // ... other fields
  },
  "entry_point": "0x0000000071727De22E5E9d8BAf0edAc6f37da032",
  "hash": "0x..."
}
```

### Output (to `tips-ingress`)

```json
{
  "uuid": "...",
  "txs": ["0x..."],  // EntryPoint.handleOps() transaction
  "block_number": 0,
  "reverting_tx_hashes": [],
  "meter_bundle_response": {...}
}
```

## Testing

```bash
# Monitor UserOperations topic
docker exec tips-kafka kafka-console-consumer \
  --bootstrap-server kafka:29092 \
  --topic tips-user-operations \
  --from-beginning

# Monitor bundles topic
docker exec tips-kafka kafka-console-consumer \
  --bootstrap-server kafka:29092 \
  --topic tips-ingress \
  --from-beginning

# View bundler logs
docker logs -f tips-aa-bundler
```

## Next Steps

The main work remaining is implementing the `convert_to_transaction()` method in `converter.rs`:

1. Add Alloy sol! types for EntryPoint contract
2. Implement ABI encoding for UserOperation structs
3. Build `handleOps([userOp], beneficiary)` calldata
4. Create, sign, and encode transaction
5. Add bundle creation and publishing logic

This keeps the bundler as a standalone, focused service that bridges UserOperations to the existing TIPS pipeline.

