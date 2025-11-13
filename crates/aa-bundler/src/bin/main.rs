//! AA Bundler Service
//!
//! Consumes UserOperations from Kafka, converts them to EntryPoint transactions,
//! and publishes bundles to the ingress queue.

use alloy_provider::{ProviderBuilder, RootProvider};
use clap::Parser;
use op_alloy_network::Optimism;
use rdkafka::producer::FutureProducer;
use rdkafka::ClientConfig;
use tips_aa_bundler::{Config, UserOperationConsumer};
use tips_aa_bundler::converter::UserOperationConverter;
use tips_audit::{connect_audit_to_publisher, BundleEvent, KafkaBundleEventPublisher};
use tips_core::kafka::load_kafka_config_from_file;
use tips_core::logger::init_logger;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let config = Config::parse();
    init_logger(&config.log_level);

    info!(
        message = "Starting AA Bundler service",
        consumer_topic = %config.consumer_topic,
        producer_topic = %config.producer_topic,
        chain_id = config.chain_id,
    );

    // Initialize RPC providers
    let _provider: RootProvider<Optimism> = ProviderBuilder::new()
        .disable_recommended_fillers()
        .network::<Optimism>()
        .connect_http(config.rpc_url.clone());

    let _simulation_provider: RootProvider<Optimism> = ProviderBuilder::new()
        .disable_recommended_fillers()
        .network::<Optimism>()
        .connect_http(config.simulation_rpc.clone());

    // Initialize converter
    let converter = UserOperationConverter::new(&config.bundler_private_key, config.chain_id)?;

    info!(
        bundler_address = %converter.bundler_address(),
        "Bundler initialized"
    );

    // Setup Kafka consumer for UserOperations
    let consumer_client_config =
        ClientConfig::from_iter(load_kafka_config_from_file(&config.consumer_kafka_properties)?);

    let (user_op_tx, mut user_op_rx) = mpsc::unbounded_channel();
    let consumer = UserOperationConsumer::new(
        consumer_client_config,
        config.consumer_topic.clone(),
        user_op_tx,
    )?;

    // Setup Kafka producer for bundles
    let producer_client_config =
        ClientConfig::from_iter(load_kafka_config_from_file(&config.producer_kafka_properties)?);
    let _bundle_producer: FutureProducer = producer_client_config.create()?;

    // Setup Kafka producer for audit
    let audit_client_config =
        ClientConfig::from_iter(load_kafka_config_from_file(&config.audit_kafka_properties)?);
    let audit_producer: FutureProducer = audit_client_config.create()?;
    let audit_publisher =
        KafkaBundleEventPublisher::new(audit_producer, config.audit_topic.clone());
    let (_audit_tx, audit_rx) = mpsc::unbounded_channel::<BundleEvent>();
    connect_audit_to_publisher(audit_rx, audit_publisher);

    // Spawn consumer task
    let consumer_handle = tokio::spawn(async move {
        if let Err(e) = consumer.run().await {
            error!(error = %e, "Consumer task failed");
        }
    });

    // Process UserOperations
    let processor_handle = tokio::spawn(async move {
        info!("Starting UserOperation processor");

        while let Some(user_op_message) = user_op_rx.recv().await {
            info!(
                sender = %user_op_message.user_operation.sender(),
                entry_point = %user_op_message.entry_point,
                hash = %user_op_message.hash,
                "Processing UserOperation"
            );

            // Convert UserOperation to transaction
            match converter.convert_to_transaction(&user_op_message) {
                Ok(_entry_point_tx) => {
                    // TODO: Create Bundle and publish to tips-ingress
                    // let bundle = Bundle {
                    //     txs: vec![entry_point_tx],
                    //     block_number: 0, // Will be set by ingress
                    //     reverting_tx_hashes: vec![],
                    //     ..Default::default()
                    // };
                    
                    info!(
                        sender = %user_op_message.user_operation.sender(),
                        "Successfully converted UserOperation (would publish bundle)"
                    );
                }
                Err(e) => {
                    error!(
                        error = %e,
                        sender = %user_op_message.user_operation.sender(),
                        "Failed to convert UserOperation to transaction"
                    );
                }
            }
        }
    });

    // Wait for tasks
    tokio::select! {
        _ = consumer_handle => {
            error!("Consumer task ended unexpectedly");
        }
        _ = processor_handle => {
            error!("Processor task ended unexpectedly");
        }
    }

    Ok(())
}

