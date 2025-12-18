# Builder Testing Guide

This document explains the test structure for the TIPS builder with UserOperation support.

## Test Structure

### 1. Unit Tests (`src/main.rs`)

**Location**: `crates/builder/src/main.rs` (in `#[cfg(test)]` module)

**Run**: `cargo test -p tips-builder --bin tips-builder`

**What they test**:
- Bundle creation and configuration
- UserOp addition to bundles
- `handleOps()` calldata generation
- Bundle transaction creation
- Transaction collector behavior
- Bundle hashing
- Nonce management
- **Midpoint insertion logic** ← This is the key test showing bundler tx goes in middle

**13 tests total** - All fast, no external dependencies

Key test for midpoint:
```rust
test_bundler_tx_inserted_at_midpoint
test_midpoint_detection
test_userops_bundle_only_inserted_once
```

### 2. Integration Tests (`tests/integration_tests.rs`)

**Location**: `crates/builder/tests/integration_tests.rs`

**Run**: `just test-integration` or `cargo test -p tips-builder --test integration_tests`

**What they test**:
- Kafka publish/consume flow
- UserOp serialization/deserialization
- Message key verification (hash as key)
- Batching behavior

**Requires**: Kafka running (`docker-compose up -d kafka`)

**4 tests total**:
- `test_userop_kafka_flow` - Basic Kafka round-trip
- `test_userop_batching` - Multiple UserOps batching
- `test_userop_hash_consistency` - Hash determinism
- `test_userop_serialization` - JSON round-trip

### 3. End-to-End Tests (`tests/userop_e2e_test.rs`)

**Location**: `crates/builder/tests/userop_e2e_test.rs`

**Run**: `just test-e2e`

**What they test** (THE FULL FLOW):

#### Test 1: `test_e2e_userop_to_block`
```
UserOp Creation → Kafka Publish → Kafka Consume → Bundle Creation → Calldata Generation → Midpoint Insertion
```

This test simulates the **complete end-to-end flow**:

1. **Step 1-2**: Creates 3 UserOps and publishes to Kafka (simulates ingress-rpc)
2. **Step 3-4**: Consumes UserOps from Kafka (simulates builder consumer)
3. **Step 5-6**: Creates `UserOpBundle` and generates `handleOps()` calldata
4. **Step 7-8**: Adds bundle to `InsertUserOpBundle` pipeline for midpoint insertion

**Output**:
```
========================================
END-TO-END TEST: UserOp → Kafka → Block
========================================

Step 1: Creating test UserOperations...
  ✓ Created UserOp with nonce=0
  ✓ Created UserOp with nonce=1
  ✓ Created UserOp with nonce=2

Step 2: Publishing UserOps to Kafka (simulating ingress-rpc)...
  ✓ Published UserOp 0 (hash: 0x...)
  ✓ Published UserOp 1 (hash: 0x...)
  ✓ Published UserOp 2 (hash: 0x...)

Step 3: Simulating builder Kafka consumer...
  ✓ Consumer subscribed to topic: tips-user-operation

Step 4: Consuming UserOps from Kafka...
  ✓ Consumed UserOp 1/3
  ✓ Consumed UserOp 2/3
  ✓ Consumed UserOp 3/3

Step 5: Creating UserOp bundle (simulating builder)...
  ✓ Added UserOp 0 to bundle
  ✓ Added UserOp 1 to bundle
  ✓ Added UserOp 2 to bundle

Step 6: Generating handleOps() calldata...
  ✓ Generated calldata: 1234 bytes
  ✓ Function selector: 0x1fad948c

Step 7: Verifying bundler transaction structure...
  ✓ EntryPoint: 0x0000000071727De22E5E9d8BAf0edAc6f37da032
  ✓ Beneficiary: 0x1111111111111111111111111111111111111111
  ✓ UserOp count: 3

Step 8: Simulating block building with midpoint insertion...
  ✓ Bundle added to pipeline
  ✓ Bundler transaction will be inserted at block midpoint

========================================
✓ END-TO-END TEST PASSED
========================================

Summary:
  • 3 UserOps published to Kafka ✓
  • 3 UserOps consumed from Kafka ✓
  • Bundle created with EntryPoint.handleOps() ✓
  • Calldata generated for bundler transaction ✓
  • Bundle ready for midpoint insertion ✓
```

#### Test 2: `test_e2e_multiple_batches`
Tests handling of multiple batches (10 UserOps batched into 2 groups of 5)

#### Test 3: `test_e2e_bundle_hash_verification`
Tests bundle hash determinism and uniqueness

**Requires**: Kafka running

**Note**: These tests are marked with `#[ignore]` so they don't run during normal `cargo test`. Use `just test-e2e` to run them explicitly.

## Test Commands

### Run All Tests (Fast)
```bash
# Unit tests only (no dependencies)
cargo test -p tips-builder --bin tips-builder
```

### Run Integration Tests (Needs Kafka)
```bash
# Start Kafka first
docker-compose up -d kafka

# Run integration tests
just test-integration
```

### Run E2E Tests (Full Flow)
```bash
# Start Kafka first
docker-compose up -d kafka

# Run E2E tests
just test-e2e
```

### Run Everything
```bash
# Start dependencies
docker-compose up -d kafka

# Run all tests
cargo test -p tips-builder --bin tips-builder
just test-integration
just test-e2e
```

## What Does E2E Test Prove?

The `test_e2e_userop_to_block` test proves the **complete UserOp integration**:

1. ✅ UserOps can be published to Kafka (ingress-rpc works)
2. ✅ Builder can consume UserOps from Kafka (consumer works)
3. ✅ UserOps are correctly deserialized (format works)
4. ✅ Bundles are created with multiple UserOps (bundling works)
5. ✅ `handleOps()` calldata is generated correctly (EntryPoint integration works)
6. ✅ Bundle is added to pipeline for midpoint insertion (block building works)

The unit test `test_bundler_tx_inserted_at_midpoint` proves:

7. ✅ Bundler transaction is inserted at ~50% position in block (midpoint logic works)

Together, these tests demonstrate the **entire flow from UserOp submission to block inclusion**.

## Live Testing

To test with actual services:

```bash
# Terminal 1: Start Kafka
docker-compose up -d kafka

# Terminal 2: Start ingress-rpc
just ingress-rpc

# Terminal 3: Start builder
just builder

# Terminal 4: Send UserOp
just send-userop
```

Watch the logs to see:
- Ingress RPC: "User operation queued"
- Kafka: UserOp message published
- Builder: "Received user operation", "Flushing user operations bundle"
- Builder: "Bundler tx inserted"

## Test Philosophy

- **Unit tests**: Fast, no dependencies, test individual components
- **Integration tests**: Medium speed, Kafka only, test Kafka integration
- **E2E tests**: Slower, full flow simulation, test the entire pipeline
- **Live testing**: Real services, manual verification

The E2E tests bridge the gap between unit tests and running actual services, giving high confidence that everything works together.
