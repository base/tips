set positional-arguments

alias f := fix
alias c := ci

# Default to display help menu
default:
    @just --list

# Runs all ci checks
ci:
    cargo fmt --all -- --check
    cargo clippy -- -D warnings
    cargo build
    cargo test
    cd ui && npx --yes @biomejs/biome check .
    cd ui && npm run build

# Fixes formatting and clippy issues
fix:
    cargo fmt --all
    cargo clippy --fix --allow-dirty --allow-staged
    cd ui && npx --yes @biomejs/biome check --write --unsafe .

# Resets dependencies and reformats code
sync: deps-reset sync-env fix

# Copies environment templates and adapts for docker
sync-env:
    cp .env.example .env
    cp .env.example ./ui/.env
    cp .env.example .env.docker
    sed -i '' 's/localhost:9092/host.docker.internal:9094/g' ./.env.docker
    sed -i '' 's/localhost/host.docker.internal/g' ./.env.docker

# Stops and removes all docker containers and data
stop-all:
    export COMPOSE_FILE=docker-compose.yml:docker-compose.tips.yml && docker compose down && docker compose rm && rm -rf data/

# Starts all services in docker, useful for demos
start-all: stop-all
    export COMPOSE_FILE=docker-compose.yml:docker-compose.tips.yml && mkdir -p data/kafka data/minio && docker compose build && docker compose up -d

# Starts docker services except specified ones, e.g. just start-except ui ingress-rpc
start-except programs: stop-all
    #!/bin/bash
    all_services=(kafka kafka-setup minio minio-setup ingress-rpc audit ui)
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

# Resets docker dependencies with clean data
deps-reset:
    COMPOSE_FILE=docker-compose.yml:docker-compose.tips.yml docker compose down && docker compose rm && rm -rf data/ && mkdir -p data/kafka data/minio && docker compose up -d

# Restarts docker dependencies without data reset
deps:
    COMPOSE_FILE=docker-compose.yml:docker-compose.tips.yml docker compose down && docker compose rm && docker compose up -d

# Runs the tips-audit service
audit:
    cargo run --bin tips-audit

# Runs the tips-ingress-rpc service
ingress-rpc:
    cargo run --bin tips-ingress-rpc

# Runs the tips-maintenance service
maintenance:
    cargo run --bin tips-maintenance

# Runs the tips-ingress-writer service
ingress-writer:
    cargo run --bin tips-ingress-writer

# Starts the UI development server
ui:
    cd ui && yarn dev

sequencer_url := "http://localhost:8547"
validator_url := "http://localhost:8549"
builder_url := "http://localhost:2222"
ingress_url := "http://localhost:8080"

sender := "0x70997970C51812dc3A010C7d01b50e0d17dc79C8"
sender_key := "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d"

backrunner := "0x3C44CdDdB6a900fa2b585dd299e03d12FA4293BC"
backrunner_key := "0x5de4111afa1a4b94908f83103eb1f1706367c2e68ca870fc3fb9a804cdab365a"

# Queries block numbers from sequencer, validator, and builder
get-blocks:
    echo "Sequencer"
    cast bn -r {{ sequencer_url }}
    echo "Validator"
    cast bn -r {{ validator_url }}
    echo "Builder"
    cast bn -r {{ builder_url }}

# Sends a test transaction through the ingress endpoint
send-txn:
    #!/usr/bin/env bash
    set -euxo pipefail
    echo "sending txn"
    nonce=$(cast nonce {{ sender }} -r {{ builder_url }})
    txn=$(cast mktx --private-key {{ sender_key }} 0x0000000000000000000000000000000000000000 --value 0.01ether --nonce $nonce --chain-id 13 -r {{ builder_url }})
    hash=$(curl -s {{ ingress_url }} -X POST   -H "Content-Type: application/json" --data "{\"method\":\"eth_sendRawTransaction\",\"params\":[\"$txn\"],\"id\":1,\"jsonrpc\":\"2.0\"}" | jq -r ".result")
    cast receipt $hash -r {{ sequencer_url }} | grep status
    cast receipt $hash -r {{ builder_url }} | grep status

# Sends a transaction with a backrun bundle
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

# Runs integration tests with infrastructure checks
e2e:
    #!/bin/bash
    if ! INTEGRATION_TESTS=1 cargo test --package tips-system-tests --test integration_tests; then
        echo ""
        echo "═══════════════════════════════════════════════════════════════════"
        echo "  ⚠️  Integration tests failed!"
        echo "  Make sure the infrastructure is running locally (see SETUP.md for full instructions): "
        echo "      just start-all"
        echo "      start builder-playground"
        echo "      start op-rbuilder"
        echo "═══════════════════════════════════════════════════════════════════"
        exit 1
    fi
    echo "═══════════════════════════════════════════════════════════════════"
    echo "  ✅ Integration tests passed!"
    echo "═══════════════════════════════════════════════════════════════════"
