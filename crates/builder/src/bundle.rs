use account_abstraction_core::types::{UserOperationRequest, VersionedUserOperation};
use alloy_primitives::{Address, B256, Bytes, Keccak256, TxHash};
use alloy_sol_types::{SolCall, sol};
use rblib::{prelude::*, reth};
use reth::{primitives::Recovered, revm::db::BundleState};
use serde::{Deserialize, Serialize};

sol! {
    interface IEntryPointV07 {
        function handleOps(
            PackedUserOperation[] calldata ops,
            address payable beneficiary
        ) external;
    }

    struct PackedUserOperation {
        address sender;
        uint256 nonce;
        bytes initCode;
        bytes callData;
        bytes32 accountGasLimits;
        uint256 preVerificationGas;
        bytes32 gasFees;
        bytes paymasterAndData;
        bytes signature;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserOpBundle {
    pub user_ops: Vec<UserOperationRequest>,
    pub entry_point: Address,
    pub beneficiary: Address,
    pub reverting_txs: Vec<TxHash>,
    pub dropping_txs: Vec<TxHash>,
}

impl UserOpBundle {
    pub fn new(entry_point: Address, beneficiary: Address) -> Self {
        Self {
            user_ops: Vec::new(),
            entry_point,
            beneficiary,
            reverting_txs: Vec::new(),
            dropping_txs: Vec::new(),
        }
    }

    pub fn with_user_op(mut self, user_op: UserOperationRequest) -> Self {
        self.user_ops.push(user_op);
        self
    }

    pub fn build_handleops_calldata(&self) -> Option<Bytes> {
        if self.user_ops.is_empty() {
            return None;
        }

        let packed_ops: Vec<PackedUserOperation> = self
            .user_ops
            .iter()
            .filter_map(|req| match &req.user_operation {
                VersionedUserOperation::PackedUserOperation(op) => {
                    let init_code = if let Some(factory) = op.factory {
                        let mut ic = factory.to_vec();
                        ic.extend_from_slice(&op.factory_data.clone().unwrap_or_default());
                        Bytes::from(ic)
                    } else {
                        Bytes::new()
                    };

                    let paymaster_and_data = if let Some(paymaster) = op.paymaster {
                        let mut pd = paymaster.to_vec();
                        let pvgl: [u8; 16] = op
                            .paymaster_verification_gas_limit
                            .unwrap_or_default()
                            .to::<u128>()
                            .to_be_bytes();
                        let pogl: [u8; 16] = op
                            .paymaster_post_op_gas_limit
                            .unwrap_or_default()
                            .to::<u128>()
                            .to_be_bytes();
                        pd.extend_from_slice(&pvgl);
                        pd.extend_from_slice(&pogl);
                        pd.extend_from_slice(&op.paymaster_data.clone().unwrap_or_default());
                        Bytes::from(pd)
                    } else {
                        Bytes::new()
                    };

                    let vgl_bytes: [u8; 16] = op.verification_gas_limit.to::<u128>().to_be_bytes();
                    let cgl_bytes: [u8; 16] = op.call_gas_limit.to::<u128>().to_be_bytes();
                    let mut account_gas_limits = [0u8; 32];
                    account_gas_limits[..16].copy_from_slice(&vgl_bytes);
                    account_gas_limits[16..].copy_from_slice(&cgl_bytes);

                    let mpfpg_bytes: [u8; 16] =
                        op.max_priority_fee_per_gas.to::<u128>().to_be_bytes();
                    let mfpg_bytes: [u8; 16] = op.max_fee_per_gas.to::<u128>().to_be_bytes();
                    let mut gas_fees = [0u8; 32];
                    gas_fees[..16].copy_from_slice(&mpfpg_bytes);
                    gas_fees[16..].copy_from_slice(&mfpg_bytes);

                    Some(PackedUserOperation {
                        sender: op.sender,
                        nonce: op.nonce,
                        initCode: init_code,
                        callData: op.call_data.clone(),
                        accountGasLimits: account_gas_limits.into(),
                        preVerificationGas: op.pre_verification_gas,
                        gasFees: gas_fees.into(),
                        paymasterAndData: paymaster_and_data,
                        signature: op.signature.clone(),
                    })
                }
                _ => None,
            })
            .collect();

        if packed_ops.is_empty() {
            return None;
        }

        let call = IEntryPointV07::handleOpsCall {
            ops: packed_ops,
            beneficiary: self.beneficiary,
        };

        Some(call.abi_encode().into())
    }

    pub fn create_bundle_transaction(
        &self,
        bundler_address: Address,
        nonce: u64,
        chain_id: u64,
        base_fee: u128,
    ) -> Option<Recovered<types::Transaction<Optimism>>> {
        use rblib::alloy::consensus::{SignableTransaction, Signed, TxEip1559};
        use rblib::alloy::primitives::{Signature, TxKind, U256};

        let calldata = self.build_handleops_calldata()?;

        let max_fee = base_fee.saturating_mul(2);
        let max_priority_fee = 1_000_000u128;

        let tx_eip1559 = TxEip1559 {
            chain_id,
            nonce,
            gas_limit: 5_000_000,
            max_fee_per_gas: max_fee,
            max_priority_fee_per_gas: max_priority_fee,
            to: TxKind::Call(self.entry_point),
            value: U256::ZERO,
            access_list: Default::default(),
            input: calldata,
        };

        let signature = Signature::from_scalars_and_parity(B256::ZERO, B256::ZERO, false);

        let hash = tx_eip1559.signature_hash();
        let signed_tx = Signed::new_unchecked(tx_eip1559, signature, hash);
        let tx = types::Transaction::<Optimism>::Eip1559(signed_tx);

        Some(Recovered::new_unchecked(tx, bundler_address))
    }
}

impl Default for UserOpBundle {
    fn default() -> Self {
        Self::new(Address::ZERO, Address::ZERO)
    }
}

impl Bundle<Optimism> for UserOpBundle {
    type PostExecutionError = UserOpBundleError;

    fn transactions(&self) -> &[Recovered<types::Transaction<Optimism>>] {
        &[]
    }

    fn without_transaction(self, tx: TxHash) -> Self {
        Self {
            user_ops: self.user_ops,
            entry_point: self.entry_point,
            beneficiary: self.beneficiary,
            reverting_txs: self
                .reverting_txs
                .into_iter()
                .filter(|t| *t != tx)
                .collect(),
            dropping_txs: self.dropping_txs.into_iter().filter(|t| *t != tx).collect(),
        }
    }

    fn is_eligible(&self, _: &BlockContext<Optimism>, _: &()) -> Eligibility {
        Eligibility::Eligible
    }

    fn is_allowed_to_fail(&self, tx: &TxHash) -> bool {
        self.reverting_txs.contains(tx)
    }

    fn is_optional(&self, tx: &TxHash) -> bool {
        self.dropping_txs.contains(tx)
    }

    fn validate_post_execution(
        &self,
        _state: &BundleState,
        _block: &BlockContext<Optimism>,
    ) -> Result<(), Self::PostExecutionError> {
        Ok(())
    }

    fn hash(&self) -> B256 {
        let mut hasher = Keccak256::default();

        for user_op in &self.user_ops {
            if let Ok(hash) = user_op.hash() {
                hasher.update(hash);
            }
        }

        hasher.update(self.entry_point);
        hasher.update(self.beneficiary);

        for tx in &self.reverting_txs {
            hasher.update(tx);
        }

        for tx in &self.dropping_txs {
            hasher.update(tx);
        }

        hasher.finalize()
    }
}

#[derive(Debug)]
pub enum UserOpBundleError {
    InvalidUserOp,
}

impl core::fmt::Display for UserOpBundleError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidUserOp => write!(f, "Invalid UserOperation"),
        }
    }
}

impl core::error::Error for UserOpBundleError {}
