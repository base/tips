# API Reference

TIPS processes all transactions as bundles. Transactions submitted via `eth_sendRawTransaction` are wrapped into a single-transaction bundle with sensible defaults.

## Bundle Identifiers

Bundles have two identifiers:
- **UUID**: Server-generated unique identifier assigned on submission
- **Bundle Hash**: `keccak(..bundle.txns)`, derived from the transaction set

## Bundle Lifecycle

### Creation

Bundles are deduplicated by bundle hash. When multiple bundles share the same hash, the latest submission defines the bundle fields:

```
bundleA = (txA)           → store: {bundleA}
bundleB = (txA, txB)      → store: {bundleA, bundleB}
bundleC = (txA, txB)      → store: {bundleA, bundleC}  # replaces bundleB
bundleD = (txC, txA)      → store: {bundleA, bundleC, bundleD}
```

### Updates

Bundles can be updated via:
- `eth_sendRawTransaction`: matches by (address, nonce)
- `eth_sendBundle`: matches by UUID

Updates are best-effort. If a bundle is already included in a flashblock before the update processes, the original bundle will be used.

### Cancellation

Cancel bundles with `eth_cancelBundle`. Cancellations are best-effort and may not take effect if the bundle is already included.

## RPC Methods

### eth_sendRawTransaction

```
eth_sendRawTransaction(bytes) → hash
```

Validates and wraps the transaction in a bundle. Replacement transactions (same address and nonce) replace the existing bundle.

**Limits:**
- 25 million gas per transaction

### eth_sendBundle

```
eth_sendBundle(EthSendBundle) → uuid
```

Submits a bundle directly. Without a replacement UUID, inserts a new bundle (merging with existing bundles sharing the same hash). With a UUID, updates the existing bundle if it still exists.

**Limits:**
- 25 million gas per bundle
- Maximum 3 transactions per bundle
- All transaction hashes must be in `reverting_tx_hashes` (revert protection not supported)
- `dropping_tx_hashes` must be empty
- Refunds not supported (`refund_percent`, `refund_recipient`, `refund_tx_hashes` must be unset/empty)
- `extra_fields` must be empty

**Reference:** [EthSendBundle](https://github.com/alloy-rs/alloy/blob/25019adf54272a3372d75c6c44a6185e4be9dfa2/crates/rpc-types-mev/src/eth_calls.rs#L252)

### eth_cancelBundle

```
eth_cancelBundle(EthCancelBundle)
```

Cancels a bundle by UUID. Best-effort; may not succeed if already included by the builder.

**Reference:** [EthCancelBundle](https://github.com/alloy-rs/alloy/blob/25019adf54272a3372d75c6c44a6185e4be9dfa2/crates/rpc-types-mev/src/eth_calls.rs#L216)
