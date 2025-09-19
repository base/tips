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
    cd ui && npx biome check --fix

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
    cp .env.example .env
    cp .env.example ./ui/.env

### RUN SERVICES ###
deps-reset:
    docker compose down && docker compose rm && rm -rf data/ && mkdir -p data/postgres data/kafka data/minio && docker compose up -d

deps:
    docker compose down && docker compose rm && docker compose up -d

audit:
    cargo run --bin tips-audit

ingress:
    cargo run --bin tips-ingress

maintenance:
    cargo run --bin tips-maintenance

ui:
    cd ui && yarn dev