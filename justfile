### DEVELOPMENT COMMANDS ###
ci:
    cargo fmt --all -- --check
    cargo clippy -- -D warnings
    cargo build
    cargo test

fix:
    cargo fmt --all
    cargo clippy --fix --allow-dirty --allow-staged

create-migration name:
    touch crates/datastore/migrations/$(date +%s)_{{ name }}.sql

sync:
    ### DATABASE ###
    cargo sqlx prepare -D postgresql://postgres:postgres@localhost:5432/postgres --workspace --all --no-dotenv
    ###   ENV    ###
    cp .env.example .env

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