use alloy_consensus::constants::KECCAK_EMPTY;
use alloy_consensus::transaction::Recovered;
use alloy_primitives::Address;
use clap::Parser;
use eyre::Result;
use futures::StreamExt;
use op_alloy_rpc_types::Transaction;
use op_revm::l1block::L1BlockInfo;
use reth::api::FullNodeComponents;
use reth::providers::AccountReader;
use reth_exex::{ExExContext, ExExEvent, ExExNotification};
use reth_node_api::Block;
use reth_node_api::BlockBody;
use reth_optimism_cli::{Cli, chainspec::OpChainSpecParser};
use reth_optimism_evm::extract_l1_info_from_tx;
use reth_optimism_node::OpNode;
use reth_optimism_node::args::RollupArgs;
use reth_primitives::RecoveredBlock;
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

    pub async fn validate_tx<B>(
        &mut self,
        block: &RecoveredBlock<B>,
        address: Address,
        tx: &Recovered<Transaction>,
        data: &[u8],
    ) -> Result<()>
    where
        B: Block,
    {
        let mut l1_info = self.fetch_l1_block_info(block)?;
        let account = self.fetch_account_info(address)?;
        validation::validate_tx(account, tx, data, &mut l1_info).await?;
        Ok(())
    }

    fn fetch_l1_block_info<B>(&mut self, block: &RecoveredBlock<B>) -> Result<L1BlockInfo>
    where
        B: Block,
    {
        let l1_info = extract_l1_info_from_tx(
            block
                .body()
                .transactions()
                .first()
                .expect("block contains no transactions"),
        )?;
        Ok(l1_info)
    }

    fn fetch_account_info(&mut self, address: Address) -> Result<validation::AccountInfo> {
        let account = self
            .ctx
            .provider()
            .basic_account(&address)?
            .expect("account not found");
        Ok(validation::AccountInfo {
            balance: account.balance,
            nonce: account.nonce,
            code_hash: account.bytecode_hash.unwrap_or(KECCAK_EMPTY),
        })
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
