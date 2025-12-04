// src/user_operation.rs

use alloy_primitives::{Address, B256, Bytes, FixedBytes, U256, keccak256};
use alloy_rpc_types::erc4337;
use alloy_sol_types::{SolValue, sol};

sol! {
    struct PackedUserOperationStruct {
        bytes32 userOpTypeHash;
        address sender;
        uint256 nonce;
        bytes32 initCodeHash;
        bytes32 callDataHash;
        bytes32 accountGasLimits;
        uint256 preVerificationGas;
        bytes32 gasFees;
        bytes32 paymasterAndDataHash;
    }
}
pub const USEROP_TYPEHASH: &str = "PackedUserOperation(address sender,uint256 nonce,bytes initCode,bytes callData,bytes32 accountGasLimits,uint256 preVerificationGas,bytes32 gasFees,bytes paymasterAndData)";
pub const EIP712_DOMAIN_TYPEHASH: &str =
    "EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)";

// From EntryPoint.sol
const DOMAIN_NAME: &str = "ERC4337";
const DOMAIN_VERSION: &str = "1";

pub fn build_init_code_bytes(user_op: &erc4337::PackedUserOperation) -> Bytes {
    match user_op.factory {
        Some(factory) => {
            let factory_data = user_op
                .factory_data
                .as_ref()
                .map(|b| b.as_ref())
                .unwrap_or(&[]);
            let mut buf = Vec::with_capacity(20 + factory_data.len());
            buf.extend_from_slice(factory.as_slice()); // 20‑byte address
            buf.extend_from_slice(factory_data);
            Bytes::from(buf)
        }
        None => Bytes::new(), // no deployment => empty initCode
    }
}

fn pack_account_gas_limits(verification_gas: U256, call_gas: U256) -> FixedBytes<32> {
    let mask = (U256::from(1u64) << 128) - U256::from(1u64);
    let v = verification_gas & mask;
    let c = call_gas & mask;
    FixedBytes::from((v << 128) | c)
}

pub fn calc_gas_fees(max_priority_fee_per_gas: U256, max_fee_per_gas: U256) -> FixedBytes<32> {
    // keep only low 128 bits of each
    let mask = (U256::from(1u64) << 128) - U256::from(1u64);
    let hi = max_priority_fee_per_gas & mask;
    let lo = max_fee_per_gas & mask;

    // hi in high 128 bits, lo in low 128 bits
    FixedBytes::from((hi << 128) | lo)
}

pub fn encode_packed_user_operation(
    user_op: &erc4337::PackedUserOperation,
    overrideInitCodeHash: B256,
) -> Vec<u8> {
    let initCodeHash = if overrideInitCodeHash == B256::ZERO {
        keccak256(&build_init_code_bytes(user_op))
    } else {
        overrideInitCodeHash
    };
    let packed_user_operation: PackedUserOperationStruct = PackedUserOperationStruct {
        userOpTypeHash: keccak256(USEROP_TYPEHASH.as_bytes()),
        sender: user_op.sender,
        nonce: user_op.nonce,
        initCodeHash: initCodeHash,
        callDataHash: keccak256(&user_op.call_data),
        accountGasLimits: pack_account_gas_limits(
            user_op.verification_gas_limit,
            user_op.call_gas_limit,
        ),
        preVerificationGas: user_op.pre_verification_gas,
        gasFees: calc_gas_fees(user_op.max_priority_fee_per_gas, user_op.max_fee_per_gas),
        paymasterAndDataHash: keccak256(&user_op.paymaster_data.as_ref().unwrap_or(&Bytes::new())),
    };
    return packed_user_operation.abi_encode();
}

pub fn encode_user_operation(user_op: &erc4337::UserOperation) -> Vec<u8> {
    let packed_user_operation: PackedUserOperationStruct = PackedUserOperationStruct {
        userOpTypeHash: keccak256(USEROP_TYPEHASH.as_bytes()),
        sender: user_op.sender,
        nonce: user_op.nonce,
        initCodeHash: keccak256(&user_op.init_code),
        callDataHash: keccak256(&user_op.call_data),
        accountGasLimits: pack_account_gas_limits(
            user_op.verification_gas_limit,
            user_op.call_gas_limit,
        ),
        preVerificationGas: user_op.pre_verification_gas,
        gasFees: calc_gas_fees(user_op.max_priority_fee_per_gas, user_op.max_fee_per_gas),
        paymasterAndDataHash: keccak256(&user_op.paymaster_and_data),
    };
    return packed_user_operation.abi_encode();
}

pub fn compute_domain_separator(chain_id: i32, entry_point: Address) -> B256 {
    let name_hash = keccak256(DOMAIN_NAME.as_bytes());
    let version_hash = keccak256(DOMAIN_VERSION.as_bytes());

    // keccak256(abi.encode(
    //   EIP712_DOMAIN_TYPEHASH,
    //   keccak256("ERC4337"),
    //   keccak256("1"),
    //   chainId,
    //   entryPointAddress
    // ))
    sol! {
        struct DomainSeparatorData {
            bytes32 typeHash;
            bytes32 nameHash;
            bytes32 versionHash;
            uint256 chainId;
            address verifyingContract;
        }
    }

    let data = DomainSeparatorData {
        typeHash: keccak256(EIP712_DOMAIN_TYPEHASH.as_bytes()),
        nameHash: name_hash,
        versionHash: version_hash,
        chainId: U256::from(chain_id),
        verifyingContract: entry_point,
    };

    keccak256(data.abi_encode())
}

pub fn to_typed_data_hash(domain_separator: B256, struct_hash: B256) -> B256 {
    // Create the 66-byte buffer: 0x19 0x01 + 32 bytes + 32 bytes
    let mut enc = [0u8; 66]; // Create a 66-byte buffer
    // Set magic constants
    enc[0] = 0x19;
    enc[1] = 0x01;

    // Copy domain separator
    enc[2..34].copy_from_slice(domain_separator.as_slice());

    // Copy struct hash
    enc[34..66].copy_from_slice(struct_hash.as_slice());

    // keccak256 hash
    keccak256(enc)
}
