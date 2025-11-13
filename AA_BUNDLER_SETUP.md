# ✅ AA Bundler Service Complete

## What Was Created

A complete bundler service that bridges UserOperations from Kafka to the TIPS bundling pipeline.

### **New Crate: `crates/aa-bundler`**

**Files Created:**
- `Cargo.toml` - Service dependencies
- `src/lib.rs` - Module exports
- `src/types.rs` - UserOperation types and Kafka message format
- `src/config.rs` - Configuration and CLI args
- `src/consumer.rs` - Kafka consumer for UserOperations
- `src/converter.rs` - UserOp → EntryPoint transaction converter (stub)
- `src/bin/main.rs` - Service entry point
- `README.md` - Complete documentation

### **Kafka Configuration Files:**
- `docker/aa-bundler-consumer-kafka-properties` - Consumer config
- `docker/aa-bundler-producer-kafka-properties` - Producer config

### **Docker Integration:**
- Updated `Cargo.toml` - Added to workspace
- Updated `Dockerfile` - Build and copy binary
- Updated `docker-compose.tips.yml` - Added aa-bundler service

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                   User sends UserOperation                   │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│              Ingress RPC (eth_sendUserOperation)            │
│         Publishes to Kafka: tips-user-operations            │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│                    AA Bundler Service                        │
│  ┌───────────────────────────────────────────────────┐      │
│  │ 1. Consumer: Reads from tips-user-operations      │      │
│  └───────────────────────────────────────────────────┘      │
│  ┌───────────────────────────────────────────────────┐      │
│  │ 2. Validator: Checks supported entry points       │      │
│  └───────────────────────────────────────────────────┘      │
│  ┌───────────────────────────────────────────────────┐      │
│  │ 3. Converter: UserOp → EntryPoint.handleOps() tx  │      │
│  └───────────────────────────────────────────────────┘      │
│  ┌───────────────────────────────────────────────────┐      │
│  │ 4. Publisher: Creates bundle, publishes to Kafka  │      │
│  └───────────────────────────────────────────────────┘      │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│           Kafka Topic: tips-ingress (bundles)                │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│              Bundle Pool → Builder → L2 Block                │
└─────────────────────────────────────────────────────────────┘
```

## Configuration

### Required Environment Variables

```bash
# Consumer (reads UserOperations)
TIPS_AA_BUNDLER_KAFKA_CONSUMER_PROPERTIES_FILE=/app/docker/aa-bundler-consumer-kafka-properties
TIPS_AA_BUNDLER_KAFKA_CONSUMER_TOPIC=tips-user-operations

# Producer (writes bundles)
TIPS_AA_BUNDLER_KAFKA_PRODUCER_PROPERTIES_FILE=/app/docker/aa-bundler-producer-kafka-properties
TIPS_AA_BUNDLER_KAFKA_PRODUCER_TOPIC=tips-ingress

# Audit
TIPS_AA_BUNDLER_KAFKA_AUDIT_PROPERTIES_FILE=/app/docker/audit-kafka-properties
TIPS_AA_BUNDLER_KAFKA_AUDIT_TOPIC=tips-audit

# Bundler Configuration
TIPS_AA_BUNDLER_PRIVATE_KEY=0x1234567890abcdef...
TIPS_AA_BUNDLER_CHAIN_ID=8453

# RPC URLs
TIPS_AA_BUNDLER_RPC_URL=http://host.docker.internal:8545
TIPS_AA_BUNDLER_SIMULATION_RPC=http://host.docker.internal:8545

# Logging
TIPS_AA_BUNDLER_LOG_LEVEL=info
```

## Running the Service

### With Docker Compose

```bash
# Add environment variables to .env.docker
cat >> .env.docker <<EOF
TIPS_AA_BUNDLER_KAFKA_CONSUMER_PROPERTIES_FILE=/app/docker/aa-bundler-consumer-kafka-properties
TIPS_AA_BUNDLER_KAFKA_PRODUCER_PROPERTIES_FILE=/app/docker/aa-bundler-producer-kafka-properties
TIPS_AA_BUNDLER_KAFKA_AUDIT_PROPERTIES_FILE=/app/docker/audit-kafka-properties
TIPS_AA_BUNDLER_PRIVATE_KEY=0x0000000000000000000000000000000000000000000000000000000000000001
TIPS_AA_BUNDLER_CHAIN_ID=8453
TIPS_AA_BUNDLER_RPC_URL=http://host.docker.internal:8545
TIPS_AA_BUNDLER_SIMULATION_RPC=http://host.docker.internal:8545
EOF

