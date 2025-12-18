# UserOp Integration - Local Development Guide

This guide explains how to run the complete TIPS UserOperation integration locally for development and testing.

## Quick Start

### 1. Setup Environment

```bash
# Sync environment files
just sync

# This creates:
# - .env (for local development)
# - .env.docker (for Docker services)
# - ui/.env (for UI)
```

### 2. Start Dependencies

```bash
# Start Kafka and MinIO only
docker-compose up -d kafka minio

# Wait for services to be healthy
sleep 10
```

### 3. Run Services Locally

Open 3 terminal windows:

**Terminal 1 - Ingress RPC:**
```bash
just ingress-rpc
```

**Terminal 2 - Builder:**
```bash
just builder
```

**Terminal 3 - Audit (Optional):**
```bash
just audit
```

### 4. Test UserOp Submission

```bash
# Send a test UserOperation
just send-userop
```

You should see:
- Ingress RPC logs: "User operation queued"
- Kafka topic receives the UserOp
- Builder logs: "Received user operation" and "Flushing user operations bundle"

## Alternative: Run Everything in Docker

```bash
# Start all services in Docker
just start-all
```

## Development Workflows

### Developing the Builder

Run all services except builder in Docker, then run builder locally:

```bash
# Start everything except builder
just start-except builder

# In another terminal, run builder locally
just builder
```

### Developing Ingress RPC

```bash
# Start everything except ingress-rpc
just start-except ingress-rpc

# In another terminal, run ingress-rpc locally
just ingress-rpc
```

### Developing Both Builder and Ingress RPC

```bash
# Start only dependencies
just start-except builder ingress-rpc

# Terminal 1: Run ingress-rpc locally
just ingress-rpc

# Terminal 2: Run builder locally
just builder
```

## Testing

### Unit Tests

```bash
# Run all builder unit tests (13 tests)
cargo test -p tips-builder --bin tips-builder
```

### Integration Tests

```bash
# Run builder integration tests (requires Kafka)
just test-integration
```

This runs 4 integration tests:
- `test_userop_kafka_flow` - End-to-end Kafka publish/consume
- `test_userop_batching` - Batching of multiple UserOps
- `test_userop_hash_consistency` - Hash determinism
- `test_userop_serialization` - JSON serialization

### End-to-End Test

```bash
# Requires ingress-rpc and Kafka running
just test-userop-e2e
```

Or use the script directly:
```bash
./scripts/test-userop-integration.sh
```

## Monitoring

### Check Kafka Topics

```bash
# List all topics
docker exec tips-kafka kafka-topics --list --bootstrap-server localhost:29092

# Check UserOp topic
docker exec tips-kafka kafka-console-consumer \
  --bootstrap-server localhost:29092 \
  --topic tips-user-operation \
  --from-beginning \
  --max-messages 5
```

### Check Consumer Group Lag

```bash
docker exec tips-kafka kafka-consumer-groups \
  --bootstrap-server localhost:29092 \
  --describe \
  --group tips-builder
```

### Builder Logs

When running locally, you'll see structured logs:

```
INFO user_op_received user_op_hash=0x... entry_point=0x...
INFO userop_batch_flushed entry_point=0x... count=3
INFO bundle_created bundle_hash=0x... user_op_count=3
INFO bundler_tx_inserted position=3 total_txs=6
```

## Configuration

### Local Development (.env)

```bash
# Kafka (local)
TIPS_BUILDER_KAFKA_BROKERS=localhost:9092
TIPS_BUILDER_KAFKA_PROPERTIES_FILE=./docker/builder-kafka-properties
TIPS_BUILDER_KAFKA_TOPIC=tips-user-operation
TIPS_BUILDER_KAFKA_GROUP_ID=tips-builder

# Batching
TIPS_BUILDER_USEROP_BATCH_SIZE=100
TIPS_BUILDER_USEROP_BATCH_TIMEOUT_MS=1000

# Bundler
TIPS_BUILDER_BUNDLER_ADDRESS=0x1111111111111111111111111111111111111111
TIPS_BUILDER_ENTRY_POINT=0x0000000071727De22E5E9d8BAf0edAc6f37da032
```

### Docker (.env.docker)

The `just sync-env` command automatically converts localhost to host.docker.internal:

```bash
# Kafka (Docker)
TIPS_BUILDER_KAFKA_BROKERS=host.docker.internal:9094
TIPS_BUILDER_KAFKA_PROPERTIES_FILE=/app/docker/builder-kafka-properties
```

