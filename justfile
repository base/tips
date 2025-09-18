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