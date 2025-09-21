use clap::Parser;
use reth_node_ethereum::EthereumNode;
use tips_simulator::{
    ListenersWithWorkers,
    SimulatorNodeConfig,
    MempoolListenerConfig
};
use tracing::info;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    dotenvy::dotenv().ok();

    let config = SimulatorNodeConfig::parse();
    let exex_config: tips_simulator::types::ExExSimulationConfig = (&config).into();
    let mempool_config: MempoolListenerConfig = (&config).into();
    
    info!(
        database_url = %config.database_url,
        max_concurrent = config.max_concurrent_simulations,
        timeout_ms = config.simulation_timeout_ms,
        kafka_brokers = %config.kafka_brokers,
        kafka_topic = %config.kafka_topic,
        "Starting reth node with both ExEx and mempool event listeners"
    );

    config.node.run(|builder, _| async move {
        let handle = builder
            .node(EthereumNode::default())
            .install_exex("tips-simulator", move |ctx| async move {
                let listeners = ListenersWithWorkers::new(
                    ctx, 
                    exex_config, 
                    mempool_config,
                    config.max_concurrent_simulations,
                    config.simulation_timeout_ms
                ).await
                .map_err(|e| eyre::eyre!("Failed to initialize listeners: {}", e))?;
                
                info!("Both ExEx and mempool event listeners initialized successfully");
                
                Ok(listeners.run())
            })
            .launch()
            .await?;
        
        info!("Reth node with both listeners started successfully");
        
        handle.wait_for_node_exit().await
    })?;
    
    Ok(())
}
