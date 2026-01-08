# Local Development Setup

This guide covers setting up and running TIPS locally with all required dependencies.

## Prerequisites

- Docker and Docker Compose
- Rust (latest stable)
- Go (1.21+)
- Just command runner (`cargo install just`)
- Git

## Clone Repositories

Clone the three required repositories:

```bash
# TIPS (this repository)
git clone https://github.com/base/tips.git

# builder-playground (separate directory)
git clone https://github.com/flashbots/builder-playground.git
cd builder-playground
git remote add danyal git@github.com:danyalprout/builder-playground.git
git checkout danyal/base-overlay

# op-rbuilder (separate directory)
git clone https://github.com/base/op-rbuilder.git
```

## Start Services

### 1. TIPS Infrastructure

```bash
cd tips
just sync      # Sync and load environment variables
just start-all # Start all TIPS services
```

This starts:
- Docker containers (Kafka, MinIO, node-reth)
- Ingress RPC service
- Audit service
- Bundle pool service
- UI

### 2. builder-playground

Provides L1/L2 blockchain infrastructure:

```bash
cd builder-playground
go run main.go cook opstack \
  --external-builder http://host.docker.internal:4444/ \
  --enable-latest-fork 0 \
  --flashblocks \
  --base-overlay \
  --flashblocks-builder ws://host.docker.internal:1111/ws
```

Keep this terminal running.

### 3. op-rbuilder

Handles L2 block building:

```bash
cd op-rbuilder
just run-playground
```

Keep this terminal running.

## Test the System

```bash
cd tips
just send-txn
```

This submits a test transaction through the ingress → audit → bundle pool → builder pipeline.

## Port Reference

| Service | Port | Description |
|---------|------|-------------|
| TIPS Ingress RPC | 8080 | Bundle submission endpoint |
| TIPS UI | 3000 | Debug interface |
| MinIO Console | 7001 | Object storage UI |
| MinIO API | 7000 | Object storage API |
| Kafka | 9092 | Message broker |
| op-rbuilder | 4444 | Block builder API |

## Development Workflow

1. Keep builder-playground and op-rbuilder running
2. Run `just start-all` after code changes
3. Test with `just send-txn`
4. View logs with `docker logs -f <service-name>`
5. Access UI at http://localhost:3000

Query block information:

```bash
just get-blocks
```

## Stop Services

```bash
cd tips
just stop-all

# Ctrl+C in op-rbuilder terminal
# Ctrl+C in builder-playground terminal
```
