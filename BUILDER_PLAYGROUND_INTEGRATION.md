# TIPS Builder + Builder-Playground Integration Guide

## Overview

This guide verifies that TIPS builder works correctly with builder-playground replacing op-rbuilder.

## Prerequisites

### 1. Build tips-builder Docker Image

The builder-playground integration expects a Docker image named `tips-builder:latest`.

```bash
# In the tips repo
cd /Users/williamlaw/src/opensource/tips

# Build the Docker image
docker build -t tips-builder:latest .
```

Verify the image exists:
```bash
docker images | grep tips-builder
# Should show: tips-builder   latest   ...
```

### 2. Verify TIPS Configuration

Check your TIPS environment files have the correct settings:

**`.env` (for local development):**
```bash
# Kafka (local)
TIPS_BUILDER_KAFKA_BROKERS=localhost:9092
TIPS_BUILDER_KAFKA_TOPIC=tips-user-operation
TIPS_BUILDER_KAFKA_GROUP_ID=tips-builder

# Bundler
TIPS_BUILDER_BUNDLER_ADDRESS=0x1111111111111111111111111111111111111111
TIPS_BUILDER_ENTRY_POINT=0x0000000071727De22E5E9d8BAf0edAc6f37da032
```

**`.env.docker` (for Docker):**
```bash
# Kafka (Docker)
TIPS_BUILDER_KAFKA_BROKERS=host.docker.internal:9094
TIPS_BUILDER_KAFKA_TOPIC=tips-user-operation
TIPS_BUILDER_KAFKA_GROUP_ID=tips-builder
```

## Integration Verification Steps

### Step 1: Verify Kafka Compatibility

The builder-playground should deploy Kafka that's compatible with TIPS:

**Required Kafka Configuration:**
- Bootstrap server ports: `9092` (host), `9094` (docker)
- Topic: `tips-user-operation` (with 3 partitions)
- Replication factor: 1 (for local dev)

**Verify with builder-playground:**
```bash
# After running builder-playground with tips-builder
docker exec -it <kafka-container-name> kafka-topics --list --bootstrap-server localhost:29092 | grep tips-user-operation
```

If the topic doesn't exist, create it:
```bash
docker exec -it <kafka-container-name> kafka-topics \
  --create \
  --if-not-exists \
  --topic tips-user-operation \
  --bootstrap-server localhost:29092 \
  --partitions 3 \
  --replication-factor 1
```

### Step 2: Run Builder-Playground with TIPS Builder

```bash
cd /path/to/builder-playground

# Run with tips-builder
./scripts/build-tips-builder.sh
builder-playground cook opstack --external-builder tips-builder
```

This should:
- ✅ Deploy Kafka
- ✅ Deploy tips-builder container
- ✅ Connect builder to Kafka
- ✅ Connect builder to sequencer/validator

### Step 3: Verify TIPS Ingress-RPC (Runs Separately)

TIPS ingress-rpc runs **outside** builder-playground and publishes UserOps to Kafka:

```bash
# In tips repo - Terminal 1
cd /Users/williamlaw/src/opensource/tips
just ingress-rpc
```

Verify it's running:
```bash
curl http://localhost:8080 -X POST -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_chainId","params":[],"id":1}'
```

### Step 4: Test Regular Transactions

```bash
# In tips repo
just send-txn
```

Expected flow:
1. Transaction sent to ingress-rpc (port 8080)
2. Ingress-rpc forwards to builder via builder-playground
3. Builder includes in block
4. Transaction appears on-chain

### Step 5: Test UserOperations

```bash
# In tips repo
just send-userop
```

Expected flow:
1. UserOp sent to ingress-rpc (port 8080)
2. Ingress-rpc validates and publishes to Kafka
3. Builder consumes from Kafka
4. Builder creates bundle with handleOps()
5. Bundler tx inserted at block midpoint
6. UserOps executed on-chain

## Port Configuration Reference

### TIPS Ports (Your Setup)
- **8080**: Ingress-RPC (receives txs and UserOps)
- **9092**: Kafka (local access)
- **9094**: Kafka (Docker access via host.docker.internal)
- **2222**: Builder RPC (block building)
- **8547**: Sequencer RPC
- **8549**: Validator RPC

### Builder-Playground Expected Ports
Should match TIPS configuration above, specifically:
- Kafka must be accessible at `localhost:9092` and `host.docker.internal:9094`
- Builder should expose RPC on port `2222`
- Builder should connect to sequencer/validator at their ports

## Common Issues & Fixes

### Issue 1: Kafka Connection Failed

**Symptom:**
```
ERROR Kafka consumer error: Failed to connect to broker
```

**Fix:**
```bash
# Check Kafka is running
docker ps | grep kafka

# Check Kafka ports
docker port <kafka-container-name>

# Verify topic exists
docker exec <kafka-container-name> kafka-topics --list --bootstrap-server localhost:29092
```

### Issue 2: UserOps Not Appearing in Blocks

**Symptom:**
```bash
just send-userop  # Returns success but UserOp not in block
```

**Debug steps:**

1. Check ingress-rpc logs:
```bash
# Should see: "User operation queued"
```

2. Check Kafka has the message:
```bash
docker exec <kafka-container-name> kafka-console-consumer \
  --bootstrap-server localhost:29092 \
  --topic tips-user-operation \
  --from-beginning \
  --max-messages 1
```

3. Check builder logs:
```bash
docker logs <tips-builder-container>
# Should see: "Received user operation"
# Should see: "Flushing user operations bundle"
# Should see: "Bundler tx inserted"
```

