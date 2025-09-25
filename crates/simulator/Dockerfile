FROM rust:1-bookworm AS base

# Add dependencies for building reth-mdbx-sys
RUN apt-get update && apt-get install -y \
    clang \
    libclang-dev \
    llvm-dev \
    pkg-config && \
    rm -rf /var/lib/apt/lists/*

RUN cargo install cargo-chef --locked
WORKDIR /app

FROM base AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM base AS builder
COPY --from=planner /app/recipe.json recipe.json

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo chef cook --recipe-path recipe.json

COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build --bin tips-simulator && \
    cp target/debug/tips-simulator /tmp/tips-simulator

FROM debian:bookworm

RUN apt-get update && apt-get install -y libssl3 ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /tmp/tips-simulator /app/tips-simulator

CMD ["/app/tips-simulator", "node", "--datadir", "/data", "--http.addr", "0.0.0.0", "--builder.playground", "/playground"]

