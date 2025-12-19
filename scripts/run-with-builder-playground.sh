#!/bin/bash
set -e

echo "=========================================="
echo "TIPS Ingress-RPC for Builder-Playground"
echo "=========================================="
echo ""

# Step 1: Check prerequisites
echo "Step 1: Checking prerequisites..."

if ! command -v docker &> /dev/null; then
    echo "❌ Docker not found. Please install Docker."
    exit 1
fi
echo "✓ Docker found"

if ! docker images | grep -q "tips-builder.*latest"; then
    echo "⚠ tips-builder:latest image not found"
    echo "  Building Docker image..."
    docker build -t tips-builder:latest .
    echo "✓ Docker image built"
else
    echo "✓ tips-builder:latest image exists"
fi

# Step 2: Stop conflicting services
echo ""
echo "Step 2: Stopping any conflicting TIPS Docker services..."
docker-compose down 2>/dev/null || true
echo "✓ Conflicting services stopped"

# Step 3: Check if Kafka is accessible
echo ""
echo "Step 3: Waiting for Kafka to be accessible..."
max_retries=30
retry_count=0

while [ $retry_count -lt $max_retries ]; do
    if nc -z localhost 9092 2>/dev/null; then
        echo "✓ Kafka is accessible on localhost:9092"
        break
    fi

    retry_count=$((retry_count + 1))

    if [ $retry_count -eq 1 ]; then
        echo "⚠ Kafka not accessible on localhost:9092"
        echo "  Make sure builder-playground is running with:"
        echo "  go run main.go cook opstack --external-builder http://host.docker.internal:2222 --enable-latest-fork 0 --flashblocks --base-overlay --flashblocks-builder ws://host.docker.internal:1111/ws"
        echo ""
        echo "  Waiting for Kafka... (${retry_count}/${max_retries})"
    else
        echo "  Still waiting... (${retry_count}/${max_retries})"
    fi

    sleep 2
done

if [ $retry_count -eq $max_retries ]; then
    echo "❌ Kafka not accessible after ${max_retries} retries"
    echo "   Please start builder-playground first"
    exit 1
fi

# Step 4: Check if builder is accessible
echo ""
echo "Step 4: Checking if builder is accessible..."
if curl -s -f http://localhost:2222 -X POST -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","method":"eth_chainId","params":[],"id":1}' > /dev/null 2>&1; then
    echo "✓ Builder accessible on port 2222"
else
    echo "⚠ Builder not accessible on port 2222"
    echo "  This is OK if builder-playground hasn't fully started yet"
fi

# Step 5: Check if sequencer is accessible
echo ""
echo "Step 5: Checking if sequencer is accessible..."
if curl -s -f http://localhost:8547 -X POST -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","method":"eth_chainId","params":[],"id":1}' > /dev/null 2>&1; then
    echo "✓ Sequencer accessible on port 8547"
else
    echo "⚠ Sequencer not accessible on port 8547"
    echo "  Please wait for builder-playground to fully start"
fi

# Step 6: Start ingress-rpc
echo ""
echo "Step 6: Starting ingress-rpc..."
echo "=========================================="
echo ""

# Load environment and run
set -a
source .env
set +a

export TIPS_INGRESS_KAFKA_USEROP_PROPERTIES_FILE=./docker/ingress-userop-kafka-properties.local
export TIPS_INGRESS_KAFKA_INGRESS_PROPERTIES_FILE=./docker/ingress-bundles-kafka-properties.local
export TIPS_INGRESS_KAFKA_AUDIT_PROPERTIES_FILE=./docker/ingress-audit-kafka-properties.local

echo "Starting tips-ingress-rpc..."
echo "  Kafka: localhost:9092"
echo "  Listening: 0.0.0.0:8080"
echo "  Builder RPC: http://localhost:2222"
echo "  Simulation RPC: http://localhost:8549"
echo ""

cargo run --bin tips-ingress-rpc
