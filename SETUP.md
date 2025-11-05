# TIPS Local Development Setup

This guide walks you through setting up and running TIPS locally with all required dependencies.

## Prerequisites

- Docker and Docker Compose
- Rust (latest stable)
- Go (1.21+)
- Just command runner (`cargo install just`)
- Git

## Step 1: Clone Required Repositories

Clone the three repositories you'll need:

```bash
# Clone TIPS (this repository)
git clone https://github.com/base/tips.git

# Clone builder-playground in a separate directory
git clone https://github.com/flashbots/builder-playground.git
cd builder-playground
git remote add danyal git@github.com:danyalprout/builder-playground.git # TODO: change this once it's upstreamed
git checkout danyal/base-overlay

# Clone op-rbuilder in a separate directory
git clone https://github.com/base/op-rbuilder.git
cd op-rbuilder
git checkout tips-prototype
```

## Step 2: Start TIPS Infrastructure

```bash
cd tips

# Sync (and load env vars) and start all TIPS services
just sync
just start-all
```

This will:
- Reset and start Docker containers (Kafka, MinIO, node-reth services)
- Start the TIPS ingress RPC service
- Start the audit service
- Start the bundle pool service
- Start the UI

## Step 3: Start builder-playground

The builder-playground provides the L1/L2 blockchain infrastructure.

```bash
cd builder-playground

# Start the playground
go run main.go cook opstack --external-builder http://host.docker.internal:4444/ --enable-latest-fork 0 --flashblocks --base-overlay --flashblocks-builder ws://host.docker.internal:1111/ws
```

Keep this terminal running. The playground will:
- Start L1 and L2 nodes
- Provide blockchain infrastructure for TIPS
- Expose services on various ports

## Step 4: Start op-rbuilder

The op-rbuilder handles block building for the L2.

```bash
cd op-rbuilder

# Start the builder (ensure you're on tips-prototype branch)
just run-playground
```

Keep this terminal running. The builder will:
- Connect to the builder-playground infrastructure
- Handle block building requests
- Expose builder API on port 4444

## Step 5: Access the UI and send a test transaction

Once everything is running, you can test the system:

```bash
cd tips

# Send a test transaction
just send-txn
```

This will:
- Submit a transaction bundle to TIPS
- Process it through the ingress → audit → bundle pool pipeline
- Send it to the builder for inclusion in blocks

## Ports Reference

| Service | Port | Description |
|---------|------|-------------|
| TIPS Ingress RPC | 8080 | Main RPC endpoint for bundle submission |
| TIPS UI | 3000 | Web interface |
| MinIO Console | 7001 | Object storage UI |
| MinIO API | 7000 | Object storage API |
| Kafka | 9092 | Message broker |
| op-rbuilder | 4444 | Block builder API |
| builder-playground | Various | L1/L2 blockchain infrastructure |

If you want to get information regarding the sequencer, validator, and builder, you can run:

```bash
just get-blocks
```

## Development Workflow

For active development:

1. Keep builder-playground and op-rbuilder running
2. Use `just start-all` to restart TIPS services after code changes
3. Use `just send-txn` to test transaction flow
4. Monitor logs with `docker logs -f <service-name>`
5. Access TIPS UI at http://localhost:3000 for debugging

## Stopping Services

To stop everything:

```bash
# Stop TIPS services
cd tips
just stop-all

# Stop op-rbuilder (Ctrl+C in terminal)

# Stop builder-playground (Ctrl+C in terminal)
```
