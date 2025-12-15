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
    # Change kafka ports
    sed -i '' 's/localhost:9092/host.docker.internal:9094/g' ./.env.docker
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

send-txn:
    #!/usr/bin/env bash
    set -euxo pipefail
    echo "sending txn"
    nonce=$(cast nonce {{ sender }} -r {{ builder_url }})
    txn=$(cast mktx --private-key {{ sender_key }} 0x0000000000000000000000000000000000000000 --value 0.00001ether --nonce $nonce --chain-id {{ chain_id }} -r {{ builder_url }})
    hash=$(curl -s {{ ingress_url }} -X POST   -H "Content-Type: application/json" --data "{\"method\":\"eth_sendRawTransaction\",\"params\":[\"$txn\"],\"id\":1,\"jsonrpc\":\"2.0\"}" | jq -r ".result")
    cast receipt $hash -r {{ sequencer_url }} | grep status
    cast receipt $hash -r {{ builder_url }} | grep status
