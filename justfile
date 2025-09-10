# Run all CI checks locally
ci: check test fmt clippy build

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
run:
    cargo run -- --mempool-url http://localhost:2222

# Run autofixes everything
fix: fmt-fix clippy-fix

# Format code (fix)
fmt-fix:
    cargo fmt --all

# Run clippy with fixes
clippy-fix:
    cargo clippy --fix --allow-dirty --allow-staged