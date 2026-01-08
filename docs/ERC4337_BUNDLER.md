# TIPS ERC-4337 Bundler

## Overview

ERC-4337 bundlers include user operations (smart account transactions) onchain. TIPS includes a native bundler that provides optimal speed and cost for user operations with full audit tracing via the TIPS UI.

## Architecture

### Ingress

TIPS exposes `eth_sendUserOperation` and performs standard ERC-7562 validation checks while managing the user operation mempool. Supported entry points: v0.6 through v0.9.

Base node reth includes `base_validateUserOperation` for validating user ops before adding them to the queue.

### ERC-7562 Validation

[ERC-7562](https://eips.ethereum.org/EIPS/eip-7562) protects bundlers from DoS attacks through unpaid computation and reverting transactions. The rules restrict:

- Opcodes
- Reputation
- Storage access
- Bundle rules
- Staking for globally used contracts (paymasters, factories)

These restrictions minimize cross-transactional dependencies that could allow one transaction to invalidate another.

TIPS streams events from the block builder to the audit stream, updating ingress rules and reputation/mempool limits. A Redis cluster maintains reputation scores for user operation entities (sender, paymaster, factory) tracking `opsSeen` and `opsIncluded` over configured intervals.

User operations from `BANNED` entities are filtered out before validation.

### Block Building

Native bundler integration enables:

- **Larger bundles**: Reduces signatures from worst case 2N to best case N+1 (N = number of user ops), improving data availability on Base Chain
- **Priority fee ordering**: User operations ordered by priority fee within bundles

Initial approach: One large bundle at the middle of each flashblock with priority fee ordering within that bundle.

#### Bundle Construction

1. Incrementally stack user op validation phases
2. Attempt to include the transaction
3. Prune and resubmit any reverting ops
4. Execute once the bundle is built (no revert risk in execution phase)

#### Key Management

The block builder requires a hot bundler key that accrues ETH. Balance is swept periodically (every N blocks) to the sequencer address.

Future AA V2 phases will add a new transaction type to remove this hot key requirement.

## RPC Methods

### Base Node Methods

| Method | Description |
|--------|-------------|
| `base_validateUserOperation` | Validates user operation conforms to ERC-7562 with successful validation phase |
| `eth_supportedEntrypoints` | Returns supported entry points |
| `eth_estimateUserOperationGas` | Returns PVG and gas limit estimates |
| `eth_sendUserOperation` | Sends user operation to TIPS pipeline after validation |
| `eth_getUserOperationByHash` | Gets user operation by hash (flashblock enabled) |
| `eth_getUserOperationReceipt` | Gets user operation receipt (flashblock enabled) |

### Gas Estimation

`eth_estimateUserOperationGas` returns:
- Verification gas limit
- Execution gas limit
- PreVerificationGas (PVG)

PVG covers bundler tip, entrypoint overhead, and L1 data fee.

### PreVerificationGas (PVG)

Current issues:
1. Primary source of overcharging
2. User ops stuck in mempool if PVG too low

Solutions:
- Future AA V2 will decouple L1 data fee from bundler tip + overhead via `l1GasFeeLimit`
- TIPS native bundler amortizes costs across user ops in bundles, enabling lower PVG values
- Clear error messages for PVG too low conditions

## Call Flow

```
Wallet (EIP-5792)
    ↓
eth_sendUserOperation
    ↓
base_validateUserOperation (ERC-7562)
    ↓
User Operation Queue (Kafka)
    ↓
User Operation Mempool
    ↓
rBuilder (bundle construction)
    ↓
Block Inclusion
    ↓
Audit Pipeline
```
