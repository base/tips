### DEVELOPMENT COMMANDS ###
ci:
    # Rust
    cargo fmt --all -- --check
    cargo clippy -- -D warnings
    cargo build
    cargo test
    # UI
    cd ui && npm run lint
    cd ui && npm run build

test-integration:
    #!/usr/bin/env bash
    set -e
    echo "Starting integration tests..."

    # Check if Kafka is running
    if ! docker ps | grep -q tips-kafka; then
        echo "Starting Kafka..."
        docker-compose up -d kafka kafka-setup
        sleep 5
    fi

    # Run builder integration tests (basic Kafka tests)
    echo "Running builder integration tests..."
    cargo test -p tips-builder --test integration_tests -- --test-threads=1

    echo "✓ All integration tests passed!"

test-e2e:
    #!/usr/bin/env bash
    set -e
    echo "Starting END-TO-END tests..."

    # Check if Kafka is running
    if ! docker ps | grep -q tips-kafka; then
        echo "Kafka not running. Starting..."
        docker-compose up -d kafka kafka-setup
        sleep 10
    fi

    # Run E2E tests (these are marked with #[ignore] so we need --ignored)
    echo "Running UserOp end-to-end tests..."
    cargo test -p tips-builder --test userop_e2e_test -- --ignored --nocapture --test-threads=1

    echo "✓ All E2E tests passed!"

test-userop-e2e:
    #!/usr/bin/env bash
    set -e
    echo "Running UserOp end-to-end test..."

    # Ensure services are running
    if ! docker ps | grep -q tips-kafka; then
        echo "Kafka not running. Start with: just start-except builder ingress-rpc"
        exit 1
    fi

    # Run the integration test script
    ./scripts/test-userop-integration.sh

fix:
    # Rust
    cargo fmt --all
    cargo clippy --fix --allow-dirty --allow-staged
    # UI
    cd ui && npx biome check --write --unsafe

sync: deps-reset
    ###   ENV    ###
    just sync-env
    ###    REFORMAT   ###
    just fix

sync-env:
    cp .env.example .env
    cp .env.example ./ui/.env
    cp .env.example .env.docker
    # Change kafka ports for builder
    sed -i '' 's/localhost:9092/host.docker.internal:9094/g' ./.env.docker
    # Change builder kafka properties file path for docker
    sed -i '' 's|TIPS_BUILDER_KAFKA_PROPERTIES_FILE=./docker/builder-kafka-properties|TIPS_BUILDER_KAFKA_PROPERTIES_FILE=/app/docker/builder-kafka-properties|g' ./.env.docker
    # Change other dependencies
    sed -i '' 's/localhost/host.docker.internal/g' ./.env.docker

stop-all:
    export COMPOSE_FILE=docker-compose.yml:docker-compose.tips.yml && docker compose down && docker compose rm && rm -rf data/

# Start every service running in docker, useful for demos
start-all: stop-all
    export COMPOSE_FILE=docker-compose.yml:docker-compose.tips.yml && mkdir -p data/kafka data/minio && docker compose build && docker compose up -d

# Start every service in docker, except the one you're currently working on. e.g. just start-except ui ingress-rpc
start-except programs: stop-all
    #!/bin/bash
    all_services=(kafka kafka-setup minio minio-setup ingress-rpc audit ui builder)
    exclude_services=({{ programs }})

    # Create result array with services not in exclude list
    result_services=()
    for service in "${all_services[@]}"; do
        skip=false
        for exclude in "${exclude_services[@]}"; do
            if [[ "$service" == "$exclude" ]]; then
                skip=true
                break
            fi
        done
        if [[ "$skip" == false ]]; then
            result_services+=("$service")
        fi
    done

    export COMPOSE_FILE=docker-compose.yml:docker-compose.tips.yml && mkdir -p data/kafka data/minio && docker compose build && docker compose up -d ${result_services[@]}

### RUN SERVICES ###
deps-reset:
    COMPOSE_FILE=docker-compose.yml:docker-compose.tips.yml docker compose down && docker compose rm && rm -rf data/ && mkdir -p data/kafka data/minio && docker compose up -d

deps:
    COMPOSE_FILE=docker-compose.yml:docker-compose.tips.yml docker compose down && docker compose rm && docker compose up -d

audit:
    cargo run --bin tips-audit

ingress-rpc:
    cargo run --bin tips-ingress-rpc

maintenance:
    cargo run --bin tips-maintenance

ingress-writer:
    cargo run --bin tips-ingress-writer

builder:
    cargo run --bin tips-builder

ui:
    cd ui && yarn dev

sequencer_url := "http://localhost:8547"
validator_url := "http://localhost:8549"
builder_url := "http://localhost:2222"
ingress_url := "http://localhost:8080"

get-blocks:
    echo "Sequencer"
    cast bn -r {{ sequencer_url }}
    echo "Validator"
    cast bn -r {{ validator_url }}
    echo "Builder"
    cast bn -r {{ builder_url }}

sender := "0x70997970C51812dc3A010C7d01b50e0d17dc79C8"
sender_key := "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d"

