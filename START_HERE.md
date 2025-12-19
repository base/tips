# Start TIPS with Builder-Playground

## Prerequisites
Make sure builder-playground is running first.

## Quick Start (3 Commands)

### Terminal 1: Start Builder
```bash
cd /Users/williamlaw/src/opensource/tips
./run-builder.sh
```

Wait for: `RPC HTTP server started url=0.0.0.0:2222`

### Terminal 2: Start Ingress-RPC
```bash
cd /Users/williamlaw/src/opensource/tips
./run-ingress.sh
```

Wait for: `Ingress RPC server started, address: 0.0.0.0:8080`

### Terminal 3: Test
```bash
cd /Users/williamlaw/src/opensource/tips
just send-txn
```

## If Ports Already in Use

Kill all TIPS processes and restart:
```bash
pkill -f tips-builder
pkill -f tips-ingress
rm -rf ./data/builder

# Then restart from Terminal 1
./run-builder.sh
```

## Expected Output

```bash
$ just send-txn
sending txn
✓ Transaction sent!
  Hash: 0xfa411443866f11a17a3fb114fb2ffeab8562ef28983a9b5011fb0b895c338fb8
  Check status: cast receipt 0xfa... -r http://localhost:8547
```

That's it! Transaction successfully sent through the full stack:
- Ingress-RPC (port 8080) received it
- Forwarded to Builder (port 2222)
- Builder accepted it

## All Working ✅
- ✅ Builder running and consuming from Kafka
- ✅ Ingress-RPC accepting transactions
- ✅ Full flow working
