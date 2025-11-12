# User Operations (EIP-4337) Setup

## Overview

The TIPS ingress-rpc service now supports `eth_sendUserOperation` which accepts EIP-4337 UserOperations and publishes them to a dedicated Kafka topic for processing by a bundler service.

## Kafka Topic

A new Kafka topic `tips-user-operations` has been created with:
- **Partitions**: 3
- **Replication Factor**: 1

## Configuration

### Environment Variables

For UserOperation support, configure these optional environment variables:

```bash
# Kafka properties file for user operations
TIPS_INGRESS_KAFKA_USER_OPS_PROPERTIES_FILE=/app/docker/user-operations-kafka-properties

# Kafka topic name
TIPS_INGRESS_KAFKA_USER_OPS_TOPIC=tips-user-operations
```

### Docker Setup

The `docker/user-operations-kafka-properties` file contains:
```properties
bootstrap.servers=host.docker.internal:9094
message.timeout.ms=5000
```

## Usage

### Starting with Docker Compose

```bash
# Start all services including Kafka with the new topic
docker-compose up -d

# Verify the topic was created
docker exec tips-kafka kafka-topics --list --bootstrap-server kafka:29092
```

You should see:
```
tips-audit
tips-ingress
tips-user-operations
```

### Sending a UserOperation

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

### Monitoring UserOperations

```bash
# View messages in the user-operations topic
docker exec tips-kafka kafka-console-consumer \
  --bootstrap-server kafka:29092 \
  --topic tips-user-operations \
  --from-beginning
```

## Message Format

UserOperations are published to Kafka as JSON with the following structure:

```json
{
  "user_operation": {
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
  },
  "entry_point": "0x0000000071727De22E5E9d8BAf0edAc6f37da032",
  "hash": "0x..."
}
```

## Architecture

```
User
  ↓
eth_sendUserOperation
  ↓
Ingress RPC Service
  ↓
Kafka Topic: tips-user-operations
  ↓
[Future] Bundler Service
  ↓
Kafka Topic: tips-ingress
  ↓
Bundle Pool → Builder
```

## Next Steps

To complete the flow, a bundler service needs to be created that:

1. **Consumes** from `tips-user-operations` topic
2. **Converts** UserOperations to EntryPoint transactions
3. **Creates** bundles from those transactions
4. **Publishes** to `tips-ingress` topic

This keeps the ingress service simple and delegates complex bundling logic to a dedicated service.

## Local Development

For local development without Docker:

```bash
# Set environment variables
export TIPS_INGRESS_KAFKA_USER_OPS_PROPERTIES_FILE=docker/user-operations-kafka-properties
export TIPS_INGRESS_KAFKA_USER_OPS_TOPIC=tips-user-operations

# Run the service
cargo run --bin tips-ingress-rpc
```

## Disabling UserOperations

UserOperation support is optional. If you don't set `TIPS_INGRESS_KAFKA_USER_OPS_PROPERTIES_FILE`, the service will:
- Still accept `eth_sendUserOperation` calls
- Log the UserOperation
- Return a hash
- But **not** publish to Kafka

This allows for gradual rollout and testing.

