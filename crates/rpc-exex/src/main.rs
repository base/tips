use clap::Parser;
use eyre::Result;
use futures::StreamExt;
use reth::api::FullNodeComponents;
use reth::builder::Node;
use reth::providers::providers::BlockchainProvider;
use reth_exex::{ExExContext, ExExEvent, ExExNotification};
use reth_optimism_cli::{Cli, chainspec::OpChainSpecParser};
use reth_optimism_node::OpNode;
use reth_optimism_node::args::RollupArgs;
use tracing::{debug, info};

mod validation;

pub struct RpcExEx<Node>
where
    Node: FullNodeComponents,
{
    ctx: ExExContext<Node>,
}

impl<Node> RpcExEx<Node>
where
    Node: FullNodeComponents,
{
    pub fn new(ctx: ExExContext<Node>) -> Self {
        Self { ctx }
    }

    pub async fn run(mut self) -> Result<()> {
        info!(target = "tips-rpc-exex", "Starting RPC EXEX service");

        loop {
            tokio::select! {
                Some(notification) = self.ctx.notifications.next() => {
                    match notification {
                        Ok(ExExNotification::ChainCommitted { new }) => {
                            info!(committed_chain = ?new.range(), "Received commit");
                            self.ctx.events.send(ExExEvent::FinishedHeight(new.tip().num_hash()))?;
                        }
                        Ok(ExExNotification::ChainReorged { old, new }) => {
                            info!(from_chain = ?old.range(), to_chain = ?new.range(), "Received reorg");
                            self.ctx.events.send(ExExEvent::FinishedHeight(new.tip().num_hash()))?;
                        }
                        Ok(ExExNotification::ChainReverted { old }) => {
                            info!(reverted_chain = ?old.range(), "Received revert");
                            self.ctx.events.send(ExExEvent::FinishedHeight(old.tip().num_hash()))?;
                        }
                        Err(e) => {
                            debug!(target = "tips-rpc-exex", error = %e, "Error receiving notification");
                            return Err(e);
                        }
                    }
                }
            }
        }
    }
}

fn main() -> Result<()> {
    let rollup_args = RollupArgs {
        disable_txpool_gossip: true,
        ..Default::default()
    };
    Cli::<OpChainSpecParser, ()>::parse().run(|builder, _| async move {
        let handler = builder
            .node(OpNode::new(rollup_args))
            .install_exex("tips-rpc-exex", move |ctx| async move {
                Ok(RpcExEx::new(ctx).run())
            })
            .launch()
            .await?;

        handler.wait_for_node_exit().await?;
        Ok(())
    })?;

    Ok(())
}
