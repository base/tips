use clap::Parser;
use reth_node_ethereum::EthereumNode;
use tips_simulator::{
    init_shared_event_simulators, 
    run_simulators_with_shared_workers, 
    SimulatorNodeConfig,
    MempoolSimulatorConfig
};
use tracing::info;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    dotenvy::dotenv().ok();

    let config = SimulatorNodeConfig::parse();
    let exex_config: tips_simulator::types::ExExSimulationConfig = (&config).into();
    let mempool_config: MempoolSimulatorConfig = (&config).into();
    
    info!(
        database_url = %config.database_url,
        max_concurrent = config.max_concurrent_simulations,
        timeout_ms = config.simulation_timeout_ms,
        kafka_brokers = %config.kafka_brokers,
        kafka_topic = %config.kafka_topic,
        "Starting reth node with both ExEx and mempool event simulators"
    );

    config.node.run(|builder, _| async move {
        let handle = builder
            .node(EthereumNode::default())
            .install_exex("tips-simulator", move |ctx| async move {
                let (worker_pool, exex_simulator, mempool_simulator) = 
                    init_shared_event_simulators(
                        ctx, 
                        exex_config, 
                        mempool_config,
                        config.max_concurrent_simulations,
                        config.simulation_timeout_ms
                    ).await
                    .map_err(|e| eyre::eyre!("Failed to initialize simulators: {}", e))?;
                
                info!("Both ExEx and mempool event simulators initialized successfully");
                
                Ok(run_simulators_with_shared_workers(worker_pool, exex_simulator, mempool_simulator))
            })
            .launch()
            .await?;
        
        info!("Reth node with both simulators started successfully");
        
        handle.wait_for_node_exit().await
    })?;
    
    Ok(())
}
