# ERC-4337 UserOperation Integration

This document describes the complete integration of ERC-4337 UserOperations into the TIPS block builder system.

## System Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                         TIPS System                               │
│                                                                   │
│  ┌─────────────┐                                                 │
│  │   Client    │                                                 │
│  │  (Wallet)   │                                                 │
│  └──────┬──────┘                                                 │
│         │ eth_sendUserOperation                                  │
│         ▼                                                         │
│  ┌─────────────────────────────────────┐                        │
│  │       Ingress RPC Service           │                        │
│  │  (crates/ingress-rpc)               │                        │
│  │                                      │                        │
│  │  • Validates UserOp via simulation  │                        │
│  │  • Publishes to Kafka topic         │                        │
│  │  • Returns user_operation_hash      │                        │
│  └────────────┬────────────────────────┘                        │
│               │                                                   │
│               ▼                                                   │
│  ┌─────────────────────────────────────┐                        │
│  │          Kafka Broker               │                        │
│  │                                      │                        │
│  │  Topic: tips-user-operation         │                        │
│  │  • 3 partitions                     │                        │
│  │  • JSON-serialized UserOps          │                        │
│  │  • Key: user_op_hash (B256)         │                        │
│  └────────────┬────────────────────────┘                        │
│               │                                                   │
│               ▼                                                   │
│  ┌─────────────────────────────────────┐                        │
│  │      UserOp Kafka Consumer          │                        │
│  │  (crates/builder/kafka_consumer.rs) │                        │
│  │                                      │                        │
│  │  • Consumes from topic              │                        │
│  │  • Batches by EntryPoint            │                        │
│  │  • Creates UserOpBundles            │                        │
│  │  • Adds to InsertUserOpBundle       │                        │
│  └────────────┬────────────────────────┘                        │
│               │                                                   │
│               ▼                                                   │
│  ┌─────────────────────────────────────┐                        │
│  │    Block Building Pipeline          │                        │
│  │  (crates/builder/main.rs)           │                        │
│  │                                      │                        │
│  │  • InterleavedUserOpsStep           │                        │
│  │  • Collects regular transactions    │                        │
│  │  • Inserts bundler tx at midpoint   │                        │
│  │  • Creates handleOps() calldata     │                        │
│  └────────────┬────────────────────────┘                        │
│               │                                                   │
│               ▼                                                   │
│  ┌─────────────────────────────────────┐                        │
│  │           Block Output              │                        │
│  │                                      │                        │
│  │  TX[0-N/2]:  Regular transactions   │                        │
│  │  TX[N/2]:    Bundler transaction    │                        │
│  │              ↳ handleOps([...])     │                        │
│  │  TX[N/2+1]:  Regular transactions   │                        │
│  └─────────────────────────────────────┘                        │
└──────────────────────────────────────────────────────────────────┘
```

## Components

### 1. Ingress RPC (`crates/ingress-rpc`)

**Modified Files:**
- `src/service.rs` - Implemented `eth_sendUserOperation` endpoint

**Functionality:**
- Receives UserOperation requests via JSON-RPC
- Validates UserOps by calling simulation service
- Publishes validated UserOps to `tips-user-operation` Kafka topic
- Returns `user_operation_hash` to client

**Environment Variables:**
- `TIPS_INGRESS_VALIDATE_USER_OPERATION_TIMEOUT_MS` - Validation timeout (default: 2000ms)

### 2. Builder (`crates/builder`)

**New Files:**
- `src/kafka_consumer.rs` - Kafka consumer for UserOperations
- `src/bundle.rs` - UserOpBundle with EntryPoint integration
- `src/userops_pipeline.rs` - Pipeline step for UserOp insertion
- `src/userops.rs` - UserOperationOrder for orderpool

**Modified Files:**
- `src/main.rs` - Integrated Kafka consumer and pipeline

**Functionality:**

#### Kafka Consumer
- Subscribes to `tips-user-operation` topic
- Batches UserOps by EntryPoint address
- Configurable batch size and timeout
- Converts UserOps to `UserOpBundle`

#### UserOpBundle
- Implements `Bundle<Optimism>` trait from rblib
- Builds `EntryPoint.handleOps()` calldata for v0.7
- Creates signed EIP-1559 transactions
- Supports bundle hashing and validation

#### Pipeline Integration
- `InterleavedUserOpsStep` - Custom pipeline step
- `TransactionCollector` - Tracks transaction position
- Inserts bundler transaction at block midpoint
- Ensures one bundle per block

**Environment Variables:**
- `TIPS_BUILDER_KAFKA_BROKERS` - Kafka bootstrap servers (default: localhost:9092)
- `TIPS_BUILDER_KAFKA_PROPERTIES_FILE` - Kafka properties file
- `TIPS_BUILDER_KAFKA_TOPIC` - Topic name (default: tips-user-operation)
- `TIPS_BUILDER_KAFKA_GROUP_ID` - Consumer group (default: tips-builder)
- `TIPS_BUILDER_USEROP_BATCH_SIZE` - Batch size (default: 100)
- `TIPS_BUILDER_USEROP_BATCH_TIMEOUT_MS` - Batch timeout (default: 1000ms)

### 3. Configuration

**New Files:**
- `docker/builder-kafka-properties` - Kafka consumer configuration
- `.env.builder.example` - Example environment variables

**Docker Compose:**
- Kafka topic `tips-user-operation` already created with 3 partitions

## UserOperation Flow

### 1. Submission

```bash
curl -X POST http://localhost:8080 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "eth_sendUserOperation",
    "params": [{
      "sender": "0x...",
      "nonce": "0x0",
      "callData": "0x...",
      "callGasLimit": "0x186a0",
      "verificationGasLimit": "0x7a120",
      "preVerificationGas": "0x5208",
      "maxFeePerGas": "0x77359400",
      "maxPriorityFeePerGas": "0x3b9aca00",
      "signature": "0x..."
    }],
    "id": 1
  }'
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": {
    "user_operation_hash": "0x..."
  },
  "id": 1
}
```

### 2. Validation

The ingress RPC validates the UserOperation by calling:
```
base_validateUserOperation(user_operation) → simulation_provider
```

Returns validation result with:
- `expiration_timestamp` - When UserOp expires
- `gas_used` - Gas used in simulation

### 3. Kafka Queueing

Published message format:
```json
{
  "type": "PackedUserOperation",
  "sender": "0x...",
  "nonce": "0x0",
  "callData": "0x...",
  "callGasLimit": "0x186a0",
  "verificationGasLimit": "0x7a120",
  "preVerificationGas": "0x5208",
  "maxFeePerGas": "0x77359400",
  "maxPriorityFeePerGas": "0x3b9aca00",
  "signature": "0x...",
  ...
}
```

Key: `user_operation_hash` (B256)
Topic: `tips-user-operation`
Partition: Round-robin (0-2)

### 4. Consumption & Batching

Builder consumer:
1. Receives UserOps from Kafka
2. Groups by EntryPoint address
3. Accumulates until batch size or timeout
4. Creates `UserOpBundle` with all UserOps for each EntryPoint

### 5. Block Building

Pipeline execution:
1. `OptimismPrologue` - Initialize block context
2. `Loop` + `InterleavedUserOpsStep`:
   - Pull orders from OrderPool
   - Track transaction count
   - When midpoint reached:
     - Merge all pending UserOp bundles
     - Build `handleOps()` calldata
     - Create bundler transaction
     - Insert into block
   - Continue with remaining transactions

### 6. Block Structure

Final block:
```
Block 12345:
├─ 0: 0xabc... (regular transaction)
├─ 1: 0xdef... (regular transaction)
├─ 2: 0x123... (regular transaction)
│
├─ 3: 0x456... (BUNDLER TRANSACTION)
│     ↳ to: EntryPoint (0x0000000071727De22E5E9d8BAf0edAc6f37da032)
│     ↳ data: handleOps([userOp1, userOp2, userOp3], beneficiary)
│     ↳ from: bundler (0x111...111)
│
├─ 4: 0x789... (regular transaction)
├─ 5: 0xabc... (regular transaction)
└─ 6: 0xdef... (regular transaction)
```

## EntryPoint Integration

### V0.7 Format

UserOperations use the packed format:
- `accountGasLimits`: bytes32 (verificationGasLimit || callGasLimit)
- `gasFees`: bytes32 (maxPriorityFeePerGas || maxFeePerGas)
- `paymasterAndData`: bytes (paymaster || verificationGasLimit || postOpGasLimit || data)

### handleOps() Call

```solidity
interface IEntryPointV07 {
    function handleOps(
        PackedUserOperation[] calldata ops,
        address payable beneficiary
    ) external;
}
```

The bundler transaction calls `handleOps()` with:
- `ops`: Array of PackedUserOperation structs
- `beneficiary`: Address receiving bundler fees (set to bundler_address)

### Execution

EntryPoint executes each UserOp:
1. Validate UserOp signature
2. Pay bundler for pre-verification gas
3. Call account's validateUserOp()
4. Execute callData on account
5. Pay bundler for execution gas
6. Emit UserOperationEvent

## Testing

### Unit Tests

```bash
cargo test -p tips-builder
```

Tests include:
- UserOpBundle creation and serialization
- handleOps() calldata generation
- Bundle transaction creation
- Midpoint insertion logic
- Kafka consumer batching
- Nonce management

### Integration Test

```bash
./scripts/test-userop-integration.sh
```

Verifies:
1. Kafka is running
2. Ingress RPC is accessible
3. UserOp submission succeeds
4. UserOp appears in Kafka topic
5. Returns user_operation_hash

### Manual Testing

1. Start services:
```bash
docker-compose up -d kafka minio
cargo run -p ingress-rpc &
cargo run -p tips-builder &
```

2. Submit UserOp:
```bash
./scripts/test-userop-integration.sh
```

3. Check logs:
```bash
# Builder logs
grep "User operation" builder.log

