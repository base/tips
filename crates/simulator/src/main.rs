use anyhow::Result;
use clap::Parser;
use reth_node_builder::{NodeBuilder, NodeConfig};
use reth_node_ethereum::EthereumNode;
use tips_simulator::{init_simulator_exex, SimulatorNodeConfig};
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables
    dotenvy::dotenv().ok();

    // Parse command line arguments
    let config = SimulatorNodeConfig::parse();
    
    // Extract simulator config
    let simulator_config = config.clone().into();
    
    info!(
        database_url = %config.database_url,
        max_concurrent = config.max_concurrent_simulations,
        timeout_ms = config.simulation_timeout_ms,
        "Starting reth node with Tips Simulator ExEx"
    );

    // Create node builder with ExEx
    let handle = NodeBuilder::new(config.node.clone())
        .node(EthereumNode::default())
        .install_exex("tips-simulator", move |ctx| async move {
            // Initialize the simulator ExEx
            let exex = init_simulator_exex(ctx, simulator_config).await?;
            
            info!("Tips Simulator ExEx installed successfully");
            
            // Run the ExEx
            Ok(exex.run())
        })
        .launch()
        .await?;

    info!("Reth node with Tips Simulator ExEx started successfully");

    // Wait for the node to finish
    handle.wait_for_node_exit().await
}
