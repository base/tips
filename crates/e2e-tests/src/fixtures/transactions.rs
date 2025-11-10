use alloy_consensus::{SignableTransaction, TxEip1559};
use alloy_primitives::{Address, Bytes, U256};
use alloy_signer_local::PrivateKeySigner;
use anyhow::Result;
use op_alloy_network::TxSignerSync;
use op_alloy_network::eip2718::Encodable2718;

pub fn create_test_signer() -> PrivateKeySigner {
    // First Anvil account (for unit tests with mock provider)
    "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
        .parse()
        .expect("Valid test private key")
}

/// Create a funded signer for integration tests
/// This is the same account used in justfile that has funds in builder-playground
pub fn create_funded_signer() -> PrivateKeySigner {
    // Second Anvil account - same as in justfile (0x70997970C51812dc3A010C7d01b50e0d17dc79C8)
    "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d"
        .parse()
        .expect("Valid funded private key")
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
        chain_id: 13, // Local builder-playground chain ID
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
