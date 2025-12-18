# TIPS Builder - ERC-4337 UserOp Support

This builder integrates ERC-4337 UserOperations with rblib, following the enshrining pattern from [base-op-rbuilder#1](https://github.com/shamit05/base-op-rbuilder/pull/1).

## Overview

The TIPS Builder implements a complete end-to-end system for ERC-4337 UserOperation bundling:

1. **Ingress RPC** receives `eth_sendUserOperation` requests
2. **Kafka** queues UserOperations in the `tips-user-operation` topic
3. **Builder** consumes UserOps from Kafka and creates bundles
4. **Block Building** inserts bundler transactions at the midpoint of blocks
5. **EntryPoint** executes all UserOps atomically via `handleOps()`

## Running Tests

```bash
cargo test -p tips-builder
```

All tests verify the UserOperation bundling functionality, including midpoint insertion behavior.

## Configuration

### Environment Variables

Create a `.env` file based on `.env.builder.example`:

```bash
# Kafka Configuration
TIPS_BUILDER_KAFKA_BROKERS=localhost:9092
TIPS_BUILDER_KAFKA_PROPERTIES_FILE=./docker/builder-kafka-properties
TIPS_BUILDER_KAFKA_TOPIC=tips-user-operation
TIPS_BUILDER_KAFKA_GROUP_ID=tips-builder

# UserOp Batching
TIPS_BUILDER_USEROP_BATCH_SIZE=100
TIPS_BUILDER_USEROP_BATCH_TIMEOUT_MS=1000
```

### Kafka Properties

The builder uses `./docker/builder-kafka-properties` for Kafka configuration:
- Bootstrap servers
- Consumer group settings
- Auto-commit configuration
- Offset reset policy

## Running the Builder

### Local Development

```bash
# Start Kafka and dependencies
docker-compose up -d kafka minio

# Set environment variables
source .env

# Run the builder
cargo run -p tips-builder
```

### Docker

```bash
docker-compose up builder
```

## Integration Flow

### 1. Submit UserOperation via RPC

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

### 2. UserOp Flow

```
┌─────────────┐      ┌────────────┐      ┌─────────┐      ┌─────────┐
│ RPC Client  │─────▶│ Ingress-RPC│─────▶│  Kafka  │─────▶│ Builder │
└─────────────┘      └────────────┘      └─────────┘      └─────────┘
                            │                                    │
                            ▼                                    ▼
                     ┌─────────────┐                    ┌──────────────┐
                     │ Simulation  │                    │ OrderPool    │
                     │ (Validate)  │                    │ (Pipeline)   │
                     └─────────────┘                    └──────────────┘
                                                                │
                                                                ▼
                                                        ┌──────────────┐
                                                        │ Block (50%   │
                                                        │ regular txs) │
                                                        ├──────────────┤
                                                        │ ★ BUNDLER TX │
                                                        │ handleOps()  │
                                                        ├──────────────┤
                                                        │ Block (50%   │
                                                        │ regular txs) │
                                                        └──────────────┘
```

### 3. Block Structure

The builder creates blocks with UserOp bundles in the middle:

```
Block N:
├─ Transaction 1 (regular)
├─ Transaction 2 (regular)
├─ Transaction 3 (regular)
│
├─ BUNDLER TRANSACTION ★
│  └─ EntryPoint.handleOps([userOp1, userOp2, userOp3], beneficiary)
│     ├─ Execute userOp1
│     ├─ Execute userOp2
│     └─ Execute userOp3
│
├─ Transaction 4 (regular)
├─ Transaction 5 (regular)
└─ Transaction 6 (regular)
```

### 4. UserOp Batching

The Kafka consumer batches UserOperations before creating bundles:

- **Batch Size**: Configurable via `TIPS_BUILDER_USEROP_BATCH_SIZE` (default: 100)
- **Batch Timeout**: Configurable via `TIPS_BUILDER_USEROP_BATCH_TIMEOUT_MS` (default: 1000ms)
- **Grouping**: UserOps are grouped by EntryPoint address
- **Flushing**: Batches flush when size is reached OR timeout expires

## Monitoring

### Logs

The builder emits structured logs for:
- UserOp receipt from Kafka
- Bundle creation
- Bundler transaction insertion
- Block building events

### Metrics

Key metrics to monitor:
- `userop_batch_size`: Number of UserOps per bundle
- `userop_processing_time`: Time to process UserOps
- `bundler_tx_position`: Position of bundler tx in block (should be ~50%)
- `kafka_consumer_lag`: Consumer lag on `tips-user-operation` topic

## Architecture

### Components

**`userops.rs`** - `UserOperationOrder`
- Implements `OrderpoolOrder` trait for rblib compatibility
- Wraps `UserOperationRequest` from `account-abstraction-core`
- Supports nonce-based conflict resolution
- Works with both v0.6 and v0.7 UserOperations

**`bundle.rs`** - `UserOpBundle`
- Implements `Bundle<Optimism>` trait for rblib
- Creates EntryPoint `handleOps` calldata for bundling UserOps
- Supports bundler transaction positioning (Start/Middle/End)
- Handles both PackedUserOperation (v0.7) and UserOperation (v0.6)

### Key Feature: Bundler Transaction in Middle

The `UserOpBundle` generates a single transaction that calls `EntryPoint.handleOps(ops[], beneficiary)` with all UserOperations. This bundler transaction can be positioned:

- **Start**: Before all other transactions
- **Middle** (default): In the middle of other transactions in the block
- **End**: After all other transactions

## Usage

### Creating a UserOp Bundle

```rust
use tips_builder::{UserOpBundle, BundlerPosition};
use alloy_primitives::address;

let beneficiary = address!("0x2222...");
let entry_point = address!("0x0000000071727De22E5E9d8BAf0edAc6f37da032");

// Create bundle with UserOps
let bundle = UserOpBundle::new(beneficiary)
    .with_user_op(user_op_request_1)
    .with_user_op(user_op_request_2)
    .with_user_op(user_op_request_3)
    .with_position(BundlerPosition::Middle);

// Generate EntryPoint.handleOps calldata
let calldata = bundle.build_bundler_calldata();
```

### Transaction Ordering

When `BundlerPosition::Middle` is set and the bundle is applied in a block:

```
tx1 (regular transaction)
tx2 (regular transaction)
→ BUNDLER TX calling handleOps([userOp1, userOp2, userOp3], beneficiary)
tx3 (regular transaction)
tx4 (regular transaction)
```

The bundler transaction processes all UserOps atomically in a single transaction.

### Integration with rblib OrderPool

```rust
use tips_builder::UserOperationOrder;

// UserOps can be added to the OrderPool just like transactions
let user_op_order = UserOperationOrder::new(user_op_request)?;
pool.add_order(user_op_order);
```

## Implementation Details

### UserOperation to Bundle Transaction

The `build_bundler_calldata()` method:
1. Converts `UserOperationRequest` → `PackedUserOperation` (v0.7 format)
2. Packs gas limits into `accountGasLimits` (bytes32)
3. Packs fees into `gasFees` (bytes32)
4. Encodes as `handleOps(PackedUserOperation[], address)` calldata

### Bundle Trait Implementation

`UserOpBundle` implements rblib's `Bundle` trait:
- `transactions()`: Returns the bundler transaction if set
- `hash()`: Keccak256 of all UserOp hashes + bundler tx + beneficiary
- `without_transaction()`: Removes specific transactions
- `validate_post_execution()`: Post-execution validation hook

## Enshrining Pattern

Based on the pattern from op-rbuilder where the bundler transaction is "enshrined" into the block:

1. **Mempool** provides top UserOps sorted by gas price
2. **Bundler** creates a single transaction calling `EntryPoint.handleOps`
3. **Builder** positions this transaction in the middle of the block
4. **EntryPoint** atomically executes all UserOps in one transaction

This ensures:
- Gas-efficient bundling (one transaction for many UserOps)
- Atomic execution (all-or-nothing)
- MEV protection through positioning
- Beneficiary receives all bundle fees
