# TIPS E2E Tests

End-to-end integration tests for the TIPS (Transaction Inclusion Protocol Service) system.

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

All 8 tests will run:
- `test_rpc_client_instantiation` - Verifies client creation
- `test_send_raw_transaction_rejects_empty` - Tests empty transaction rejection
- `test_send_raw_transaction_rejects_invalid` - Tests invalid transaction rejection
- `test_send_bundle_rejects_empty` - Tests empty bundle rejection
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
- `tests/` - End-to-end test scenarios

## Notes

- Tests will be skipped if `INTEGRATION_TESTS` environment variable is not set
- All tests require the full SETUP.md infrastructure to be running
- Tests use real nonces fetched from the L2 node, so they adapt to current blockchain state
- CI/CD setup will be added later to automate infrastructure provisioning

