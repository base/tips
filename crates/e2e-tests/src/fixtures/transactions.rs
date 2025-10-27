use alloy_consensus::{SignableTransaction, TxLegacy};
use alloy_network::eip2718::Encodable2718;
use alloy_primitives::{Address, Bytes, U256};
use alloy_signer::Signer;
use alloy_signer_local::PrivateKeySigner;
use anyhow::Result;

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
    gas_price: u128,
) -> Result<Bytes> {
    let tx = TxLegacy {
        to: alloy_primitives::TxKind::Call(to),
        value,
        nonce,
        gas_limit,
        gas_price,
        input: Default::default(),
        chain_id: Some(8453),
    };

    let signature = signer.sign_hash(&tx.signature_hash()).await?;

    let envelope = tx.into_signed(signature);

    let mut buf = Vec::new();
    envelope.encode_2718(&mut buf);

    Ok(Bytes::from(buf))
}
