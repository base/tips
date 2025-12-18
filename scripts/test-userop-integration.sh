#!/bin/bash
set -e

echo "==================================="
echo "TIPS UserOp Integration Test"
echo "==================================="
echo ""

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Test configuration
INGRESS_RPC="http://localhost:8080"
ENTRY_POINT="0x0000000071727De22E5E9d8BAf0edAc6f37da032"
SENDER="0x3333333333333333333333333333333333333333"

echo -e "${YELLOW}Step 1: Check services${NC}"
echo "Checking Kafka..."
if docker ps | grep -q tips-kafka; then
    echo -e "${GREEN}✓ Kafka is running${NC}"
else
    echo -e "${RED}✗ Kafka is not running${NC}"
    echo "Run: docker-compose up -d kafka"
    exit 1
fi

echo "Checking Ingress RPC..."
if curl -s -f ${INGRESS_RPC} > /dev/null 2>&1; then
    echo -e "${GREEN}✓ Ingress RPC is running${NC}"
else
    echo -e "${YELLOW}⚠ Ingress RPC is not responding at ${INGRESS_RPC}${NC}"
fi

echo ""
echo -e "${YELLOW}Step 2: Send UserOperation${NC}"

USER_OP_REQUEST=$(cat <<EOF
{
  "jsonrpc": "2.0",
  "method": "eth_sendUserOperation",
  "params": [{
    "sender": "${SENDER}",
    "nonce": "0x0",
    "callData": "0x",
    "callGasLimit": "0x186a0",
    "verificationGasLimit": "0x7a120",
    "preVerificationGas": "0x5208",
    "maxFeePerGas": "0x77359400",
    "maxPriorityFeePerGas": "0x3b9aca00",
    "signature": "0x",
    "factory": null,
    "factoryData": null,
    "paymaster": null,
    "paymasterVerificationGasLimit": null,
    "paymasterPostOpGasLimit": null,
    "paymasterData": null
  }],
  "id": 1
}
EOF
)

echo "Sending UserOperation to ${INGRESS_RPC}..."
echo "${USER_OP_REQUEST}" | jq .

RESPONSE=$(curl -s -X POST ${INGRESS_RPC} \
  -H "Content-Type: application/json" \
  -d "${USER_OP_REQUEST}")

echo ""
echo "Response:"
echo "${RESPONSE}" | jq .

if echo "${RESPONSE}" | jq -e '.result.user_operation_hash' > /dev/null 2>&1; then
    USER_OP_HASH=$(echo "${RESPONSE}" | jq -r '.result.user_operation_hash')
    echo -e "${GREEN}✓ UserOperation queued: ${USER_OP_HASH}${NC}"
else
    echo -e "${RED}✗ Failed to queue UserOperation${NC}"
    echo "${RESPONSE}" | jq .
    exit 1
fi

echo ""
echo -e "${YELLOW}Step 3: Check Kafka topic${NC}"
echo "Checking tips-user-operation topic..."

docker exec tips-kafka kafka-console-consumer \
    --bootstrap-server localhost:29092 \
    --topic tips-user-operation \
    --from-beginning \
    --max-messages 1 \
    --timeout-ms 5000 2>/dev/null | head -5 || echo "No messages yet"

echo ""
echo -e "${YELLOW}Step 4: Verification${NC}"
echo "To verify the full integration:"
echo "1. Check builder logs for UserOp consumption"
echo "2. Monitor block production for bundler transactions"
echo "3. Verify EntryPoint.handleOps() calls on-chain"

echo ""
echo -e "${GREEN}Integration test completed!${NC}"
echo ""
echo "Next steps:"
echo "  - Run the builder: cargo run -p tips-builder"
echo "  - Monitor logs: tail -f builder.log"
echo "  - Check Kafka lag: docker exec tips-kafka kafka-consumer-groups --bootstrap-server localhost:29092 --describe --group tips-builder"