# Start services
docker-compose down
docker-compose up -d
docker-compose -f docker-compose.yml -f docker-compose.tips.yml up -d

# View logs
docker logs -f tips-aa-bundler
```

### Locally

**Option 1: Using Just Command (Recommended)**

```bash
# Set environment variables (or use .env file)
export TIPS_AA_BUNDLER_KAFKA_CONSUMER_PROPERTIES_FILE=docker/aa-bundler-consumer-kafka-properties.local
export TIPS_AA_BUNDLER_KAFKA_PRODUCER_PROPERTIES_FILE=docker/aa-bundler-producer-kafka-properties.local
export TIPS_AA_BUNDLER_KAFKA_AUDIT_PROPERTIES_FILE=docker/audit-kafka-properties.local
export TIPS_AA_BUNDLER_PRIVATE_KEY=0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80
export TIPS_AA_BUNDLER_CHAIN_ID=13
export TIPS_AA_BUNDLER_RPC_URL=http://localhost:8545
export TIPS_AA_BUNDLER_SIMULATION_RPC=http://localhost:8545

# Run with Just
just aa-bundler
```

**Option 2: Using Cargo Directly**

```bash
# Set environment variables (same as above)
export TIPS_AA_BUNDLER_KAFKA_CONSUMER_PROPERTIES_FILE=docker/aa-bundler-consumer-kafka-properties.local
export TIPS_AA_BUNDLER_KAFKA_PRODUCER_PROPERTIES_FILE=docker/aa-bundler-producer-kafka-properties.local
export TIPS_AA_BUNDLER_KAFKA_AUDIT_PROPERTIES_FILE=docker/audit-kafka-properties.local
export TIPS_AA_BUNDLER_PRIVATE_KEY=0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80
export TIPS_AA_BUNDLER_CHAIN_ID=13
export TIPS_AA_BUNDLER_RPC_URL=http://localhost:8545
export TIPS_AA_BUNDLER_SIMULATION_RPC=http://localhost:8545

# Run with Cargo
cargo run --bin tips-aa-bundler
```

**Using .env File**

Create a `.env` file in the project root:

```bash
# Consumer Configuration
TIPS_AA_BUNDLER_KAFKA_CONSUMER_PROPERTIES_FILE=docker/aa-bundler-consumer-kafka-properties.local
TIPS_AA_BUNDLER_KAFKA_CONSUMER_TOPIC=tips-user-operations

# Producer Configuration
TIPS_AA_BUNDLER_KAFKA_PRODUCER_PROPERTIES_FILE=docker/aa-bundler-producer-kafka-properties.local
TIPS_AA_BUNDLER_KAFKA_PRODUCER_TOPIC=tips-ingress

# Audit Configuration
TIPS_AA_BUNDLER_KAFKA_AUDIT_PROPERTIES_FILE=docker/audit-kafka-properties.local
TIPS_AA_BUNDLER_KAFKA_AUDIT_TOPIC=tips-audit

# Bundler Configuration
TIPS_AA_BUNDLER_PRIVATE_KEY=0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80
TIPS_AA_BUNDLER_CHAIN_ID=13

# RPC URLs
TIPS_AA_BUNDLER_RPC_URL=http://localhost:8545
TIPS_AA_BUNDLER_SIMULATION_RPC=http://localhost:8545

# Logging
TIPS_AA_BUNDLER_LOG_LEVEL=info
```

Then run:

```bash
just aa-bundler
```

## Testing the Flow

### 1. Send a UserOperation

```bash
curl -X POST http://localhost:8080 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "eth_sendUserOperation",
    "params": [{
      "sender": "0x1234567890123456789012345678901234567890",
      "nonce": "0x0",
      "initCode": "0x",
      "callData": "0xabcd",
      "callGasLimit": "0x5208",
      "verificationGasLimit": "0x5208",
      "preVerificationGas": "0x5208",
      "maxFeePerGas": "0x3b9aca00",
      "maxPriorityFeePerGas": "0x3b9aca00",
      "paymasterAndData": "0x",
      "signature": "0x"
    }, "0x0000000071727De22E5E9d8BAf0edAc6f37da032"],
    "id": 1
  }'
