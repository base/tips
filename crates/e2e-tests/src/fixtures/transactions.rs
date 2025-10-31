use alloy_consensus::{SignableTransaction, TxEip1559};
use alloy_primitives::{Address, Bytes, U256};
use alloy_signer_local::PrivateKeySigner;
use anyhow::Result;
use op_alloy_network::TxSignerSync;
use op_alloy_network::eip2718::Encodable2718;

pub fn create_test_signer() -> PrivateKeySigner {
    "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
        .parse()
        .expect("Valid test private key")
}

pub async fn create_signed_transaction(
    signer: &PrivateKeySigner,
    to: Address,
    value: U256,
    nonce: u64,
    gas_limit: u64,
    max_fee_per_gas: u128,
) -> Result<Bytes> {
    let mut tx = TxEip1559 {
        chain_id: 8453, // Base chain ID
        nonce,
        gas_limit,
        max_fee_per_gas,
        max_priority_fee_per_gas: max_fee_per_gas / 10, // 10% of max fee as priority fee
        to: to.into(),
        value,
        access_list: Default::default(),
        input: Default::default(),
    };

    let signature = signer.sign_transaction_sync(&mut tx)?;

    let envelope = op_alloy_consensus::OpTxEnvelope::Eip1559(tx.into_signed(signature));

    let mut buf = Vec::new();
    envelope.encode_2718(&mut buf);

    Ok(Bytes::from(buf))
}