# Kafka consumer lag
docker exec tips-kafka kafka-consumer-groups \
  --bootstrap-server localhost:29092 \
  --describe \
  --group tips-builder
```

## Monitoring

### Key Metrics

- `userop_received_total` - Total UserOps received
- `userop_batch_size` - Average batch size
- `userop_bundle_created_total` - Total bundles created
- `bundler_tx_position` - Position in block (should be ~50%)
- `kafka_consumer_lag` - Consumer lag on topic

### Logs

```bash
# UserOp reception
INFO user_op_received user_op_hash=0x... entry_point=0x...

# Batch flushing
INFO userop_batch_flushed entry_point=0x... count=3

# Bundle creation
INFO bundle_created bundle_hash=0x... user_op_count=3

# Bundler transaction insertion
INFO bundler_tx_inserted position=3 total_txs=6
```

### Kafka Monitoring

```bash
# List consumer groups
docker exec tips-kafka kafka-consumer-groups \
  --bootstrap-server localhost:29092 \
  --list

# Check consumer lag
docker exec tips-kafka kafka-consumer-groups \
  --bootstrap-server localhost:29092 \
  --describe \
  --group tips-builder

# View topic messages
docker exec tips-kafka kafka-console-consumer \
  --bootstrap-server localhost:29092 \
  --topic tips-user-operation \
  --from-beginning \
  --max-messages 10