## Troubleshooting

### Kafka Connection Issues

**Problem:** Builder can't connect to Kafka

**Solution:**
```bash
# Check Kafka is running
docker ps | grep kafka

# Check Kafka health
docker exec tips-kafka kafka-broker-api-versions --bootstrap-server localhost:9092

# Restart Kafka
docker-compose restart kafka
```

### UserOps Not Appearing in Blocks

**Check Kafka Consumer:**
```bash
docker exec tips-kafka kafka-consumer-groups \
  --bootstrap-server localhost:29092 \
  --describe \
  --group tips-builder
```

**Check Builder Logs:**
```bash
# If running in Docker
docker logs tips-builder -f

# Look for errors
grep ERROR builder.log
```

**Verify UserOps in Kafka:**
```bash
docker exec tips-kafka kafka-console-consumer \
  --bootstrap-server localhost:29092 \
  --topic tips-user-operation \
  --from-beginning
```

### Integration Tests Failing

**Ensure Kafka is running:**
```bash
docker-compose up -d kafka kafka-setup
sleep 5
```

**Clean Kafka data:**
```bash
just stop-all
just start-except builder ingress-rpc
```

## Advanced Usage

### Custom UserOp Submission

```bash
curl -X POST http://localhost:8080 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "eth_sendUserOperation",
    "params": [{
      "sender": "0x3333333333333333333333333333333333333333",
      "nonce": "0x0",
      "callData": "0x",
      "callGasLimit": "0x186a0",
      "verificationGasLimit": "0x7a120",
      "preVerificationGas": "0x5208",
      "maxFeePerGas": "0x77359400",
      "maxPriorityFeePerGas": "0x3b9aca00",
      "signature": "0x",
      "factory": null,
      "factoryData": null,
      "paymaster": null,
      "paymasterVerificationGasLimit": null,
      "paymasterPostOpGasLimit": null,
      "paymasterData": null
    }],
    "id": 1
  }'
```

### Adjust Batch Settings

Edit `.env`:
```bash
# Smaller batches, faster bundling
TIPS_BUILDER_USEROP_BATCH_SIZE=10
TIPS_BUILDER_USEROP_BATCH_TIMEOUT_MS=500

# Larger batches, more efficient
TIPS_BUILDER_USEROP_BATCH_SIZE=200
TIPS_BUILDER_USEROP_BATCH_TIMEOUT_MS=2000
```

Restart builder for changes to take effect.

### Monitor Kafka in Real-time

```bash
# Terminal 1: Watch incoming UserOps
docker exec tips-kafka kafka-console-consumer \
  --bootstrap-server localhost:29092 \
  --topic tips-user-operation

# Terminal 2: Send UserOps
just send-userop

# Terminal 3: Watch builder logs
just builder
```

## Architecture Overview

```
┌─────────────┐
│   Client    │
└──────┬──────┘
       │ eth_sendUserOperation
       ▼
┌─────────────────────────────┐
│      Ingress RPC            │
│  (localhost:8080)           │
│  • Validates UserOp         │
│  • Publishes to Kafka       │
└──────────┬──────────────────┘
           │
           ▼
┌─────────────────────────────┐
│     Kafka Broker            │
│  (localhost:9092)           │
│  Topic: tips-user-operation │
└──────────┬──────────────────┘
           │
           ▼
┌─────────────────────────────┐
│   Builder Kafka Consumer    │
│  (batches by EntryPoint)    │
└──────────┬──────────────────┘
           │
           ▼
┌─────────────────────────────┐
│   UserOpBundle Creation     │
│  (handleOps calldata)       │
└──────────┬──────────────────┘
           │
           ▼
┌─────────────────────────────┐
│   Block Building Pipeline   │
│  (midpoint insertion)       │
└──────────┬──────────────────┘
           │
           ▼
┌─────────────────────────────┐
│      Block Output           │
│  [TX | TX | BUNDLE | TX | TX]│
└─────────────────────────────┘
```

## Next Steps

- Run the full integration: `just test-userop-e2e`
- Monitor metrics at http://localhost:9002/metrics
- View UI at http://localhost:3000
- Explore block structure with cast/foundry tools

For more details, see:
- [USEROP_INTEGRATION.md](./USEROP_INTEGRATION.md) - Complete architecture
- [crates/builder/README.md](./crates/builder/README.md) - Builder documentation
- [scripts/test-userop-integration.sh](./scripts/test-userop-integration.sh) - Test script
