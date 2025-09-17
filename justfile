# Run all CI checks locally
ci: rust-ci ui-ci

# Run Rust CI checks
rust-ci: check test fmt clippy build

# Run UI CI checks
ui-ci: ui-format ui-lint-check ui-typecheck ui-build

# Run UI formatting
ui-format:
    cd ui && npm run format

# Run UI linting with fixes
ui-lint:
    cd ui && npx biome check --write

# Run UI linting (check only)
ui-lint-check:
    cd ui && npm run lint

# Run UI TypeScript type checking
ui-typecheck:
    cd ui && npx tsc --noEmit

# Build UI for production
ui-build:
    cd ui && npm run build

db:
    #!/usr/bin/env bash
    set -euxo pipefail
    docker container stop tips-db
    docker container rm tips-db
    docker run -d --name tips-db -e POSTGRES_PASSWORD=postgres -p 5432:5432 postgres
    sleep 2
    for file in ./crates/datastore/migrations/*.sql; do
      echo $file
      psql -d postgres://postgres:postgres@localhost:5432/postgres -f $file
    done


create-migration name:
    touch crates/datastore/migrations/$(date +%s)_{{ name }}.sql

# Pull database schema using drizzle-kit
ui-db-schema:
    cd ui && npx drizzle-kit pull --dialect=postgresql --url=postgresql://postgres:postgres@localhost:5432/postgres
    cd ui && mv ./drizzle/relations.ts ./src/db/
    cd ui && mv ./drizzle/schema.ts ./src/db/
    cd ui && rm -rf ./drizzle

# Check code compilation
check:
    cargo check

# Run tests
test:
    cargo test

# Check formatting
fmt:
    cargo fmt --all -- --check

# Run clippy lints
clippy:
    cargo clippy -- -D warnings

# Build release binary
build:
    cargo build

# Run the ingress service with default mempool URL
ingress:
    cargo run --bin tips-ingress

ui:
    cd ui && yarn dev

# Run autofixes everything
fix: fix-rust fix-ui

# Fix Rust code issues
fix-rust: fmt-fix clippy-fix

# Fix UI code issues
fix-ui:
    cd ui && npx biome check --fix

# Format code (fix)
fmt-fix:
    cargo fmt --all

# Run clippy with fixes
clippy-fix:
    cargo clippy --fix --allow-dirty --allow-staged