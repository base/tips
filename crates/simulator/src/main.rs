use reth_optimism_cli::commands::Commands;
use reth_optimism_node::args::RollupArgs;
use tips_simulator::{config::Cli, config::CliExt, ListenersWithWorkers};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn init_tracing() {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer())
        .init();
}

fn main() -> eyre::Result<()> {
    dotenvy::dotenv().ok();

    init_tracing();

    let cli = <Cli as CliExt>::parsed();
    let config = match &cli.command {
        Commands::Node(node) => node.ext.clone(),
        _ => eyre::bail!("tips-simulator must be run with the node command"),
    };
    let playground_enabled = config.has_playground();
    let (cli, exex_config, mempool_config, chain_block_time) = config.into_parts(cli);
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

    if let Commands::Node(node_command) = &cli.command {
        match node_command.rpc.auth_jwtsecret.as_ref() {
            Some(secret_path) => info!(
                auth_jwtsecret = %secret_path.display(),
                "Auth RPC JWT secret configured"
            ),
            None => info!("Auth RPC JWT secret not configured"),
        }
    }

    cli.run(|builder, _| async move {
        // Keep the Base mempool private.
        let rollup_args = RollupArgs {
            disable_txpool_gossip: true,
            ..Default::default()
        };

        let handle = builder
            .node(reth_optimism_node::OpNode::new(rollup_args))
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
