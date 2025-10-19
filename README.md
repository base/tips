![Base](./docs/logo.png)

# TIPS - Transaction Inclusion & Prioritization Stack

> [!WARNING]
> This repository is an experimental project for enabling bundles, transaction simulation, and transaction tracing on Base.
> It is currently **not production ready** and should be used for exploration and experimentation only.

## Overview

TIPS (Transaction Inclusion & Prioritization Stack) is a modular system designed to handle transaction bundles, provide simulation capabilities, and enable comprehensive transaction tracing for the Base network. The stack consists of multiple interconnected components that work together to process, store, audit, and serve transaction data.

## Architecture

### 🗄️ Datastore (`crates/datastore`)
Postgres-based storage layer providing APIs for persisting and retrieving transaction bundles.

### 📊 Audit (`crates/audit`)
Event streaming and archival system that:
- Publishes bundle events to Kafka for real-time processing
- Archives bundle history to S3 for long-term storage
- See [S3 Storage Format](docs/AUDIT_S3_FORMAT.md) for detailed data structure information

### 🔌 Ingress RPC (`crates/ingress-rpc`)
JSON-RPC server that accepts bundle submissions and simulation requests from external clients.

### ⚙️ Bundler (`crates/bundler`)
Core processing engine that validates, simulates, and manages bundle lifecycle states. See [Bundle States](docs/BUNDLE_STATES.md) for state transition details.

### 🚀 Egress (`crates/egress`)
Component responsible for submitting finalized bundles to the network.

## Installation

### Prerequisites
- Rust (latest stable version)
- Docker and Docker Compose
- PostgreSQL 14+
- Access to Kafka (for audit functionality)
- Access to S3-compatible storage (for archival)

### Setup

1. Clone the repository:
```bash
git clone https://github.com/base/tips.git
cd tips
```

2. Copy the environment template:
```bash
cp .env.example .env
```

3. Configure your `.env` file with appropriate values for:
   - Database connection strings
   - Kafka endpoints
   - S3 credentials
   - RPC endpoints

## Running

### Using Docker Compose

```bash
# Start all services
docker-compose up -d

# Or use the TIPS-specific configuration
docker-compose -f docker-compose.tips.yml up -d
```

### Using Just (recommended for development)

```bash
# View available commands
just --list

# Run the full stack
just run
```

### Manual Setup

```bash
# Build the project
cargo build --release

# Run migrations
cargo run --bin migrate

# Start the services
cargo run --bin ingress-rpc
```

## Testing

```bash
# Run all tests
cargo test

# Run tests for a specific crate
cargo test -p datastore

# Run integration tests
cargo test --test '*'
```

## API Documentation

For detailed API specifications and usage examples, see:
- [API Documentation](docs/API.md) - Complete RPC method reference
- [Bundle States](docs/BUNDLE_STATES.md) - Bundle lifecycle and state transitions
- [S3 Storage Format](docs/AUDIT_S3_FORMAT.md) - Audit data structure and archival format

## Usage Recommendations

- **Development Only**: This software is experimental and should not be used in production environments
- **Testing**: Always test bundle submissions in a staging environment first
- **Monitoring**: Enable comprehensive logging and monitoring when running experimental deployments
- **Data Retention**: Configure appropriate S3 lifecycle policies for audit data

## Current Limitations

- Not production-hardened or audited for security
- API interfaces may change without notice
- Limited error recovery mechanisms
- Performance characteristics not fully optimized
- Documentation is work-in-progress

## Project Structure

```
├── crates/           # Rust workspace crates
│   ├── audit/        # Event streaming and archival
│   ├── bundler/      # Bundle processing engine
│   ├── datastore/    # PostgreSQL storage layer
│   ├── egress/       # Network submission
│   └── ingress-rpc/  # RPC server
├── docs/             # Documentation
├── docker/           # Docker configurations
├── ui/               # User interface (if applicable)
└── .sqlx/            # SQLx metadata
```

## Contributing

This is an experimental project. Contributions, ideas, and feedback are welcome as we explore and refine the concepts.

## License

See LICENSE file for details.
