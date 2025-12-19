#!/bin/bash
set -e

# Load environment
set -a
source .env
set +a

# Override Kafka properties files to use local versions
export TIPS_INGRESS_KAFKA_USEROP_PROPERTIES_FILE=./docker/ingress-userop-kafka-properties.local
export TIPS_INGRESS_KAFKA_INGRESS_PROPERTIES_FILE=./docker/ingress-bundles-kafka-properties.local
export TIPS_INGRESS_KAFKA_AUDIT_PROPERTIES_FILE=./docker/ingress-audit-kafka-properties.local

# Use different metrics ports to avoid conflicts
export TIPS_INGRESS_METRICS_ADDR=0.0.0.0:9012
export TIPS_INGRESS_HEALTH_CHECK_ADDR=0.0.0.0:8091

# Run ingress-rpc
exec cargo run --release --bin tips-ingress-rpc
