use tips_simulator::{config::CliExt, config::SimulatorNodeConfig, ListenersWithWorkers};
use tracing::info;

fn main() -> eyre::Result<()> {
    dotenvy::dotenv().ok();

    let config = SimulatorNodeConfig::parsed();
    let playground_enabled = config.playground.is_some();
    let (cli, exex_config, mempool_config, chain_block_time) = config.into_parts();
    let max_concurrent_simulations = exex_config.max_concurrent_simulations;

    info!(
        database_url = %exex_config.database_url,
        max_concurrent = exex_config.max_concurrent_simulations,
        chain_block_time_ms = chain_block_time,
        kafka_brokers = %mempool_config.kafka_brokers.join(","),
        kafka_topic = %mempool_config.kafka_topic,
        playground = playground_enabled,
        "Starting reth node with both ExEx and mempool event listeners"
    );

    cli.run(|builder, _| async move {
        let handle = builder
            .node(reth_optimism_node::OpNode::default())
            .install_exex("tips-simulator", move |ctx| async move {
                let listeners = ListenersWithWorkers::new(
                    ctx,
                    exex_config,
                    mempool_config,
                    max_concurrent_simulations,
                    chain_block_time,
                )
                .await
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
