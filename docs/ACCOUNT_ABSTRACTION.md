# Account Abstraction (EIP-4337) Integration

## Overview

TIPS now supports EIP-4337 Account Abstraction through the ingress-rpc service. UserOperations are seamlessly converted into bundles and processed through the existing TIPS pipeline.

## Quick Start

1. **Configure the bundler**:
```bash
export TIPS_INGRESS_BUNDLER_PRIVATE_KEY="0x..."
export TIPS_INGRESS_ENTRY_POINTS="0x0000000071727De22E5E9d8BAf0edAc6f37da032"
```

2. **Start the service**:
```bash
cargo run --bin tips-ingress-rpc
```

3. **Send a UserOperation**:
```bash
curl -X POST http://localhost:8080 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "eth_sendUserOperation",
    "params": [{
      "sender": "0x...",
      "nonce": "0x0",
      "initCode": "0x",
      "callData": "0x...",
      "callGasLimit": "0x5208",
      "verificationGasLimit": "0x5208",
      "preVerificationGas": "0x5208",
      "maxFeePerGas": "0x3b9aca00",
      "maxPriorityFeePerGas": "0x3b9aca00",
      "paymasterAndData": "0x",
      "signature": "0x..."
    }, "0x0000000071727De22E5E9d8BAf0edAc6f37da032"],
    "id": 1
  }'
```

## Architecture

UserOperations follow the same path as regular bundles:

```
UserOperation → Convert to handleOps() tx → Bundle → Kafka → Bundle Pool → Builder
```

### Key Features

- **Version Support**: Both EIP-4337 v0.6 and v0.7+ UserOperations
- **Automatic Detection**: Version is detected from JSON fields automatically
- **Standard Pipeline**: UserOperations use the same bundling infrastructure as regular transactions
- **Audit Trail**: All UserOperations are logged through the audit system

## API Reference

### eth_sendUserOperation

Submits a UserOperation to be bundled and included on-chain.

**Parameters**:
1. `UserOperation` - The user operation object (v0.6 or v0.7)
2. `Address` - The EntryPoint contract address

**Returns**: `Hash` - The UserOperation hash

### eth_supportedEntryPoints

Returns the list of EntryPoint addresses supported by this bundler.

**Parameters**: None

**Returns**: `Address[]` - Array of supported EntryPoint addresses

## Configuration

| Environment Variable | Description | Default |
|---------------------|-------------|---------|
| `TIPS_INGRESS_BUNDLER_PRIVATE_KEY` | Bundler wallet private key | None (required for AA) |
| `TIPS_INGRESS_ENTRY_POINTS` | Comma-separated EntryPoint addresses | None |
| `TIPS_INGRESS_CHAIN_ID` | Chain ID for hash computation | 8453 (Base) |

## Implementation Status

**Current**: Scaffolding and pipeline integration complete
**Next**: UserOperation to transaction conversion logic

See `crates/ingress-rpc/ACCOUNT_ABSTRACTION.md` for detailed implementation status.

