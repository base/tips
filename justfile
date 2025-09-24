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

create-migration name:
    touch crates/datastore/migrations/$(date +%s)_{{ name }}.sql

sync: deps-reset
    ### DATABASE ###
    cargo sqlx prepare -D postgresql://postgres:postgres@localhost:5432/postgres --workspace --all --no-dotenv
    cd ui && npx drizzle-kit pull --dialect=postgresql --url=postgresql://postgres:postgres@localhost:5432/postgres
    cd ui && mv ./drizzle/relations.ts ./src/db/
    cd ui && mv ./drizzle/schema.ts ./src/db/
    cd ui && rm -rf ./drizzle
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
    export COMPOSE_FILE=docker-compose.yml:docker-compose.tips.yml && mkdir -p data/postgres data/kafka data/minio && docker compose build && docker compose up -d

# Start every service in docker, except the one you're currently working on. e.g. just start-except ui ingress-rpc
start-except programs: stop-all
    #!/bin/bash
    all_services=(postgres kafka kafka-setup minio minio-setup ingress-rpc ingres-writer audit maintenance ui)
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
    
    export COMPOSE_FILE=docker-compose.yml:docker-compose.tips.yml && mkdir -p data/postgres data/kafka data/minio && docker compose build && docker compose up -d ${result_services[@]}

### RUN SERVICES ###
deps-reset:
    COMPOSE_FILE=docker-compose.yml:docker-compose.tips.yml docker compose down && docker compose rm && rm -rf data/ && mkdir -p data/postgres data/kafka data/minio && docker compose up -d

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