### Issue 3: Builder Can't Reach Kafka

**Symptom:**
```
ERROR Failed to create Kafka consumer
```

**Fix:**
Verify builder container uses correct Kafka address:
- If builder runs in Docker: Use `host.docker.internal:9094`
- If builder runs locally: Use `localhost:9092`

Check builder's environment variables:
```bash
docker inspect <tips-builder-container> | grep KAFKA_BROKERS
```

Should show:
```
TIPS_BUILDER_KAFKA_BROKERS=host.docker.internal:9094
```

### Issue 4: Port Conflicts

**Symptom:**
```
Error: port 9092 already in use
```

**Fix:**
```bash
# Stop TIPS Kafka if running
docker-compose -f /Users/williamlaw/src/opensource/tips/docker-compose.yml down

# Let builder-playground manage Kafka
builder-playground cook opstack --external-builder tips-builder
```

## Testing Checklist

Use this checklist to verify everything works:

### Pre-Integration
- [ ] TIPS Docker image built: `tips-builder:latest`
- [ ] `.env` and `.env.docker` files configured
- [ ] Builder-playground cloned and built

### With Builder-Playground Running
- [ ] Kafka deployed and accessible
- [ ] Topic `tips-user-operation` exists
- [ ] Builder container running
- [ ] Builder connected to Kafka
- [ ] Builder connected to sequencer/validator

### With TIPS Ingress-RPC Running
- [ ] Ingress-RPC accessible on port 8080
- [ ] `curl` health check succeeds
- [ ] Ingress-RPC can publish to Kafka

### End-to-End Tests
- [ ] `just send-txn` succeeds
- [ ] Transaction appears in block
- [ ] `just send-userop` succeeds
- [ ] UserOp published to Kafka
- [ ] Builder consumes UserOp
- [ ] Bundle created
- [ ] Bundler tx in block at midpoint
- [ ] UserOp executed on-chain

### Monitoring
- [ ] Kafka consumer lag is 0:
  ```bash
  docker exec <kafka-container> kafka-consumer-groups \
    --bootstrap-server localhost:29092 \
    --describe \
    --group tips-builder
  ```
- [ ] Builder logs show UserOp processing
- [ ] Blocks being produced regularly

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                     Your Local Machine                       │
│                                                              │
│  ┌────────────────┐                                         │
│  │ TIPS Repo      │                                         │
│  │                │                                         │
│  │ just ingress-rpc (port 8080)                            │
│  │   │                                                      │
│  │   ├─→ Validates txs/UserOps                            │
│  │   ├─→ Publishes to Kafka                               │
│  │   └─→ Forwards txs to builder                          │
│  │                                                          │
│  │ just send-txn                                           │
│  │ just send-userop                                        │
│  └────────────────┘                                         │
│                                                              │
│         ↓ (publishes)                                        │
│                                                              │
│  ┌────────────────────────────────────────────────┐        │
│  │    Builder-Playground Managed Services         │        │
│  │                                                 │        │
│  │  ┌─────────────────┐                           │        │
│  │  │ Kafka           │ (ports 9092, 9094)        │        │
│  │  │ Topic: tips-user-operation                  │        │
│  │  └─────────────────┘                           │        │
│  │         ↓                                       │        │
│  │  ┌─────────────────┐                           │        │
│  │  │ TIPS Builder    │ (port 2222)               │        │
│  │  │ (tips-builder:latest)                       │        │
│  │  │  • Consumes from Kafka                      │        │
│  │  │  • Builds blocks                            │        │
│  │  │  • Inserts bundler tx at midpoint           │        │
│  │  └─────────────────┘                           │        │
│  │         ↓                                       │        │
│  │  ┌─────────────────┐                           │        │
│  │  │ Sequencer       │ (port 8547)               │        │
│  │  └─────────────────┘                           │        │
│  │         ↓                                       │        │
│  │  ┌─────────────────┐                           │        │
│  │  │ Validator       │ (port 8549)               │        │
│  │  └─────────────────┘                           │        │
│  └────────────────────────────────────────────────┘        │
└─────────────────────────────────────────────────────────────┘
```

## Expected Behavior

### Regular Transaction Flow
1. `just send-txn` → ingress-rpc:8080
2. ingress-rpc → builder:2222
3. builder → includes in block
4. block → sequencer → validator

### UserOperation Flow
1. `just send-userop` → ingress-rpc:8080
2. ingress-rpc → validates → Kafka
3. Kafka → builder (consumer)
4. builder → creates bundle
5. builder → inserts bundler tx at midpoint:
   ```
   Block:
   [TX, TX, TX, BUNDLER_TX, TX, TX, TX]
                ^^^^^^^^^^^
                EntryPoint.handleOps()
   ```
6. block → sequencer → validator

## Next Steps After Verification

Once everything works:

1. **Monitor Performance:**
   ```bash
   # Watch builder logs
   docker logs -f <tips-builder-container>

   # Watch Kafka lag
   watch -n 1 "docker exec <kafka-container> kafka-consumer-groups ..."
   ```

2. **Test Load:**
   ```bash
   # Send multiple UserOps
   for i in {1..10}; do just send-userop; sleep 1; done
   ```

3. **Verify Bundle Insertion:**
   ```bash
   # Check block structure
   cast block latest --rpc-url http://localhost:8547
   ```

4. **Check EntryPoint Events:**
   ```bash
   # Look for UserOperationEvent
   cast logs 0x0000000071727De22E5E9d8BAf0edAc6f37da032 \
     --rpc-url http://localhost:8547
   ```
