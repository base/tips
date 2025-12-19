#!/bin/bash

echo "Transaction: 0x9557315dc3fb7b05ea8185b980f27634bc2e1e43ad068232db5b9940f96af022"
echo "Block: 178"
echo ""
echo "Checking validator (what UI shows)..."
sleep 2

# Check if validator has caught up to block 178
VAL_BLOCK=$(cast block-number --rpc-url http://localhost:8549 2>/dev/null || echo "0")
echo "Validator latest block: $VAL_BLOCK"

if [ "$VAL_BLOCK" -ge 178 ]; then
    echo "✓ Validator has synced! Your transaction should be visible in UI"
    echo "  Go to: http://localhost:3000/block/178"
else
    echo "⏳ Validator is still syncing (at block $VAL_BLOCK, needs 178)"
    echo "   Wait a few seconds and refresh the UI"
fi
