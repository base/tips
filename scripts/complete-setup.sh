#!/bin/bash
set -e

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo "=========================================="
echo "TIPS Complete Setup for Builder-Playground"
echo "=========================================="
echo ""

# Step 1: Check builder-playground is running
echo "Step 1: Checking if builder-playground is running..."
if ! curl -s -f http://localhost:8547 -X POST -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","method":"eth_chainId","params":[],"id":1}' > /dev/null 2>&1; then
    echo -e "${RED}✗ Builder-playground sequencer not accessible${NC}"
    echo ""
    echo "Please start builder-playground first:"
    echo "  cd /path/to/builder-playground"
    echo "  go run main.go cook opstack --enable-latest-fork 0 --flashblocks --base-overlay"
    echo ""
    exit 1
fi
echo -e "${GREEN}✓ Builder-playground sequencer running${NC}"

# Step 2: Check Kafka
echo ""
echo "Step 2: Checking Kafka..."
if docker ps | grep -q kafka; then
    echo -e "${GREEN}✓ Kafka already running${NC}"
else
    echo -e "${YELLOW}⚠ Starting Kafka...${NC}"
    docker-compose up -d kafka kafka-setup
    echo "  Waiting for Kafka to be ready..."
    sleep 15
    echo -e "${GREEN}✓ Kafka started${NC}"
fi

# Verify Kafka is accessible
if nc -z localhost 9092 2>/dev/null; then
    echo -e "${GREEN}✓ Kafka accessible on localhost:9092${NC}"
else
    echo -e "${RED}✗ Kafka not accessible${NC}"
    exit 1
fi

# Step 3: Check topic exists
echo ""
echo "Step 3: Verifying Kafka topic..."
if docker exec tips-kafka kafka-topics --list --bootstrap-server localhost:29092 2>/dev/null | grep -q "tips-user-operation"; then
    echo -e "${GREEN}✓ Topic 'tips-user-operation' exists${NC}"
else
    echo -e "${YELLOW}⚠ Creating topic...${NC}"
    docker exec tips-kafka kafka-topics \
        --create \
        --if-not-exists \
        --topic tips-user-operation \
        --bootstrap-server localhost:29092 \
        --partitions 3 \
        --replication-factor 1 2>/dev/null
    echo -e "${GREEN}✓ Topic created${NC}"
fi

# Step 4: Build tips-builder
echo ""
echo "Step 4: Building tips-builder..."
if [ ! -f "target/release/tips-builder" ]; then
    echo "  Building in release mode..."
    cargo build --bin tips-builder --release --quiet
    echo -e "${GREEN}✓ Builder built${NC}"
else
    echo -e "${GREEN}✓ Builder already built${NC}"
fi

# Step 5: Build tips-ingress-rpc
echo ""
echo "Step 5: Building tips-ingress-rpc..."
if [ ! -f "target/release/tips-ingress-rpc" ]; then
    echo "  Building in release mode..."
    cargo build --bin tips-ingress-rpc --release --quiet
    echo -e "${GREEN}✓ Ingress-RPC built${NC}"
else
    echo -e "${GREEN}✓ Ingress-RPC already built${NC}"
fi

echo ""
echo "=========================================="
echo -e "${GREEN}✓ Setup Complete!${NC}"
echo "=========================================="
echo ""
echo "Now run these commands in separate terminals:"
echo ""
echo -e "${YELLOW}Terminal 1 - Builder:${NC}"
echo "  cd $(pwd)"
echo "  ./target/release/tips-builder node \\"
echo "    --chain=optimism \\"
echo "    --datadir=./data/builder \\"
echo "    --authrpc.addr=127.0.0.1 \\"
echo "    --authrpc.port=8551 \\"
echo "    --authrpc.jwtsecret=./jwt.hex \\"
echo "    --http \\"
echo "    --http.addr=0.0.0.0 \\"
echo "    --http.port=2222 \\"
echo "    --http.api=eth,net,web3,debug,admin \\"
echo "    --rollup.sequencer-http=http://localhost:8547 \\"
echo "    --rollup.disable-tx-pool-gossip"
echo ""
echo -e "${YELLOW}Terminal 2 - Ingress-RPC:${NC}"
echo "  cd $(pwd)"
echo "  just ingress-rpc-local"
echo ""
echo -e "${YELLOW}Terminal 3 - Test:${NC}"
echo "  cd $(pwd)"
echo "  just send-txn"
echo "  just send-userop"
echo ""
