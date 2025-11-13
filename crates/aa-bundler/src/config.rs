//! Configuration for AA Bundler Service

use clap::Parser;
use url::Url;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Config {
    /// Kafka properties file for consuming user operations
    #[arg(long, env = "TIPS_AA_BUNDLER_KAFKA_CONSUMER_PROPERTIES_FILE")]
    pub consumer_kafka_properties: String,

    /// Kafka topic to consume user operations from
    #[arg(
        long,
        env = "TIPS_AA_BUNDLER_KAFKA_CONSUMER_TOPIC",
        default_value = "tips-user-operations"
    )]
    pub consumer_topic: String,

    /// Kafka properties file for publishing bundles
    #[arg(long, env = "TIPS_AA_BUNDLER_KAFKA_PRODUCER_PROPERTIES_FILE")]
    pub producer_kafka_properties: String,

    /// Kafka topic to publish bundles to
    #[arg(
        long,
        env = "TIPS_AA_BUNDLER_KAFKA_PRODUCER_TOPIC",
        default_value = "tips-ingress"
    )]
    pub producer_topic: String,

    /// Kafka properties file for audit events
    #[arg(long, env = "TIPS_AA_BUNDLER_KAFKA_AUDIT_PROPERTIES_FILE")]
    pub audit_kafka_properties: String,

    /// Kafka topic for audit events
    #[arg(
        long,
        env = "TIPS_AA_BUNDLER_KAFKA_AUDIT_TOPIC",
        default_value = "tips-audit"
    )]
    pub audit_topic: String,

    /// Bundler private key for signing EntryPoint transactions
    #[arg(long, env = "TIPS_AA_BUNDLER_PRIVATE_KEY")]
    pub bundler_private_key: String,

    /// Chain ID for transaction signing
    #[arg(long, env = "TIPS_AA_BUNDLER_CHAIN_ID", default_value = "8453")]
    pub chain_id: u64,

    /// RPC URL for simulation and gas estimation
    #[arg(long, env = "TIPS_AA_BUNDLER_RPC_URL")]
    pub rpc_url: Url,

    /// URL of the simulation RPC service for bundle metering
    #[arg(long, env = "TIPS_AA_BUNDLER_SIMULATION_RPC")]
    pub simulation_rpc: Url,

    #[arg(long, env = "TIPS_AA_BUNDLER_LOG_LEVEL", default_value = "info")]
    pub log_level: String,
}

