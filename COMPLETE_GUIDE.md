# Complete Setup Guide for TIPS with Builder-Playground

## Quick Start (3 Commands)

### 1. Run Setup (First Time Only)
```bash
cd /Users/williamlaw/src/opensource/tips
just setup-for-builder-playground
```

This will:
- ✓ Check builder-playground is running
- ✓ Start Kafka
- ✓ Create UserOp topic
- ✓ Build tips-builder
- ✓ Build tips-ingress-rpc

### 2. Start Services (3 Terminals)

**Terminal 1 - Builder:**
```bash
cd /Users/williamlaw/src/opensource/tips
just builder-with-playground
```

**Terminal 2 - Ingress-RPC:**
```bash
cd /Users/williamlaw/src/opensource/tips
just ingress-with-playground
```

**Terminal 3 - Test:**
```bash
cd /Users/williamlaw/src/opensource/tips

# Test regular transaction
just send-txn

# Test UserOperation
just send-userop
```

## What Each Component Does

### Builder-Playground (Already Running)
- Sequencer (port 8547) - Main chain
- Validator (port 8549) - Validator node
- UI (port 3000) - Block explorer

### TIPS Services (You Start)
- Kafka (port 9092) - Message queue for UserOps
- Builder (port 2222) - Consumes UserOps, builds blocks
- Ingress-RPC (port 8080) - Receives txs and UserOps

## Transaction Flow

### Regular Transaction
```
User → just send-txn
  ↓
Ingress-RPC (8080) validates
  ↓
Builder (2222) includes in block
  ↓
Sequencer (8547) finalizes
  ↓
Block appears on chain
```

### UserOperation
```
User → just send-userop
  ↓
Ingress-RPC (8080) validates
  ↓
Kafka (9092) queues
  ↓
Builder (2222) consumes from Kafka
  ↓
Builder creates handleOps() bundle
  ↓
Builder inserts at block midpoint
  ↓
Sequencer (8547) finalizes
  ↓
UserOp executed on chain
```

## Troubleshooting

### Issue: "Builder-playground sequencer not accessible"
**Solution:**
```bash
cd /path/to/builder-playground
go run main.go cook opstack --enable-latest-fork 0 --flashblocks --base-overlay
```

### Issue: "Kafka not accessible"
**Solution:**
```bash
cd /Users/williamlaw/src/opensource/tips
docker-compose up -d kafka kafka-setup
sleep 10
```

### Issue: "Connection refused on port 2222"
**Solution:** Builder hasn't started yet. Check logs:
```bash
# Builder logs should show:
# "Starting Kafka consumer"
# "Builder RPC listening on 0.0.0.0:2222"
```

### Issue: "Transaction failed"
**Solution:** Check all services are running:
```bash
# Check ports
lsof -i :9092  # Kafka
lsof -i :2222  # Builder
lsof -i :8080  # Ingress-RPC
lsof -i :8547  # Sequencer

# Check Docker
docker ps | grep kafka

# Check builder logs
# (should see Kafka consumer starting)
```

## Architecture Diagram

```
┌─────────────────────────────────────────┐
│      Builder-Playground (Running)       │
│                                          │
│  Sequencer (8547) ← Builder connects    │
│  Validator (8549)                        │
│  UI (3000)                               │
└─────────────────────────────────────────┘
                ↑
                │
┌───────────────┴─────────────────────────┐
│         TIPS Services (You Start)       │
│                                          │
│  Kafka (9092)                           │
│     ↓                                    │
│  Builder (2222) ─────→ Sequencer        │
│     ↑                                    │
│  Ingress-RPC (8080)                     │
│     ↑                                    │
└─────┼───────────────────────────────────┘
      │
   Your Tests
```

## Verifying Everything Works

### 1. Check Services
```bash
# All should respond
curl http://localhost:8547 -X POST -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_chainId","params":[],"id":1}'

curl http://localhost:2222 -X POST -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_chainId","params":[],"id":1}'

curl http://localhost:8080 -X POST -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_chainId","params":[],"id":1}'
```

### 2. Check Kafka
```bash
# Topic should exist
docker exec tips-kafka kafka-topics --list --bootstrap-server localhost:29092

# Should show: tips-user-operation
```

### 3. Send Test Transaction
```bash
just send-txn
# Should output:
# Transaction hash: 0x...
# status: 1 (success)
```

