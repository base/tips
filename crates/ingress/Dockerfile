FROM rust:1-bookworm AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/
COPY .sqlx/ ./.sqlx/

RUN cargo build --release --bin tips-ingress

FROM debian:bookworm

RUN apt-get update && apt-get install -y libssl3 && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/tips-ingress /app/tips-ingress

EXPOSE 3000

CMD ["/app/tips-ingress"]