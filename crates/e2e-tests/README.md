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
cargo test -- --include-ignored
```

## Stopping Services

```bash
just stop-all
```

## Test Structure

- `src/client/` - RPC client for interacting with TIPS services
- `src/fixtures/` - Test data generators (transactions, signers)
- `tests/` - End-to-end test scenarios

## Notes

- Tests expect services running on `localhost:8080` (ingress-rpc)
- Ignored tests require a fully configured local node running via `just start-all`

