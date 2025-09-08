# Run all CI checks locally
ci: check test fmt clippy build

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