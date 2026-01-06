# Bundle States

Bundles transition through the following states during their lifecycle.

## State Definitions

| State | Description | Arguments |
|-------|-------------|-----------|
| **Created** | Bundle created with initial transaction list | bundle |
| **Updated** | Bundle modified (transactions added/removed) | bundle |
| **Cancelled** | Bundle explicitly cancelled | nonce \| uuid |
| **IncludedByBuilder** | Bundle included in flashblock by builder | flashblockNum, blockNum, builderId |
| **IncludedInBlock** | Bundle confirmed in blockchain | blockNum, blockHash |
| **Dropped** | Bundle dropped from processing | reason |

## Drop Reasons

| Reason | Description |
|--------|-------------|
| `TIMEOUT` | Bundle expired without inclusion |
| `INCLUDED_BY_OTHER` | Overlapping bundle caused this bundle's transactions to become non-includable |
| `REVERTED` | A non-revertible transaction reverted |

## Mempool Limits

Bundles may be dropped when limits are exceeded:

### Bundle Limits
- Timeout (block or flashblock deadline)
- Target block number passed

### Account Limits
- Fixed number of transactions per account in mempool
- Excess transactions dropped by descending nonce

### Global Limits
- Mempool pruned at capacity based on:
  - Bundle age
  - Low base fee

### Overlapping Bundles

When bundles share transactions, inclusion of one may invalidate another:

```
bundleA = (txA, txB)
bundleB = (txA)

If bundleB is included and txA in bundleA cannot be dropped,
bundleA is marked INCLUDED_BY_OTHER and removed.
```
