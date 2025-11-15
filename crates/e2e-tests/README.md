# TIPS E2E Tests

End-to-end integration tests and load testing tools for the TIPS (Transaction Inclusion Protocol Service) system.

## Overview

This crate provides:
1. **Integration Tests** - Discrete test scenarios for TIPS functionality
2. **Load Testing Runner** - Multi-wallet concurrent load testing tool

## Prerequisites

All tests require the full infrastructure from `SETUP.md` running:
- TIPS ingress service (port 8080) via `just start-all`
- builder-playground (L1/L2 blockchain) on `danyal/base-overlay` branch
- op-rbuilder (block builder)
- Kafka
- MinIO

## Running Tests

### Start Infrastructure

Follow guidelines in `SETUP.md`

### Run Integration Tests

```bash
cd tips
INTEGRATION_TESTS=1 cargo test --package tips-e2e-tests -- --nocapture
```

All 5 tests will run:
- `test_rpc_client_instantiation` - Verifies client creation
- `test_send_valid_transaction` - End-to-end transaction submission
- `test_send_bundle_with_valid_transaction` - End-to-end single-transaction bundle
- `test_send_bundle_with_replacement_uuid` - Bundle replacement with UUID tracking
- `test_send_bundle_with_multiple_transactions` - Multi-transaction bundle

### Environment Variables

| Variable | Purpose | Default | Required |
|----------|---------|---------|----------|
| `INTEGRATION_TESTS` | Enable integration tests | (unset) | Yes |
| `INGRESS_URL` | TIPS ingress service URL | `http://localhost:8080` | No |
| `SEQUENCER_URL` | L2 sequencer node | `http://localhost:8547` | No |

## Test Structure

- `src/client/` - RPC client for interacting with TIPS services
- `src/fixtures/` - Test data generators (transactions, signers)
- `src/bin/runner/` - Load testing runner implementation
- `tests/` - End-to-end test scenarios

## Load Testing

For load testing and performance metrics, see [METRICS.md](./METRICS.md).

The `tips-e2e-runner` binary provides:
- Multi-wallet concurrent load generation
- Time-to-inclusion tracking
- Throughput and latency metrics
- Reproducible test scenarios

Quick start:
```bash
# Build
cargo build --release --bin tips-e2e-runner

# Setup wallets (optional: add --output wallets.json to save)
./target/release/tips-e2e-runner setup --master-key <KEY> --num-wallets 100 --output wallets.json

# Run load test
./target/release/tips-e2e-runner load --rate 100 --duration 5m
```

See [METRICS.md](./METRICS.md) for complete documentation.

---

## Integration Test Notes

- Tests will be skipped if `INTEGRATION_TESTS` environment variable is not set
- All tests require the full SETUP.md infrastructure to be running
- Tests use real nonces fetched from the L2 node, so they adapt to current blockchain state
- CI/CD setup will be added later to automate infrastructure provisioning