### 4. Send Test UserOp
```bash
just send-userop
# Should output:
# ✓ UserOperation queued: 0x...
```

### 5. Check Builder Logs
Builder logs should show:
```
Starting Kafka consumer
Subscribed to topic: tips-user-operation
Builder RPC listening on 0.0.0.0:2222
Connected to sequencer: http://localhost:8547
```

### 6. Check Kafka Consumer
```bash
docker exec tips-kafka kafka-consumer-groups \
  --bootstrap-server localhost:29092 \
  --describe \
  --group tips-builder

# Should show:
# GROUP           TOPIC               PARTITION  CURRENT-OFFSET  LOG-END-OFFSET  LAG
# tips-builder    tips-user-operation 0          5               5               0
# tips-builder    tips-user-operation 1          3               3               0
# tips-builder    tips-user-operation 2          2               2               0
```

## Advanced Usage

### Monitor Kafka Messages
```bash
# Watch UserOps being published
docker exec tips-kafka kafka-console-consumer \
  --bootstrap-server localhost:29092 \
  --topic tips-user-operation \
  --from-beginning
```

### Send Multiple UserOps
```bash
for i in {1..10}; do
  just send-userop
  sleep 1
done
```

### View Blocks
```bash
# Latest block
cast block latest --rpc-url http://localhost:8547

# Specific block
cast block 100 --rpc-url http://localhost:8547
```

### View Transactions in Block
```bash
# Get latest block number
BLOCK=$(cast block latest --rpc-url http://localhost:8547 | grep number | awk '{print $2}')

# Get transactions in block
cast block $BLOCK --rpc-url http://localhost:8547
```

## Stopping Services

### Stop TIPS Services
```bash
# Stop builder and ingress-rpc (Ctrl+C in their terminals)

# Stop Kafka
cd /Users/williamlaw/src/opensource/tips
docker-compose down
```

### Stop Builder-Playground
```bash
# Ctrl+C in builder-playground terminal
```

## Clean Restart

### Full Clean Restart
```bash
# 1. Stop everything
cd /Users/williamlaw/src/opensource/tips
docker-compose down
pkill -f tips-builder
pkill -f tips-ingress-rpc

# 2. Clean data
rm -rf ./data/builder

# 3. Start builder-playground
cd /path/to/builder-playground
go run main.go cook opstack --enable-latest-fork 0 --flashblocks --base-overlay

# 4. Run TIPS setup
cd /Users/williamlaw/src/opensource/tips
just setup-for-builder-playground

# 5. Start services
# Terminal 1:
just builder-with-playground

# Terminal 2:
just ingress-with-playground

# Terminal 3:
just send-txn
```

## Summary of Commands

```bash
# Setup (first time)
just setup-for-builder-playground

# Run builder
just builder-with-playground

# Run ingress-rpc
just ingress-with-playground

# Test transaction
just send-txn

# Test UserOp
just send-userop

# Verify setup
just verify-builder-playground

# Stop Kafka
docker-compose down

# Clean data
rm -rf ./data/builder
```

## Expected Output

### Successful Transaction
```bash
$ just send-txn
sending txn
Transaction hash: 0x1234...
status               1 (success)
blockNumber          42
gasUsed              21000
```

### Successful UserOp
```bash
$ just send-userop
Sending UserOperation to ingress RPC...
Response: {"jsonrpc":"2.0","id":1,"result":{"user_operation_hash":"0xabcd..."}}
✓ UserOperation queued: 0xabcd...
```

### Builder Logs (Expected)
```
Starting tips-builder...
Initializing Kafka consumer
  Brokers: localhost:9092
  Topic: tips-user-operation
  Group: tips-builder
✓ Kafka consumer ready
Starting builder RPC server
✓ Builder RPC listening on 0.0.0.0:2222
Connecting to sequencer
✓ Connected to http://localhost:8547
Builder ready to process transactions
```

### Ingress-RPC Logs (Expected)
```
Starting tips-ingress-rpc...
Loading configuration
✓ Kafka connected (localhost:9092)
✓ Builder RPC: http://localhost:2222
✓ Simulation RPC: http://localhost:8549
Starting HTTP server
✓ Listening on 0.0.0.0:8080
Ingress-RPC ready
```
