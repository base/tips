# Simple Account Abstraction Endpoint

## Overview

A minimal implementation of `eth_sendUserOperation` that accepts EIP-4337 UserOperations and prepares them for bundling via Kafka.

## Implementation

### What It Does

1. **Accepts UserOperations** via `eth_sendUserOperation(userOp, entryPoint)`
2. **Logs the request** with sender, entry point, and nonce
3. **Returns a hash** for tracking

### What's Next (TODO)

- Push UserOperation to Kafka for bundler to process
- The bundler will handle conversion to EntryPoint transactions
- The bundler will create bundles and submit them

## Usage

### Call the Endpoint

```bash
curl -X POST http://localhost:8080 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "eth_sendUserOperation",
    "params": [
      {
        "sender": "0x1234567890123456789012345678901234567890",
        "nonce": "0x0",
        "initCode": "0x",
        "callData": "0xabcd",
        "callGasLimit": "0x5208",
        "verificationGasLimit": "0x5208",
        "preVerificationGas": "0x5208",
        "maxFeePerGas": "0x3b9aca00",
        "maxPriorityFeePerGas": "0x3b9aca00",
        "paymasterAndData": "0x",
        "signature": "0x"
      },
      "0x0000000071727De22E5E9d8BAf0edAc6f37da032"
    ],
    "id": 1
  }'
```

### Response

```json
{
  "jsonrpc": "2.0",
  "result": "0x...",
  "id": 1
}
```

## Files

- **`src/user_operation.rs`** - UserOperation types (v0.6 and v0.7)
- **`src/service.rs`** - RPC endpoint implementation
- **`src/lib.rs`** - Module exports

## Next Steps

To complete the implementation:

1. **Push to Kafka** - Send UserOperation to a dedicated Kafka topic
2. **Bundler Service** - Create a service that:
   - Consumes UserOperations from Kafka
   - Converts them to EntryPoint transactions
   - Creates bundles
   - Pushes to the existing tips-ingress Kafka topic

This keeps the ingress simple and moves the complexity to a dedicated bundler service.

