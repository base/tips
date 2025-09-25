# API

## Overview

TIPS only processes bundles. Transactions sent via `eth_sendRawTransaction` are wrapped into a bundle (see: [EthSendBundle](https://github.com/alloy-rs/alloy/blob/25019adf54272a3372d75c6c44a6185e4be9dfa2/crates/rpc-types-mev/src/eth_calls.rs#L252))
with a single transaction and sensible defaults.

Bundles can be identified in two ways:
- Bundle UUID: Generated server-side on submission of a new bundle, globally unique
- Bundle Hash: `keccak(..bundle.txns)`, unique per set of transactions

### Bundle Creates
Any bundle that's created as part of TIPS, will be deduplicated by bundle hash in the TIPS bundle store with the latest bundle
defining the fields. For example:
```
Multiple Bundles inserted in the order of A, B, C, D

# Single bundle transction
bundleA     = (txA)
bundleStore = {bundleA}

# Bundle with overlapping transactions
bundleB     = (txA, txB)
bundleStore = {bundleA, bundleB}

# Bundle with identical bundle hash
bundleC     = (txA, txB)
bundleStore = {bundleA, bundleC}

# Bundle with overlapping transactions
bundleD     = (txC, txA)
bundleStore = {bundleA, bundleC, bundleD}
```

### Bundle Updates
There are two ways bundles can be updated, either via `eth_sendRawTransaction` (address, nonce) or `eth_sendBundle` (UUID), see below for more details.

Bundle updates are **best effort**. For example:
```
bundleA = createBundle(txA)
# In parrallel
includedByBuilder(bundleA)          # bundleA is included in the current Flashblock
updateBundle(bundleA, [txB, txC])   # bundleA is updated to bundleA`
# Bundle A will have been removed from the bundle store
```

### Bundle Cancellation:
Bundles can be cancelled via `eth_cancelBundle`. Similar to bundle updates, cancellations are processed as **best effort**.

## RPC Methods

### eth_sendRawTransaction(Bytes) -> Hash
Transactions provided to this endpoint, are validated and then wrapped in a bundle (with defaults) and added to the bundle store.
Previously submitted transactions can be replaced by submitting a new transaction from the same address and nonce. These will 
replace bundles with the same bundle hash submitted via this endpoint or `eth_sendBundle`.

**Limitations:**
- 25 million gas per transaction

### eth_sendBundle([EthSendBundle](https://github.com/alloy-rs/alloy/blob/25019adf54272a3372d75c6c44a6185e4be9dfa2/crates/rpc-types-mev/src/eth_calls.rs#L252)) -> UUID
If a replacement UUID is not provided, this will attempt to insert the bundle. If a bundle with the same bundle hash already exists
the bundle will be combined with the existing one.

If a UUID is provided, this endpoint will only attempt to update a bundle, if that bundle is no longer in the bundle store, the 
update will be dropped.

**Limitations**
- 25 million gas per bundle
- Can only provide three transactions at once
- Revert protection is not supported, all transaction hashes must be in `reverting_tx_hashes`
- Partial transaction dropping is not supported, `dropping_tx_hashes` must be empty
- Refunds are not initially supported
  - `refund_percent` must not be set
  - `refund_receipient` must not be set
  - `refund_tx_hashes` must be empty
- extra_fields must be empty

### eth_cancelBundle([EthCancelBundle](https://github.com/alloy-rs/alloy/blob/25019adf54272a3372d75c6c44a6185e4be9dfa2/crates/rpc-types-mev/src/eth_calls.rs#L216))
- Will cancel the bundle matching the UUID
- Cancellation is the best effort, if the builder has already included it is will go through