backrunner := "0x3C44CdDdB6a900fa2b585dd299e03d12FA4293BC"
backrunner_key := "0x5de4111afa1a4b94908f83103eb1f1706367c2e68ca870fc3fb9a804cdab365a"

send-txn:
    #!/usr/bin/env bash
    set -euxo pipefail
    echo "sending txn"
    nonce=$(cast nonce {{ sender }} -r {{ builder_url }})
    txn=$(cast mktx --private-key {{ sender_key }} 0x0000000000000000000000000000000000000000 --value 0.01ether --nonce $nonce --chain-id 13 -r {{ builder_url }})
    hash=$(curl -s {{ ingress_url }} -X POST   -H "Content-Type: application/json" --data "{\"method\":\"eth_sendRawTransaction\",\"params\":[\"$txn\"],\"id\":1,\"jsonrpc\":\"2.0\"}" | jq -r ".result")
    cast receipt $hash -r {{ sequencer_url }} | grep status
    cast receipt $hash -r {{ builder_url }} | grep status

send-userop:
    #!/usr/bin/env bash
    set -euxo pipefail
    echo "Sending UserOperation to ingress RPC..."

    USER_OP='{"sender":"0x3333333333333333333333333333333333333333","nonce":"0x0","callData":"0x","callGasLimit":"0x186a0","verificationGasLimit":"0x7a120","preVerificationGas":"0x5208","maxFeePerGas":"0x77359400","maxPriorityFeePerGas":"0x3b9aca00","signature":"0x","factory":null,"factoryData":null,"paymaster":null,"paymasterVerificationGasLimit":null,"paymasterPostOpGasLimit":null,"paymasterData":null}'

    response=$(curl -s {{ ingress_url }} -X POST \
        -H "Content-Type: application/json" \
        --data "{\"method\":\"eth_sendUserOperation\",\"params\":[$USER_OP],\"id\":1,\"jsonrpc\":\"2.0\"}")

    echo "Response: $response"

    user_op_hash=$(echo "$response" | jq -r ".result.user_operation_hash")
    if [ "$user_op_hash" != "null" ]; then
        echo "✓ UserOperation queued: $user_op_hash"
    else
        echo "✗ Failed to queue UserOperation"
        exit 1
    fi

send-txn-with-backrun:
    #!/usr/bin/env bash
    set -euxo pipefail

    # 1. Get nonce and send target transaction from sender account
    nonce=$(cast nonce {{ sender }} -r {{ builder_url }})
    echo "Sending target transaction from sender (nonce=$nonce)..."
    target_txn=$(cast mktx --private-key {{ sender_key }} \
        0x0000000000000000000000000000000000000000 \
        --value 0.01ether \
        --nonce $nonce \
        --chain-id 13 \
        -r {{ builder_url }})

    target_hash=$(curl -s {{ ingress_url }} -X POST \
        -H "Content-Type: application/json" \
        --data "{\"method\":\"eth_sendRawTransaction\",\"params\":[\"$target_txn\"],\"id\":1,\"jsonrpc\":\"2.0\"}" \
        | jq -r ".result")
    echo "Target tx sent: $target_hash"

    # 2. Build backrun transaction from backrunner account (different account!)
    backrun_nonce=$(cast nonce {{ backrunner }} -r {{ builder_url }})
    echo "Building backrun transaction from backrunner (nonce=$backrun_nonce)..."
    backrun_txn=$(cast mktx --private-key {{ backrunner_key }} \
        0x0000000000000000000000000000000000000001 \
        --value 0.001ether \
        --nonce $backrun_nonce \
        --chain-id 13 \
        -r {{ builder_url }})

    # 3. Compute tx hashes for reverting_tx_hashes
    backrun_hash_computed=$(cast keccak $backrun_txn)
    echo "Target tx hash: $target_hash"
    echo "Backrun tx hash: $backrun_hash_computed"

    # 4. Construct and send bundle with reverting_tx_hashes
    echo "Sending backrun bundle..."
    bundle_json=$(jq -n \
        --arg target "$target_txn" \
        --arg backrun "$backrun_txn" \
        --arg target_hash "$target_hash" \
        --arg backrun_hash "$backrun_hash_computed" \
        '{
            txs: [$target, $backrun],
            blockNumber: 0,
            revertingTxHashes: [$target_hash, $backrun_hash]
        }')

    bundle_hash=$(curl -s {{ ingress_url }} -X POST \
        -H "Content-Type: application/json" \
        --data "{\"method\":\"eth_sendBackrunBundle\",\"params\":[$bundle_json],\"id\":1,\"jsonrpc\":\"2.0\"}" \
        | jq -r ".result")
    echo "Bundle sent: $bundle_hash"

    # 5. Wait and verify both transactions
    echo "Waiting for transactions to land..."
    sleep 5

    echo "=== Target transaction (from sender) ==="
    cast receipt $target_hash -r {{ sequencer_url }} | grep -E "(status|blockNumber|transactionIndex)"

    echo "=== Backrun transaction (from backrunner) ==="
    cast receipt $backrun_hash_computed -r {{ sequencer_url }} | grep -E "(status|blockNumber|transactionIndex)" || echo "Backrun tx not found yet"
