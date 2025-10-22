use alloy_consensus::transaction::Recovered;
use alloy_primitives::{Address, Bytes};
use jsonrpsee::core::RpcResult;
use op_alloy_consensus::OpTxEnvelope;
use tokio::sync::oneshot;

#[derive(Debug)]
pub struct ValidationData {
    pub address: Address,
    pub tx: Recovered<OpTxEnvelope>,
    pub data: Bytes,
    pub response_tx: oneshot::Sender<RpcResult<()>>,
}
