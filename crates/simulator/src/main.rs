use clap::Parser;
use reth_node_ethereum::EthereumNode;
use tips_simulator::{init_exex_event_simulator, SimulatorNodeConfig};
use tracing::info;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // Load environment variables
    dotenvy::dotenv().ok();

    // Parse command line arguments
    let config = SimulatorNodeConfig::parse();
    // Extract simulator config
    let simulator_config: tips_simulator::types::ExExSimulationConfig = (&config).into();
    
    info!(
        database_url = %config.database_url,
        max_concurrent = config.max_concurrent_simulations,
        timeout_ms = config.simulation_timeout_ms,
        "Starting reth node with ExEx event simulator"
    );

    // Launch the node with ExEx using the CLI
    config.node.run(|builder, _| async move {
        let handle = builder
            .node(EthereumNode::default())
            .install_exex("tips-simulator", move |ctx| async move {
                // Initialize the ExEx event simulator
                let consensus_simulator = init_exex_event_simulator(ctx, simulator_config).await
                    .map_err(|e| eyre::eyre!("Failed to initialize simulator: {}", e))?;
                
                info!("ExEx event simulator installed successfully");
                
                // Run the ExEx event simulator
                Ok(consensus_simulator.run())
            })
            .launch()
            .await?;
        
        info!("Reth node with ExEx event simulator started successfully");
        
        // Wait for the node to finish
        handle.wait_for_node_exit().await
    })?;
    
    Ok(())
}
