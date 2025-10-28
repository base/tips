# TIPS E2E Tests

End-to-end tests for the TIPS (Transaction Inclusion Protocol Service) system.

## Prerequisites

- Docker Desktop (running)
- [just](https://github.com/casey/just) command runner: `brew install just`
- Rust toolchain

## Running All Tests

From the repository root:

```bash
# 1. Set up environment variables (first time only)
just sync-env

# 2. Start all TIPS services
just start-all

# 3. Run tests
cd crates/e2e-tests
cargo test 
```

## Stopping Services

```bash
just stop-all
```

## Test Structure

- `src/client/` - RPC client for interacting with TIPS services
- `src/fixtures/` - Test data generators (transactions, signers)
- `tests/` - End-to-end test scenarios

### Test Categories

**Basic Tests (No Server Required):**
- `test_rpc_client_instantiation` - Verifies client creation
- `test_send_raw_transaction_rejects_empty` - Tests empty transaction handling
- `test_send_raw_transaction_rejects_invalid` - Tests invalid transaction handling
- `test_send_bundle_rejects_empty` - Tests empty bundle handling

**Integration Tests (Require Running Server + Ethereum Node):**
- `test_send_valid_transaction` - Tests valid transaction submission
- `test_send_bundle_with_valid_transaction` - Tests bundle with single transaction
- `test_send_bundle_with_replacement_uuid` - Tests bundle replacement functionality
- `test_send_bundle_with_multiple_transactions` - Tests bundle with multiple transactions

**Note:** Integration tests require:
1. `RUN_NODE_TESTS=1` environment variable to run
2. TIPS services running (`just start-all`)
3. An Ethereum node running at port 2222 (configured via `TIPS_INGRESS_RPC_MEMPOOL`)

**If you set `RUN_NODE_TESTS=1` but the Ethereum node is not running**, the integration tests will **fail**. This is intentional - setting the environment variable asserts you have the full stack ready.

### Running Specific Tests

```bash
# Run only basic tests (no server needed)
cargo test --package tips-e2e-tests

# Run all tests including integration tests (requires: just start-all)
RUN_NODE_TESTS=1 cargo test --package tips-e2e-tests

# Run a specific integration test
RUN_NODE_TESTS=1 cargo test --package tips-e2e-tests test_send_bundle_with_valid_transaction
```

## Notes

- Tests expect services running on `localhost:8080` (ingress-rpc)
- Basic tests work without any services running (gracefully handle connection errors)
- Integration tests **require** `RUN_NODE_TESTS=1` environment variable to run
- If `RUN_NODE_TESTS=1` is set, integration tests will **fail** if the Ethereum node is not running
- The ingress-rpc service expects an Ethereum node at port 2222 (configurable via `TIPS_INGRESS_RPC_MEMPOOL`)
- To run integration tests successfully: Start Ethereum node → `just start-all` → `RUN_NODE_TESTS=1 cargo test`

