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
