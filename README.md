# tips
A prototype of a Transaction Inclusion Pipeline Service for private sequencers that does not use the P2P mempool. The project
aims to increase throughput, improve transaction tracking, reduce latency and add support for bundles.

This project is currently at:

https://github.com/flashbots/builder-playground
https://github.com/base/tips/pull/new/prototype
https://github.com/base/op-rbuilder/pull/new/tips-prototype

## Architecture Overview

The project consists of three main crates:

### 🔌 Ingress (`crates/ingress`)
The main entry point that provides a JSON-RPC API for receiving transactions and bundles. It handles:
- Transaction validation and processing
- Bundle creation and management
- Dual-write capability to legacy mempool
- PostgreSQL persistence via the datastore layer

### 🗄️ Datastore (`crates/datastore`)
PostgreSQL-based storage layer that provides:
- Bundle persistence and retrieval
- Transaction tracking and indexing
- Database schema management via migrations
- See [Database Schema](crates/datastore/migrations/1757444171_create_bundles_table.sql) for table structure

### 📊 Audit (`crates/audit`)
Event streaming and archival system that:
- Publishes bundle lifecycle events to Kafka
- Archives bundle history to S3 for long-term storage
- Provides transaction lookup capabilities
- See [S3 Storage Format](crates/audit/S3_FORMAT.md) for data structure details


### Local Development
You can run the whole system locally with:

tips:
```sh
just db && sleep 3 && just ingress
```

builder-playground:
```sh
# TODO: Figure out the flashblocks/websocket proxy/validator setup
go run main.go cook opstack --external-builder http://host.docker.internal:4444 --enable-latest-fork 0
```

op-rbuilder:
```sh
just run-playground

# Send transactions with
just send-txn
```

[optional]  tips
```sh
just ui
```

## Dev Notes

### Services Access
- **MinIO UI**: http://localhost:9001 (minioadmin/minioadmin)
- **PostgreSQL**: localhost:5432 (postgres/postgres)
- **Kafka**: localhost:9092

### Debugging
```sh
# Connect to the database
psql -d postgres://postgres:postgres@localhost:5432/postgres

# Update the UI's schema
just ui-db-schema

# Start all services with Docker Compose
docker-compose up -d
```