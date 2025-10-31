# TIPS E2E Tests

End-to-end tests for the TIPS (Transaction Inclusion Protocol Service) system.

## Prerequisites

- Docker (running) - for Kafka
- Rust toolchain

**Note:** These tests use a mock provider and do not require an Optimism/Ethereum node to run

## Running All Tests

From the repository root:

```bash

# Run basic tests (no Kafka required)
cargo test --package tips-e2e-tests

# Run all tests including Kafka queue tests
KAFKA_QUEUE_TESTS=1 cargo test --package tips-e2e-tests
```

## Test Structure

- `src/client/` - RPC client for interacting with TIPS services
- `src/fixtures/` - Test data generators (transactions, signers)
- `tests/` - End-to-end test scenarios

### Test Categories

**Basic Tests (No External Dependencies):**
- `test_rpc_client_instantiation` - Verifies client creation
- `test_send_raw_transaction_rejects_empty` - Tests empty transaction handling
- `test_send_raw_transaction_rejects_invalid` - Tests invalid transaction handling
- `test_send_bundle_rejects_empty` - Tests empty bundle handling

**Kafka Queue Tests (Require KAFKA_QUEUE_TESTS=1 and Running Kafka):**
- `test_send_valid_transaction` - Tests valid transaction submission
- `test_send_bundle_with_valid_transaction` - Tests bundle with single transaction
- `test_send_bundle_with_replacement_uuid` - Tests bundle replacement functionality
- `test_send_bundle_with_multiple_transactions` - Tests bundle with multiple transactions


## Notes

- Tests start their own ingress-rpc server instance with a mock provider
- The mock provider returns large balances (100 ETH) and minimal L1 costs for all addresses
- Kafka queue provider is required as an external dependency for full e2e tests

## Architecture

The tests use a `MockProvider` that implements the validation traits (`AccountInfoLookup` and `L1BlockInfoLookup`) but returns static mock data instead of querying a real blockchain. This allows tests to run quickly without external node dependencies while still testing the core validation and RPC logic.

