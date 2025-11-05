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
git clone https://github.com/your-org/tips.git

# Clone builder-playground in a separate directory
git clone https://github.com/flashbots/builder-playground.git

# Clone op-rbuilder in a separate directory
git clone https://github.com/base/op-rbuilder.git
cd op-rbuilder
git checkout tips-prototype
```

## Step 2: Start builder-playground

The builder-playground provides the L1/L2 blockchain infrastructure.

```bash
cd builder-playground

# Start the playground
go run main.go cook opstack --external-builder http://host.docker.internal:4444 --enable-latest-fork 0
```

Keep this terminal running. The playground will:
- Start L1 and L2 nodes
- Provide blockchain infrastructure for TIPS
- Expose services on various ports

## Step 3: Start op-rbuilder

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

## Step 4: Start TIPS Infrastructure

Now start the TIPS services.

```bash
cd tips

# Sync and start all TIPS services
just sync
just start-all
```

This will:
- Reset and start Docker containers (Kafka, MinIO, node-reth services)
- Start the TIPS ingress RPC service
- Start the audit service
- Start the bundle pool service
- Start the UI

## Step 5: Verify Everything is Running

Check that all services are healthy:

```bash
# Check Docker containers
docker ps

# Check TIPS services are responding
curl http://localhost:8080/health  # Ingress RPC
curl http://localhost:7000         # MinIO console
curl http://localhost:3000         # TIPS UI
```

You should see:
- Kafka, MinIO, and node-reth containers running
- TIPS services responding to health checks
- Builder-playground blockchain running
- op-rbuilder connected and building blocks

## Step 6: Send Test Transactions

Once everything is running, you can test the system:

```bash
cd tips

# Send a test transaction through TIPS
just send-txn
```

This will:
- Submit a transaction bundle to TIPS
- Process it through the ingress → audit → bundle pool pipeline
- Send it to the builder for inclusion in blocks

## Service Architecture

When everything is running, you'll have:

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│ builder-        │    │ op-rbuilder     │    │ TIPS            │
│ playground      │◄──►│ (tips-prototype)│◄──►│ (this repo)     │
│                 │    │                 │    │                 │
│ - L1/L2 nodes   │    │ - Block builder │    │ - Ingress RPC   │
│ - Infrastructure│    │ - Port 4444     │    │ - Audit service │
└─────────────────┘    └─────────────────┘    │ - Bundle pool   │
                                              │ - UI (port 3000)│
                                              └─────────────────┘
```

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

## Troubleshooting

### Builder-playground not starting
- Ensure Go 1.21+ is installed
- Check that no other services are using the same ports
- Try `go mod tidy` in the builder-playground directory

### op-rbuilder connection issues
- Verify you're on the `tips-prototype` branch
- Ensure builder-playground is fully started before starting op-rbuilder
- Check that port 4444 is not in use by other services

### TIPS services failing
- Run `just sync` to reset Docker containers
- Check Docker daemon is running
- Verify all required ports are available

### Transaction submission failing
- Ensure all three components (playground, builder, TIPS) are running
- Check service health endpoints
- Review logs: `docker logs <container-name>`

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