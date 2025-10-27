# Bundle States

- **Created**:
  - Initial bundle creation with transaction list
  - Arguments: (bundle)
- **Updated**:
  - Bundle modification (transaction additions/removals)
  - Arguments: (bundle)
- **Cancelled**:
  - Bundle explicitly cancelled
  - Arguments: (nonce | uuid)
- **IncludedByBuilder**:
  - Bundle included by builder in flashblock
  - Arguments: (flashblockNum, blockNum, builderId)
- **IncludedInBlock**:
  - Final confirmation in blockchain
  - Arguments: (blockNum, blockHash)
- **Dropped**:
  - Bundle dropped from processing
  - Arguments: Why(enum Reason)
    - "TIMEOUT": Bundle expired without inclusion
    - "INCLUDED_BY_OTHER": Another bundle caused the transactions in this bundle to not be includable
    - "REVERTED": A transaction reverted which was not allowed to

### Dropping Transactions
Transactions can be dropped because of multiple reasons, all of which are indicated on 
the audit log for a transaction. The initial prototype has the following limits:

- Included by other
  - There are two bundles that overlap (e.g. bundleA=(txA, txB) and bundleB=(txA), if bundleB is included and txA in
    bundleA is not allowed to be dropped, then bundleA will be marked as "Included By Other" and dropped.
- Bundle Limits
  - Timeouts (block or flashblock)
  - Block number 
- Account Limits
  - An account can only have a fixed number (TBD) of transactions in the mempool, 
    transactions will be dropped by descending nonce
- Global Limits
  - When the mempool reaches a certain size (TBD), it will be pruned based on a combination of:
    - Bundle age
    - Low base fee