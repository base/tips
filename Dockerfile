FROM rust:1-bookworm AS base

RUN apt-get update && apt-get -y upgrade && apt-get install -y libclang-dev pkg-config libsasl2-dev libssl-dev

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
    cargo build && \
    cp target/debug/tips-maintenance /tmp/tips-maintenance && \
    cp target/debug/tips-ingress-rpc /tmp/tips-ingress-rpc && \
    cp target/debug/tips-ingress-writer /tmp/tips-ingress-writer && \
    cp target/debug/tips-audit /tmp/tips-audit

FROM debian:bookworm

RUN apt-get update && apt-get install -y libssl3 ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /tmp/tips-maintenance /app/tips-maintenance
COPY --from=builder /tmp/tips-audit /app/tips-audit
COPY --from=builder /tmp/tips-ingress-rpc /app/tips-ingress-rpc
COPY --from=builder /tmp/tips-ingress-writer /app/tips-ingress-writer