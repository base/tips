# TIPS Load Testing & Metrics

Load testing tool for measuring TIPS performance under concurrent transaction load.

## Overview

The `tips-e2e-runner` is a binary tool that provides:
- **Multi-wallet concurrent sending** - Distribute load across N wallets
- **Time-to-inclusion tracking** - Measure from RPC call to receipt
- **Throughput metrics** - Track sent vs included transactions per second
- **Configurable rates** - Easily adjust target TPS
- **Reproducible tests** - Seed support for deterministic results
- **JSON export** - Save metrics for analysis

## Quick Start

### 1. Build the Runner

```bash
cd tips
cargo build --release --bin tips-e2e-runner
```

### 2. Setup Wallets

Fund N wallets from a master wallet (requires a funded account):

```bash
./target/release/tips-e2e-runner setup \
  --master-key 0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d \
  --sequencer http://localhost:8547 \
  --num-wallets 100 \
  --fund-amount 0.1 \
  --output wallets.json
```

**Parameters:**
- `--master-key`: Private key of funded wallet (e.g., Anvil test account)
- `--sequencer`: L2 sequencer RPC URL (default: http://localhost:8547)
- `--num-wallets`: Number of wallets to create (default: 100)
- `--fund-amount`: ETH to fund each wallet (default: 0.1)
- `--output`: Output file for wallet data (optional, e.g., wallets.json)

This will:
1. Generate N new random wallets
2. Send funding transactions to each wallet
3. Wait for confirmations
4. Save wallet details to JSON file (if `--output` is specified)

### 3. Run Load Test

Send transactions through TIPS and measure performance:

```bash
./target/release/tips-e2e-runner load \
  --target http://localhost:8080 \
  --sequencer http://localhost:8547 \
  --wallets wallets.json \
  --rate 100 \
  --duration 5m \
  --output metrics.json
```

**Parameters:**
- `--target`: TIPS ingress RPC URL (default: http://localhost:8080)
- `--sequencer`: L2 sequencer for nonce and receipt polling (default: http://localhost:8547)
- `--wallets`: Path to wallets JSON file (default: wallets.json)
- `--rate`: Target transaction rate in tx/s (default: 100)
- `--duration`: Test duration (e.g., "5m", "300s", "1h")
- `--tx-timeout`: Timeout for transaction inclusion in seconds (default: 60)
- `--seed`: Random seed for reproducibility (optional)
- `--output`: Output file for metrics JSON (optional)

## Example Output

```
🚀 Starting load test...

Configuration:
  Target:              http://localhost:8080
  Sequencer:           http://localhost:8547
  Wallets:             100
  Target Rate:         100 tx/s
  Rate per Wallet:     1.00 tx/s
  Duration:            5m 0s
  TX Timeout:          60s

[####################] 300s/300s | Sent: 30000

⏳ Waiting for pending transactions to resolve...
✅ All transactions resolved

Load Test Results
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Configuration:
  Target:              http://localhost:8080
  Sequencer:           http://localhost:8547
  Wallets:             100
  Target Rate:         100 tx/s
  Duration:            300s
  TX Timeout:          60s

Throughput:
  Sent:                100.0 tx/s (30000 total)
  Included:            98.5 tx/s (29550 total)
  Success Rate:        98.5%

Transaction Results:
  Included:            29550 (98.5%)
  Timed Out:           350 (1.2%)
  Send Errors:         100 (0.3%)

Time to Inclusion:
  p50:                 4200ms
  p95:                 8100ms
  p99:                 12300ms
  max:                 45700ms
  min:                 2100ms

💾 Metrics saved to: metrics.json
```

## Metrics Explained

### Throughput Metrics

- **Sent Rate**: Transactions sent to TIPS per second
- **Included Rate**: Transactions actually included in blocks per second
- **Success Rate**: Percentage of sent transactions that were included

### Time to Inclusion

- Measures time from `eth_sendRawTransaction` call to receipt available
- Includes TIPS processing, Kafka queuing, bundle pool, and block inclusion
- p50/p95/p99 are latency percentiles

### Transaction Results

- **Included**: Successfully included in a block
- **Timed Out**: Not included within timeout period (default 60s)
- **Send Errors**: Failed to send to TIPS RPC

## Use Cases

### Baseline Performance Testing
```bash
# Test at 100 tx/s for 5 minutes
tips-e2e-runner load --rate 100 --duration 5m
```

### Find Maximum Throughput
```bash
# Gradually increase rate to find breaking point
tips-e2e-runner load --rate 500 --duration 2m
tips-e2e-runner load --rate 1000 --duration 2m
tips-e2e-runner load --rate 2000 --duration 2m
```

### Reproducible Testing
```bash
# Use seed for identical test runs
tips-e2e-runner load --rate 100 --duration 5m --seed 42
```

### Long-Running Stability Test
```bash
# Run for 1 hour at moderate load
tips-e2e-runner load --rate 50 --duration 1h --output stability-test.json
```

## Architecture

The runner uses:
- **Sender Tasks**: One async task per wallet sending at rate/N
- **Receipt Poller**: Background task polling sequencer for receipts every 2s
- **Transaction Tracker**: Concurrent data structure tracking all transaction states
- **Metrics Calculator**: Computes percentiles and aggregates using hdrhistogram

## Troubleshooting

**Problem**: "Insufficient master wallet balance"
- **Solution**: Ensure master wallet has enough ETH (num_wallets × fund_amount + gas)

**Problem**: Many "Send Errors"
- **Solution**: TIPS service may be down or unreachable, check `just start-all`

**Problem**: High timeout rate
- **Solution**: Rate may be too high for system capacity, reduce `--rate` value

**Problem**: "Failed to load wallets"
- **Solution**: Run `setup` command first to create wallets.json

## Future Enhancements (Phase 2)

- Direct sequencer comparison mode (measure TIPS overhead)
- Burst and ramp load patterns
- Terminal visualization with live graphs
- Kafka and S3 audit verification
- Multiple wallet funding strategies