```

### 2. Monitor Kafka Topics

```bash
# Watch UserOperations being published
docker exec tips-kafka kafka-console-consumer \
  --bootstrap-server kafka:29092 \
  --topic tips-user-operations \
  --from-beginning

# Watch bundles being created
docker exec tips-kafka kafka-console-consumer \
  --bootstrap-server kafka:29092 \
  --topic tips-ingress \
  --from-beginning
```

### 3. Check Logs

```bash
# Ingress RPC logs
docker logs -f tips-ingress-rpc

# Bundler logs
docker logs -f tips-aa-bundler

# Bundle pool logs (if running)
docker logs -f tips-bundle-pool
```

## Implementation Status

### ✅ Complete Infrastructure
- Kafka consumer setup
- Message parsing and validation
- Entry point validation
- Converter module structure
- Audit logging
- Docker integration
- Configuration management

### 🚧 TODO: Transaction Conversion

The main remaining work is in `crates/aa-bundler/src/converter.rs`:

```rust
pub fn convert_to_transaction(&self, user_op_message: &UserOperationMessage) -> Result<Bytes> {
    // TODO: Implement
    // 1. ABI encode UserOperation struct per EIP-4337
    // 2. Build EntryPoint.handleOps([userOp], beneficiary) calldata
    // 3. Create transaction with proper gas, nonce, etc.
    // 4. Sign with bundler key
    // 5. Encode as Bytes for bundle
}
```

This requires:
- Alloy sol! types for EntryPoint contract
- ABI encoding logic for UserOperation structs
- Transaction builder with gas estimation
- Bundler nonce management

### 🚧 TODO: Bundle Publishing

In `src/bin/main.rs`, enable bundle publishing:

```rust
// After successful conversion
let bundle = Bundle {
    txs: vec![entry_point_tx],
    block_number: 0,
    reverting_tx_hashes: vec![],
    ..Default::default()
};

// Meter and publish...
```

## Services Overview

| Service | Port | Purpose |
|---------|------|---------|
| tips-ingress-rpc | 8080 | Accepts UserOperations via RPC |
| tips-aa-bundler | N/A | Converts UserOps to bundles |
| tips-audit | N/A | Archives events to S3 |
| tips-bundle-pool | N/A | (Future) Manages bundle queue |
| kafka | 9092 | Message broker |

## Message Formats

### UserOperation Message (Kafka)

```json
{
  "user_operation": {
    "sender": "0x...",
    "nonce": "0x0",
    "initCode": "0x",
    "callData": "0x...",
    "callGasLimit": "0x5208",
    "verificationGasLimit": "0x5208",
    "preVerificationGas": "0x5208",
    "maxFeePerGas": "0x3b9aca00",
    "maxPriorityFeePerGas": "0x3b9aca00",
    "paymasterAndData": "0x",
    "signature": "0x..."
  },
  "entry_point": "0x0000000071727De22E5E9d8BAf0edAc6f37da032",
  "hash": "0x..."
}
```

### Bundle Message (to tips-ingress)

```json
{
  "uuid": "550e8400-e29b-41d4-a716-446655440000",
  "txs": ["0x..."],
  "block_number": 0,
  "reverting_tx_hashes": [],
  "meter_bundle_response": {...}
}
```

## Next Steps

1. **Implement Transaction Conversion**
   - Add EntryPoint contract ABI definitions
   - Implement UserOperation ABI encoding
   - Build and sign transactions

2. **Add Bundle Creation & Publishing**
   - Create Bundle from converted transaction
   - Call metering service
   - Publish to tips-ingress

3. **Add Monitoring & Metrics**
   - Track conversion success/failure rates
   - Monitor Kafka lag
   - Log bundle publishing

4. **Error Handling**
   - Retry logic for failed conversions
   - Dead letter queue for invalid UserOps
   - Alerting for entry point issues

The infrastructure is complete and ready for the conversion logic! 🚀

