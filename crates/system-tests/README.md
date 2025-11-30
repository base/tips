# System Tests (Integration Suite)

Integration coverage for TIPS ingress RPC. Tests talk to the real services started by `just start-all`.

## What we test
- `test_client_can_connect_to_tips` – RPC connectivity.
- `test_send_raw_transaction_accepted` – `eth_sendRawTransaction` returns a tx hash.
- `test_send_bundle_accepted` – single‑tx bundle returns the correct bundle hash, appears in Kafka/audit.
- `test_send_bundle_with_three_transactions` – max-sized bundle (3 txs) flows through Kafka/audit.
- `test_cancel_bundle_endpoint` – `eth_cancelBundle` RPC (currently ignored until server supports it).

Each bundle test confirms:
1. The response hash equals `keccak256` of the tx hashes.
2. The bundle is published to the ingress Kafka topic.
3. Audit propagation works end-to-end: Kafka `BundleEvent` and a persisted S3 bundle history entry.

## How to run
```bash
# Start infrastructure (see ../../SETUP.md for full instructions)
#  - just sync && just start-all
#  - builder-playground + op-rbuilder

# Run the tests
INTEGRATION_TESTS=1 cargo test --package tips-system-tests --test integration_tests
```

Defaults:
- Kafka configs: `docker/host-*.properties` (override with the standard `TIPS_INGRESS_KAFKA_*` env vars if needed).
- S3 (MinIO): `http://localhost:7000`, bucket `tips`, credentials `minioadmin` (override with the `TIPS_AUDIT_S3_*` env vars).
- URLs: `http://localhost:8080` ingress, `http://localhost:8547` sequencer (override via `INGRESS_URL` / `SEQUENCER_URL`).
- Tx submission mode: inferred from `TIPS_TEST_TX_SUBMISSION_METHOD` (or `TIPS_INGRESS_TX_SUBMISSION_METHOD`). Set to `mempool`, `kafka`, or `mempool,kafka` so the raw‑tx test knows which behavior to verify.

