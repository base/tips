#!/bin/bash
set -e

# Load environment
set -a
source .env
set +a

# Override Kafka to use localhost instead of Docker host
export TIPS_BUILDER_KAFKA_BROKERS=localhost:9092
export TIPS_BUILDER_KAFKA_PROPERTIES_FILE=./docker/builder-kafka-properties.local

# Unset problematic env vars
unset OTEL_EXPORTER_OTLP_PROTOCOL

# Run builder with correct configuration
exec cargo run --release --bin tips-builder -- node \
    --chain=optimism \
    --datadir=./data/builder \
    --authrpc.addr=0.0.0.0 \
    --authrpc.port=8561 \
    --authrpc.jwtsecret=./jwt.hex \
    --http \
    --http.addr=0.0.0.0 \
    --http.port=2222 \
    --http.api=eth,net,web3,debug,admin \
    --rollup.sequencer-http=http://localhost:8547 \
    --rollup.disable-tx-pool-gossip \
    --disable-discovery \
    --dev
