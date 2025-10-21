use eyre::Result;
use futures::StreamExt;
use reth::api::FullNodeComponents;
use reth_exex::{ExExContext, ExExEvent, ExExNotification};
use tracing::{debug, info};

mod validation;

pub async fn rpc_exex<Node: FullNodeComponents>(mut ctx: ExExContext<Node>) -> Result<()> {
    loop {
        tokio::select! {
            Some(notification) = ctx.notifications.next() => {
                match notification {
                    Ok(ExExNotification::ChainCommitted { new }) => {
                        info!(committed_chain = ?new.range(), "Received commit");
                        ctx.events.send(ExExEvent::FinishedHeight(new.tip().num_hash()))?;
                    }
                    Ok(ExExNotification::ChainReorged { old, new }) => {
                        info!(from_chain = ?old.range(), to_chain = ?new.range(), "Received reorg");
                        ctx.events.send(ExExEvent::FinishedHeight(new.tip().num_hash()))?;
                    }
                    Ok(ExExNotification::ChainReverted { old }) => {
                        info!(reverted_chain = ?old.range(), "Received revert");
                        ctx.events.send(ExExEvent::FinishedHeight(old.tip().num_hash()))?;
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
