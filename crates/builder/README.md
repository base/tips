# TIPS Builder - ERC-4337 UserOp Support

This builder integrates ERC-4337 UserOperations with rblib, following the enshrining pattern from [base-op-rbuilder#1](https://github.com/shamit05/base-op-rbuilder/pull/1).

## Running Tests

```bash
cargo test -p tips-builder
```

All tests verify the UserOperation bundling functionality, including midpoint insertion behavior.

## Architecture

### Components

**`userops.rs`** - `UserOperationOrder`
- Implements `OrderpoolOrder` trait for rblib compatibility
- Wraps `UserOperationRequest` from `account-abstraction-core`
- Supports nonce-based conflict resolution
- Works with both v0.6 and v0.7 UserOperations

**`bundle.rs`** - `UserOpBundle`
- Implements `Bundle<Optimism>` trait for rblib
- Creates EntryPoint `handleOps` calldata for bundling UserOps
- Supports bundler transaction positioning (Start/Middle/End)
- Handles both PackedUserOperation (v0.7) and UserOperation (v0.6)

### Key Feature: Bundler Transaction in Middle

The `UserOpBundle` generates a single transaction that calls `EntryPoint.handleOps(ops[], beneficiary)` with all UserOperations. This bundler transaction can be positioned:

- **Start**: Before all other transactions
- **Middle** (default): In the middle of other transactions in the block
- **End**: After all other transactions

## Usage

### Creating a UserOp Bundle

```rust
use tips_builder::{UserOpBundle, BundlerPosition};
use alloy_primitives::address;

let beneficiary = address!("0x2222...");
let entry_point = address!("0x0000000071727De22E5E9d8BAf0edAc6f37da032");

// Create bundle with UserOps
let bundle = UserOpBundle::new(beneficiary)
    .with_user_op(user_op_request_1)
    .with_user_op(user_op_request_2)
    .with_user_op(user_op_request_3)
    .with_position(BundlerPosition::Middle);

// Generate EntryPoint.handleOps calldata
let calldata = bundle.build_bundler_calldata();
```

### Transaction Ordering

When `BundlerPosition::Middle` is set and the bundle is applied in a block:

```
tx1 (regular transaction)
tx2 (regular transaction)
→ BUNDLER TX calling handleOps([userOp1, userOp2, userOp3], beneficiary)
tx3 (regular transaction)
tx4 (regular transaction)
```

The bundler transaction processes all UserOps atomically in a single transaction.

### Integration with rblib OrderPool

```rust
use tips_builder::UserOperationOrder;

// UserOps can be added to the OrderPool just like transactions
let user_op_order = UserOperationOrder::new(user_op_request)?;
pool.add_order(user_op_order);
```

## Implementation Details

### UserOperation to Bundle Transaction

The `build_bundler_calldata()` method:
1. Converts `UserOperationRequest` → `PackedUserOperation` (v0.7 format)
2. Packs gas limits into `accountGasLimits` (bytes32)
3. Packs fees into `gasFees` (bytes32)
4. Encodes as `handleOps(PackedUserOperation[], address)` calldata

### Bundle Trait Implementation

`UserOpBundle` implements rblib's `Bundle` trait:
- `transactions()`: Returns the bundler transaction if set
- `hash()`: Keccak256 of all UserOp hashes + bundler tx + beneficiary
- `without_transaction()`: Removes specific transactions
- `validate_post_execution()`: Post-execution validation hook

## Enshrining Pattern

Based on the pattern from op-rbuilder where the bundler transaction is "enshrined" into the block:

1. **Mempool** provides top UserOps sorted by gas price
2. **Bundler** creates a single transaction calling `EntryPoint.handleOps`
3. **Builder** positions this transaction in the middle of the block
4. **EntryPoint** atomically executes all UserOps in one transaction

This ensures:
- Gas-efficient bundling (one transaction for many UserOps)
- Atomic execution (all-or-nothing)
- MEV protection through positioning
- Beneficiary receives all bundle fees
