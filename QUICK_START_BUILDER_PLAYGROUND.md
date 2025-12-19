# Quick Start: TIPS with Builder-Playground

## Complete Setup (3 Simple Steps)

### Step 1: Build TIPS Docker Image

```bash
cd /Users/williamlaw/src/opensource/tips
docker build -t tips-builder:latest .
```

### Step 2: Start Builder-Playground

In a separate terminal:

```bash
cd /path/to/builder-playground
go run main.go cook opstack \
  --external-builder http://host.docker.internal:2222 \
  --enable-latest-fork 0 \
  --flashblocks \
  --base-overlay \
  --flashblocks-builder ws://host.docker.internal:1111/ws
```

This will start:
- ✓ Kafka (ports 9092, 9094)
- ✓ TIPS Builder (port 2222)
- ✓ Sequencer (port 8547)
- ✓ Validator (port 8549)
- ✓ UI (port 3000)

### Step 3: Start TIPS Ingress-RPC

In another terminal:

```bash
cd /Users/williamlaw/src/opensource/tips
just run-with-builder-playground
```

This script will:
- ✓ Check if Docker image exists
- ✓ Stop conflicting TIPS services
- ✓ Wait for Kafka to be ready
- ✓ Verify builder and sequencer are accessible
- ✓ Start ingress-rpc with correct Kafka config

## Testing

### Test Regular Transactions

```bash
just send-txn
```

Expected output:
```
sending txn
Transaction hash: 0x...
status               1 (success)
```

### Test UserOperations

```bash
just send-userop
```

Expected output:
```
Sending UserOperation to ingress RPC...
✓ UserOperation queued: 0x...
```

## Troubleshooting

### Quick Verification

```bash
just verify-builder-playground
```

This checks:
- Docker image exists
- Kafka is running
- Topic exists
- Ingress-RPC accessible
- Builder running
- All ports correct

### Common Issues

**Issue: "Kafka not accessible"**
```bash
# Make sure builder-playground is running
# Check if Kafka port is listening
lsof -i :9092
```

**Issue: "Connection refused on port 2222"**
```bash
# Wait for builder-playground to fully start
# Check builder logs
docker logs <tips-builder-container>
```

**Issue: "Cannot run ingress-rpc"**
```bash
# Use the helper command instead
just run-with-builder-playground

# Or manually with correct env
just ingress-rpc-local
```

## Architecture

```
┌─────────────────────────────────────────────┐
│           Builder-Playground Manages         │
│                                              │
│  Kafka (9092) ──→ Builder (2222)            │
│                      ↓                       │
│                  Sequencer (8547)            │
│                      ↓                       │
│                  Validator (8549)            │
└─────────────────────────────────────────────┘
                      ↑
                      │
           ┌──────────┴──────────┐
           │  TIPS Runs Locally  │
           │                     │
           │  Ingress-RPC (8080) │
           │    • Receives txs    │
           │    • Validates       │
           │    • Publishes       │
           └─────────────────────┘
                      ↑
                      │
                  Your Tests
              (just send-txn)
              (just send-userop)
```

## What NOT to Do

❌ **Don't run `just start-all`** - This starts TIPS's own Docker services which conflict with builder-playground

❌ **Don't run `just sync`** - This resets env files which you don't need

❌ **Don't run `just ingress-rpc`** - Use `just run-with-builder-playground` or `just ingress-rpc-local` instead

## Transaction Flow

### Regular Transaction
1. `just send-txn` → ingress-rpc:8080
2. ingress-rpc → builder:2222
3. builder → includes in block
4. block → sequencer:8547 → validator:8549

### UserOperation
1. `just send-userop` → ingress-rpc:8080
2. ingress-rpc validates → publishes to Kafka
3. Kafka → builder consumes
4. builder creates bundle with handleOps()
5. builder inserts bundler tx at block midpoint
6. block → sequencer → validator

## Next Steps

Once everything works:

1. **Monitor Kafka lag**:
   ```bash
   docker exec <kafka-container> kafka-consumer-groups \
     --bootstrap-server localhost:29092 \
     --describe \
     --group tips-builder
   ```

2. **Check block structure**:
   ```bash
   cast block latest --rpc-url http://localhost:8547
   ```

3. **View UI**:
   ```
   http://localhost:3000
   ```

4. **Send multiple UserOps**:
   ```bash
   for i in {1..5}; do just send-userop; sleep 1; done
   ```
