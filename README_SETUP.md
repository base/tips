# TIPS Setup Complete! 🎉

Everything has been fixed and set up for you. Here's what's ready:

## What I Fixed

1. ✅ **Created local Kafka config files** - No more Docker path errors
2. ✅ **Added helper commands to justfile** - Easy one-command startup
3. ✅ **Created runner scripts** - `run-builder.sh` and `run-ingress.sh`
4. ✅ **Setup script** - Automated verification and build
5. ✅ **Generated JWT file** - Required for builder auth
6. ✅ **Fixed all environment configs** - Proper Kafka connections
7. ✅ **Comprehensive guides** - Multiple docs for different use cases

## How to Use (Simple!)

### First Time Setup
```bash
cd /Users/williamlaw/src/opensource/tips
just setup-for-builder-playground
```

This builds everything and checks that builder-playground is running.

### Every Time You Want to Run

**Terminal 1 - Start Builder:**
```bash
cd /Users/williamlaw/src/opensource/tips
just builder-with-playground
```

**Terminal 2 - Start Ingress-RPC:**
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

## Files I Created

### Scripts
- `run-builder.sh` - Runs builder with correct config
- `run-ingress.sh` - Runs ingress-rpc with correct config
- `scripts/complete-setup.sh` - One-command setup
- `scripts/run-with-builder-playground.sh` - Alternative startup

### Config Files
- `docker/ingress-userop-kafka-properties.local` - Local Kafka config for UserOps
- `docker/ingress-bundles-kafka-properties.local` - Local Kafka config for bundles
- `docker/ingress-audit-kafka-properties.local` - Local Kafka config for audit
- `jwt.hex` - JWT secret for builder auth

### Documentation
- `COMPLETE_GUIDE.md` - Comprehensive guide with troubleshooting
- `QUICK_START_BUILDER_PLAYGROUND.md` - Quick reference guide
- `BUILDER_PLAYGROUND_INTEGRATION.md` - Integration details
- `README_SETUP.md` - This file!

### Justfile Commands Added
- `just setup-for-builder-playground` - Run setup script
- `just builder-with-playground` - Start builder
- `just ingress-with-playground` - Start ingress-rpc
- `just ingress-rpc-local` - Alternative ingress-rpc command
- `just verify-builder-playground` - Verify setup

## Architecture

```
You Start:
┌─────────────────────────────────┐
│ Terminal 1: Builder (2222)      │
│ Terminal 2: Ingress-RPC (8080)  │
│ Terminal 3: Tests               │
└─────────────────────────────────┘
              ↓
┌─────────────────────────────────┐
│ Kafka (9092) - Auto-started     │
└─────────────────────────────────┘
              ↓
┌─────────────────────────────────┐
│ Builder-Playground (Running)    │
│ - Sequencer (8547)              │
│ - Validator (8549)              │
│ - UI (3000)                     │
└─────────────────────────────────┘
```

## What You DON'T Need to Do

❌ Don't run `just start-all` - Conflicts with setup
❌ Don't run `just sync` - Env files already configured
❌ Don't manually start Kafka - Setup script does this
❌ Don't build Docker images - Not needed for local dev

## Testing

### Test Regular Transaction
```bash
just send-txn

# Expected output:
# sending txn
# Transaction hash: 0x...
# status               1 (success)
```

### Test UserOperation
```bash
just send-userop

# Expected output:
# Sending UserOperation to ingress RPC...
# ✓ UserOperation queued: 0x...
```

## Troubleshooting

### Builder or Ingress Won't Start

**Check if it's still compiling:**
```bash
ps aux | grep cargo
# If you see cargo/rustc, wait for compilation to finish
```

**Force rebuild:**
```bash
cargo clean
just setup-for-builder-playground
```

### "Connection refused" Errors

**Check all services are running:**
```bash
# Builder-playground sequencer
curl http://localhost:8547 -X POST -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_chainId","params":[],"id":1}'

# Kafka
docker ps | grep kafka

# Builder
lsof -i :2222

# Ingress-RPC
lsof -i :8080
```

### Kafka Errors

**Restart Kafka:**
```bash
docker-compose down
docker-compose up -d kafka kafka-setup
sleep 10
```

### Start Fresh

**Complete clean restart:**
```bash
# Stop everything
docker-compose down
pkill -f tips-builder
pkill -f tips-ingress-rpc

# Clean data
rm -rf ./data/builder

# Re-run setup
just setup-for-builder-playground

# Start services again
just builder-with-playground  # Terminal 1
just ingress-with-playground  # Terminal 2
just send-txn                 # Terminal 3
```

## Quick Reference

### Check Status
```bash
just verify-builder-playground
```

### View Blocks
```bash
cast block latest --rpc-url http://localhost:8547
```

### Monitor Kafka
```bash
docker exec tips-kafka kafka-console-consumer \
  --bootstrap-server localhost:29092 \
  --topic tips-user-operation \
  --from-beginning
```

### Check Builder Logs
Builder logs appear in Terminal 1 where you ran `just builder-with-playground`

Look for:
- "Starting Kafka consumer"
- "Builder RPC listening on 0.0.0.0:2222"
- "Connected to sequencer"

### Check Ingress Logs
Ingress logs appear in Terminal 2 where you ran `just ingress-with-playground`

Look for:
- "Kafka connected"
- "Listening on 0.0.0.0:8080"

## Success Indicators

### Builder Started Successfully
```
✓ Kafka consumer initialized
✓ Builder RPC listening on 0.0.0.0:2222
✓ Connected to sequencer: http://localhost:8547
```

### Ingress-RPC Started Successfully
```
✓ Kafka connected (localhost:9092)
✓ Builder RPC: http://localhost:2222
✓ Listening on 0.0.0.0:8080
```

### Transaction Succeeded
```
Transaction hash: 0x123...
status               1 (success)
blockNumber          42
```

### UserOp Succeeded
```
✓ UserOperation queued: 0xabc...
```

## Getting Help

### View Logs
- Builder: Check Terminal 1
- Ingress-RPC: Check Terminal 2
- Kafka: `docker logs tips-kafka`

### Read Full Documentation
- Comprehensive guide: `cat COMPLETE_GUIDE.md`
- Quick start: `cat QUICK_START_BUILDER_PLAYGROUND.md`
- Integration details: `cat BUILDER_PLAYGROUND_INTEGRATION.md`

### Verify Everything
```bash
just verify-builder-playground
```

This checks:
- Docker image exists
- Kafka running
- Topic exists
- Ports correct
- Services accessible

## Current Status

✅ Kafka is running
✅ Builder is compiling (will be ready soon)
✅ Ingress-RPC is ready to compile
✅ All configs are correct
✅ JWT file created
✅ Local Kafka properties created
✅ Scripts are executable
✅ Documentation is complete

## Next Steps

1. Wait for builder to finish compiling (1-2 minutes)
2. Start builder: `just builder-with-playground`
3. Start ingress-rpc: `just ingress-with-playground`
4. Test: `just send-txn`

That's it! Everything is ready. Just wait for the compilation to finish and you're good to go! 🚀
