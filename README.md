![Build Status](https://img.shields.io/github/actions/workflow/status/base/tips/ci.yml?branch=master)
![Test Coverage](https://img.shields.io/codecov/c/github/base/tips)
![Code Quality](https://img.shields.io/github/checks-status/base/tips/master)

![Base](./docs/logo.png)

# TIPS - Transaction Inclusion & Prioritization Stack

## Documentation

- [API Documentation](docs/API.md)
- [Audit S3 Storage Format](docs/AUDIT_S3_FORMAT.md)
- [Bundle States](docs/BUNDLE_STATES.md)

> [!WARNING]
> This repository is an experiment to enable bundles, transaction simulation and transaction tracing for Base. 
> It's being used to explore ideas and experiment. It is currently not production ready.

## Architecture Overview

The project consists of several components:

### 🗄️ Datastore (`crates/datastore`)

Postgres storage layer that provides API's to persist and retrieve bundles. This component handles all database operations including bundle storage, retrieval, and state management with optimized queries for high-throughput bundle processing.

### 📊 Audit (`crates/audit`)

Event streaming and archival system that:

- Provides an API to publish bundle events to Kafka
- Archives bundle history to S3 for long-term storage
- See [S3 Storage Format](docs/AUDIT_S3_FORMAT.md) for data structure details

This component ensures complete audit trails and enables historical analysis of bundle processing with scalable event streaming architecture.

### 🔌 Ingress RPC (`crates/ingress-rpc`)

The main entry point that provides a JSON-RPC API for receiving transactions and bundles. This service validates incoming requests, performs initial bundle validation, and routes submissions to the appropriate processing pipeline with support for standard Ethereum RPC methods.

### ✍️ Ingress Writer (`crates/ingress-writer`)

A service that consumes bundles from Kafka and persists them to the datastore. This component acts as the bridge between the event streaming layer and persistent storage, ensuring reliable bundle persistence with retry logic and error handling.

### 🔨 Maintenance (`crates/maintenance`)

A service that maintains the health of the TIPS DataStore, by removing stale or included bundles. This background worker performs periodic cleanup operations, monitors database health, and ensures optimal performance by pruning outdated data according to configurable retention policies.

### 🖥️ UI (`ui`)

A debug UI for viewing the state of the bundle store and S3. This web-based interface provides real-time visibility into bundle processing, status monitoring, and historical data inspection for debugging and operational insights.
