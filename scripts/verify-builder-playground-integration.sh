#!/bin/bash
set -e

echo "=================================================="
echo "TIPS + Builder-Playground Integration Verification"
echo "=================================================="
echo ""

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Check 1: Docker image exists
echo "1. Checking tips-builder Docker image..."
if docker images | grep -q "tips-builder.*latest"; then
    echo -e "${GREEN}✓ tips-builder:latest image found${NC}"
else
    echo -e "${RED}✗ tips-builder:latest image not found${NC}"
    echo "  Run: docker build -t tips-builder:latest ."
    exit 1
fi

# Check 2: Kafka running
echo ""
echo "2. Checking Kafka..."
if docker ps | grep -q kafka; then
    KAFKA_CONTAINER=$(docker ps | grep kafka | awk '{print $1}')
    echo -e "${GREEN}✓ Kafka container running: ${KAFKA_CONTAINER}${NC}"

    # Check topic
    echo "   Checking tips-user-operation topic..."
    if docker exec ${KAFKA_CONTAINER} kafka-topics --list --bootstrap-server localhost:29092 2>/dev/null | grep -q "tips-user-operation"; then
        echo -e "${GREEN}   ✓ Topic 'tips-user-operation' exists${NC}"
    else
        echo -e "${YELLOW}   ⚠ Topic 'tips-user-operation' not found${NC}"
        echo "   Creating topic..."
        docker exec ${KAFKA_CONTAINER} kafka-topics \
            --create \
            --if-not-exists \
            --topic tips-user-operation \
            --bootstrap-server localhost:29092 \
            --partitions 3 \
            --replication-factor 1 2>/dev/null
        echo -e "${GREEN}   ✓ Topic created${NC}"
    fi
else
    echo -e "${YELLOW}⚠ Kafka not running (expected if using builder-playground)${NC}"
fi

# Check 3: Ingress-RPC
echo ""
echo "3. Checking ingress-rpc..."
if curl -s -f http://localhost:8080 -X POST -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","method":"eth_chainId","params":[],"id":1}' > /dev/null 2>&1; then
    echo -e "${GREEN}✓ Ingress-RPC accessible on port 8080${NC}"
else
    echo -e "${YELLOW}⚠ Ingress-RPC not accessible${NC}"
    echo "  Run: just ingress-rpc"
fi

# Check 4: Builder
echo ""
echo "4. Checking tips-builder..."
if docker ps | grep -q tips-builder; then
    BUILDER_CONTAINER=$(docker ps | grep tips-builder | awk '{print $1}')
    echo -e "${GREEN}✓ tips-builder container running: ${BUILDER_CONTAINER}${NC}"

    # Check environment
    echo "   Checking Kafka configuration..."
    KAFKA_BROKERS=$(docker inspect ${BUILDER_CONTAINER} | grep -o 'TIPS_BUILDER_KAFKA_BROKERS=[^"]*' | head -1 || echo "")
    if [ -n "$KAFKA_BROKERS" ]; then
        echo -e "${GREEN}   ✓ ${KAFKA_BROKERS}${NC}"
    else
        echo -e "${YELLOW}   ⚠ KAFKA_BROKERS not set${NC}"
    fi
else
    echo -e "${YELLOW}⚠ tips-builder not running${NC}"
    echo "  Should be started by builder-playground"
fi

# Check 5: Port availability
echo ""
echo "5. Checking ports..."

check_port() {
    local port=$1
    local service=$2
    if lsof -Pi :${port} -sTCP:LISTEN -t >/dev/null 2>&1; then
        echo -e "${GREEN}   ✓ Port ${port} in use (${service})${NC}"
    else
        echo -e "${YELLOW}   ⚠ Port ${port} not in use (${service})${NC}"
    fi
}

check_port 8080 "ingress-rpc"
check_port 9092 "Kafka (local)"
check_port 2222 "builder"
check_port 8547 "sequencer"
check_port 8549 "validator"

# Check 6: Test connectivity
echo ""
echo "6. Testing connectivity..."

if [ -n "$KAFKA_CONTAINER" ]; then
    echo "   Testing Kafka producer..."
    echo '{"test":"message"}' | docker exec -i ${KAFKA_CONTAINER} kafka-console-producer \
        --broker-list localhost:29092 \
        --topic tips-user-operation 2>/dev/null && \
        echo -e "${GREEN}   ✓ Can produce to Kafka${NC}" || \
        echo -e "${RED}   ✗ Cannot produce to Kafka${NC}"
fi

echo ""
echo "=================================================="
echo "Verification Complete"
echo "=================================================="
echo ""
echo "Next steps:"
echo "  1. Ensure builder-playground is running with: builder-playground cook opstack --external-builder tips-builder"
echo "  2. Start ingress-rpc: just ingress-rpc"
echo "  3. Test regular tx: just send-txn"
echo "  4. Test UserOp: just send-userop"
echo ""
echo "For detailed troubleshooting, see: BUILDER_PLAYGROUND_INTEGRATION.md"
