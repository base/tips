![Base](./docs/logo.png)

# TIPS - Transaction Inclusion & Prioritization Stack

> [!WARNING]
> This repository is an experiment to enable bundles, transaction simulation and transaction tracing for Base. 
> It's being used to explore ideas and experiment. It is currently not production ready.

## Architecture Overview

The project consists of several components:

### 🗄️ Datastore (`crates/datastore`)
Postgres storage layer that provides API's to persist and retrieve bundles.

### 📊 Audit (`crates/audit`)
Event streaming and archival system that:
- Provides an API to publish bundle events to Kafka
- Archives bundle history to S3 for long-term storage
- See [S3 Storage Format](docs/AUDIT_S3_FORMAT.md) for data structure details

### 🔌 Ingress RPC (`crates/ingress-rpc`)
The main entry point that provides a JSON-RPC API for receiving transactions and bundles.

### 🔨 Maintenance (`crates/maintenance`)
A service that maintains the health of the TIPS DataStore, by removing stale or included bundles.

### ✍️ Ingress Writer (`crates/ingress-writer`)
A service that consumes bundles from Kafka and persists them to the datastore.

### 🖥️ UI (`ui`)
A debug UI for viewing the state of the bundle store and S3.

### 🧪 Simulator (`crates/simulator`)
A Reth-based execution client that:
- Simulates bundles to estimate resource usage (e.g. execution time)
- Provides transaction tracing and simulation capabilities
- Syncs from production sequencer via an op-node instance (simulator-cl)
- Used by the block builder stack to throttle transactions based on resource consumption

## 🏗️ Block Builder Stack

The block builder stack enables production-ready block building with TIPS bundle integration. It consists of:

**builder-cl**: An op-node instance running in sequencer mode that:
- Syncs from production sequencer via P2P
- Drives block building through Engine API calls
- Does not submit blocks to L1 (shadow sequencer mode)

**builder**: A modified op-rbuilder instance that:
- Receives Engine API calls from builder-cl
- Queries TIPS datastore for bundles with resource usage estimates from the simulator
- Builds blocks including eligible bundles while respecting resource constraints

**Prerequisites**:
- [builder-playground](https://github.com/flashbots/builder-playground) running locally with the `niran:authorize-signers` branch
- op-rbuilder Docker image built using `just build-rbuilder`

**Quick Start**:
```bash
# Build op-rbuilder (optionally from a specific branch)
just build-rbuilder

# Start the builder stack (requires builder-playground running)
just start-builder
```

The builder-cl syncs from the production sequencer via P2P while op-rbuilder builds blocks with TIPS bundles. Built blocks are not submitted to L1, making this safe for testing and development.
