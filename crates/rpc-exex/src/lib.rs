use alloy_consensus::constants::KECCAK_EMPTY;
use alloy_consensus::private::alloy_eips::{BlockId, BlockNumberOrTag};
use alloy_consensus::transaction::Recovered;
use alloy_primitives::Address;
use eyre::Result;
use futures::StreamExt;
use op_alloy_consensus::OpTxEnvelope;
use op_revm::l1block::L1BlockInfo;
use reth::api::FullNodeComponents;
use reth::providers::AccountReader;
use reth::providers::BlockReaderIdExt;
use reth::providers::TransactionVariant;
use reth_exex::{ExExContext, ExExEvent, ExExNotification};
use reth_node_api::Block;
use reth_node_api::BlockBody;
use reth_optimism_evm::extract_l1_info_from_tx;
use reth_primitives::RecoveredBlock;
use tips_common::ValidationData;
use tokio::sync::mpsc;
use tracing::{debug, info};

mod validation;

pub struct RpcExEx<Node>
where
    Node: FullNodeComponents,
{
    ctx: ExExContext<Node>,
    tx_receiver: mpsc::UnboundedReceiver<ValidationData>,
}

impl<Node> RpcExEx<Node>
where
    Node: FullNodeComponents,
{
    pub fn new(
        ctx: ExExContext<Node>,
        tx_receiver: mpsc::UnboundedReceiver<ValidationData>,
    ) -> Self {
        Self { ctx, tx_receiver }
    }

    pub async fn run(mut self) -> Result<()> {
        info!(target = "tips-rpc-exex", "Starting RPC ExEx service");

        loop {
            tokio::select! {
                Some(notification) = self.ctx.notifications.next() => {
                    match notification {
                        Ok(ExExNotification::ChainCommitted { new }) => {
                            debug!(committed_chain = ?new.range(), "Received commit");
                            self.ctx.events.send(ExExEvent::FinishedHeight(new.tip().num_hash()))?;
                        }
                        Ok(ExExNotification::ChainReorged { old, new }) => {
                            debug!(from_chain = ?old.range(), to_chain = ?new.range(), "Received reorg");
                            self.ctx.events.send(ExExEvent::FinishedHeight(new.tip().num_hash()))?;
                        }
                        Ok(ExExNotification::ChainReverted { old }) => {
                            debug!(reverted_chain = ?old.range(), "Received revert");
                            self.ctx.events.send(ExExEvent::FinishedHeight(old.tip().num_hash()))?;
                        }
                        Err(e) => {
                            debug!(target = "tips-rpc-exex", error = %e, "Error receiving notification");
                            return Err(e);
                        }
                    }
                }
                Some(validation_data) = self.tx_receiver.recv() => {
                    info!(target = "tips-rpc-exex", "Received transaction data for validation");

                    let block = self.ctx
                        .provider()
                        .block_with_senders_by_id(BlockId::Number(BlockNumberOrTag::Latest), TransactionVariant::WithHash)?
                        .ok_or_else(|| eyre::eyre!("latest block not found"))?;
                    self.validate_tx(&block, validation_data.address, &validation_data.tx, &validation_data.data).await?
                }
            }
        }
    }

    async fn validate_tx<B>(
        &mut self,
        block: &RecoveredBlock<B>,
        address: Address,
        tx: &Recovered<OpTxEnvelope>,
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
        // TODO: this errors on empty blocks, need to figure out how to handle this
        let l1_info = extract_l1_info_from_tx(
            block
                .body()
                .transactions()
                .first()
                .ok_or_else(|| eyre::eyre!("block contains no transactions"))?,
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