```

## Production Considerations

### Configuration

1. **Batch Size**: Tune based on block gas limit
   - Too small: Many small bundles
   - Too large: Risk of exceeding gas limit

2. **Batch Timeout**: Balance latency vs throughput
   - Shorter: Lower latency, more frequent bundles
   - Longer: Better batching, higher latency

3. **Kafka Partitions**: Scale based on throughput
   - More partitions = more parallelism
   - Consider consumer group coordination

### Security

1. **Bundler Address**: Use dedicated EOA with funds
2. **Nonce Management**: Ensure sequential nonces
3. **Gas Price**: Monitor to avoid stuck transactions
4. **EntryPoint Validation**: Whitelist trusted EntryPoints

### Scalability

1. **Horizontal Scaling**: Multiple builder instances
   - Each instance joins same consumer group
   - Kafka auto-balances partitions

2. **Vertical Scaling**: Increase batch size
   - Handle more UserOps per bundle
   - Optimize gas usage

3. **Monitoring**: Track key metrics
   - Consumer lag
   - Bundle success rate
   - Gas efficiency

## Troubleshooting

### UserOps Not Appearing in Blocks

1. Check Kafka consumer is running:
```bash
docker exec tips-kafka kafka-consumer-groups \
  --bootstrap-server localhost:29092 \
  --describe \
  --group tips-builder
```

2. Check builder logs for errors:
```bash
grep ERROR builder.log
```

3. Verify UserOps in Kafka:
```bash
docker exec tips-kafka kafka-console-consumer \
  --bootstrap-server localhost:29092 \
  --topic tips-user-operation \
  --from-beginning
```

### Bundler Transaction Failing

1. Check bundler has ETH for gas
2. Verify EntryPoint address is correct
3. Check UserOp signatures are valid
4. Review EntryPoint revert reasons

### High Kafka Consumer Lag

1. Increase consumer instances
2. Increase batch size
3. Reduce batch timeout
4. Check for slow UserOp validation

## Future Enhancements

1. **Multi-EntryPoint Support**: Handle multiple EntryPoint versions
2. **Priority Ordering**: Order UserOps by gas price
3. **Bundle Optimization**: Pack UserOps efficiently
4. **MEV Protection**: Strategic bundle positioning
5. **Metrics Dashboard**: Real-time monitoring UI
6. **Audit Trail**: Track UserOp lifecycle events

## References

- [ERC-4337 Specification](https://eips.ethereum.org/EIPS/eip-4337)
- [EntryPoint v0.7](https://github.com/eth-infinitism/account-abstraction/blob/develop/contracts/core/EntryPoint.sol)
- [rblib Documentation](https://github.com/flashbots/rblib)
- [TIPS Architecture](./README.md)
