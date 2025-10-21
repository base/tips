use alloy_consensus::transaction::Recovered;
use alloy_primitives::{Address, Bytes};
use op_alloy_consensus::OpTxEnvelope;

#[derive(Debug, Clone)]
pub struct ValidationData {
    pub address: Address,
    pub tx: Recovered<OpTxEnvelope>,
    pub data: Bytes,
}
