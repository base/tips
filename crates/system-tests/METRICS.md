# TIPS Load Testing

Multi-wallet concurrent load testing tool for measuring TIPS performance.

## Quick Start

```bash
# 1. Build
cargo build --release --bin load-test

# 2. Setup wallets
./target/release/load-test setup \
  --master-key 0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d \
  --output wallets.json

# 3. Run load test
./target/release/load-test load --wallets wallets.json --duration 60
```

---

## Configuration Options

### Setup Command

Create and fund test wallets from a master wallet. Test wallets are saved to allow test reproducibility and avoid the need to create new wallets for every test run.

**Usage:**
```bash
load-test setup --master-key <KEY> --output <FILE> [OPTIONS]
```

**Options:**

| Flag | Description | Default | Example |
|------|-------------|---------|---------|
| `--master-key` | Private key of funded wallet (required) | - | `0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d` |
| `--output` | Save wallets to JSON file (required) | - | `wallets.json` |
| `--sequencer` | L2 sequencer RPC URL | `http://localhost:8547` | `http://localhost:8547` |
| `--num-wallets` | Number of wallets to create | `100` | `100` |
| `--fund-amount` | ETH to fund each wallet | `0.1` | `0.5` |

**Environment Variables:**
- `MASTER_KEY` - Alternative to `--master-key` flag
- `SEQUENCER_URL` - Alternative to `--sequencer` flag

### Load Command

Run load test with funded wallets. Use the `--seed` flag to set the RNG seed for test reproducibility.

**Usage:**
```bash
load-test load --wallets <FILE> [OPTIONS]
```

**Options:**

| Flag | Description | Default | Example |
|------|-------------|---------|---------|
| `--wallets` | Path to wallets JSON file (required) | - | `wallets.json` |
| `--target` | TIPS ingress RPC URL | `http://localhost:8080` | `http://localhost:8080` |
| `--sequencer` | L2 sequencer RPC URL | `http://localhost:8547` | `http://localhost:8547` |
| `--rate` | Target transaction rate (tx/s) | `100` | `500` |
| `--duration` | Test duration in seconds | `300` | `60` |
| `--tx-timeout` | Timeout for tx inclusion (seconds) | `60` | `120` |
| `--seed` | Random seed for reproducibility | (none) | `42` |
| `--output` | Save metrics to JSON file | (none) | `metrics.json` |

**Environment Variables:**
- `INGRESS_URL` - Alternative to `--target` flag
- `SEQUENCER_URL` - Alternative to `--sequencer` flag

---
---

## Metrics Explained

### Output Example

```
Load Test Results
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Configuration:
  Target:              http://localhost:8080
  Sequencer:           http://localhost:8547
  Wallets:             100
  Target Rate:         100 tx/s
  Duration:            60s
  TX Timeout:          60s

Throughput:
  Sent:                100.0 tx/s (6000 total)
  Included:            98.5 tx/s (5910 total)
  Success Rate:        98.5%

Transaction Results:
  Included:            5910 (98.5%)
  Timed Out:           70 (1.2%)
  Send Errors:         20 (0.3%)
```

### Metrics Definitions

**Throughput:**
- `Sent Rate` - Transactions sent to TIPS per second
- `Included Rate` - Transactions included in blocks per second
- `Success Rate` - Percentage of sent transactions that were included

**Transaction Results:**
- `Included` - Successfully included in a block
- `Timed Out` - Not included within timeout period
- `Send Errors` - Failed to send to TIPS RPC

---

## Architecture

The runner uses:
- **Sender Tasks** - One async task per wallet sending at rate/N
- **Receipt Poller** - Background task polling sequencer for receipts every 2s
- **Transaction Tracker** - Concurrent data structure tracking all transaction states
- **Metrics Calculator** - Computes percentiles and aggregates using hdrhistogram

---
