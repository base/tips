mod bundle;
mod userops;
mod userops_pipeline;

pub use bundle::UserOpBundle;
pub use userops::UserOperationOrder;
pub use userops_pipeline::InsertUserOpBundle;

use {
    alloy_primitives::address,
    rblib::{
        pool::{HostNodeInstaller, OrderPool},
        prelude::*,
        steps::OptimismPrologue,
    },
    reth_optimism_cli::Cli,
    reth_optimism_node::{OpAddOns, OpEngineApiBuilder, OpEngineValidatorBuilder, OpNode},
    reth_optimism_rpc::OpEthApiBuilder,
    std::sync::Arc,
    userops_pipeline::TransactionCollector,
};

fn build_userops_pipeline(
    pool: &OrderPool<Optimism>,
    userops_step: InsertUserOpBundle,
) -> Pipeline<Optimism> {
    let collector = Arc::new(std::sync::Mutex::new(TransactionCollector::new(
        userops_step.clone(),
    )));

    let interleaved_step = InterleavedUserOpsStep {
        pool: pool.clone(),
        collector: collector.clone(),
    };

    let pipeline = Pipeline::<Optimism>::named("userops")
        .with_prologue(OptimismPrologue)
        .with_pipeline(Loop, interleaved_step);

    pool.attach_pipeline(&pipeline);

    pipeline
}

struct InterleavedUserOpsStep {
    pool: OrderPool<Optimism>,
    collector: Arc<std::sync::Mutex<TransactionCollector>>,
}

#[allow(clippy::let_unit_value)]
impl Step<Optimism> for InterleavedUserOpsStep {
    async fn step(
        self: Arc<Self>,
        mut checkpoint: Checkpoint<Optimism>,
        ctx: StepContext<Optimism>,
    ) -> ControlFlow<Optimism> {
        let checkpoint_ctx = *checkpoint.context();
        let orders = self
            .pool
            .best_orders_for_block(ctx.block(), &checkpoint_ctx);

        let mut count = 0;
        for order in orders {
            if count >= 100 {
                break;
            }

            if let Ok(mut collector) = self.collector.lock()
                && let Some(bundler_tx) = collector.maybe_insert_userops_bundle(&checkpoint)
            {
                match checkpoint.apply(bundler_tx) {
                    Ok(cp) => checkpoint = cp,
                    Err(_) => continue,
                }
            }

            let executable = match order.try_into_executable() {
                Ok(exec) => exec,
                Err(_) => continue,
            };

            match checkpoint.apply(executable) {
                Ok(cp) => {
                    if let Ok(mut collector) = self.collector.lock()
                        && let Some(tx) = cp.transactions().last()
                    {
                        collector.collect_transaction(tx.clone());
                    }
                    checkpoint = cp;
                    count += 1;
                }
                Err(_) => continue,
            }
        }

        ControlFlow::Ok(checkpoint)
    }
}

fn main() {
    let bundler_address = address!("0x1111111111111111111111111111111111111111");
    let entry_point = address!("0x0000000071727De22E5E9d8BAf0edAc6f37da032");
    let beneficiary = address!("0x2222222222222222222222222222222222222222");

    let userops_step = InsertUserOpBundle::new(bundler_address);

    let example_bundle = UserOpBundle::new(entry_point, beneficiary);
    userops_step.add_bundle(example_bundle);

    Cli::parse_args()
        .run(|builder, _cli_args| async move {
            let pool = OrderPool::<Optimism>::default();
            let pipeline = build_userops_pipeline(&pool, userops_step);
            let op_node = OpNode::default();

            let add_ons: OpAddOns<
                _,
                OpEthApiBuilder,
                OpEngineValidatorBuilder,
                OpEngineApiBuilder<OpEngineValidatorBuilder>,
            > = op_node
                .add_ons_builder::<types::RpcTypes<Optimism>>()
                .build();

            let handle = builder
                .with_types::<OpNode>()
                .with_components(
                    op_node
                        .components()
                        .attach_pool(&pool)
                        .payload(pipeline.into_service()),
                )
                .with_add_ons(add_ons)
                .launch()
                .await?;

            handle.wait_for_node_exit().await
        })
        .unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use account_abstraction_core::types::{UserOperationRequest, VersionedUserOperation};
    use alloy_primitives::{Address, Bytes, U256, address};
    use alloy_rpc_types::erc4337::PackedUserOperation;
    use rblib::alloy::consensus::SignableTransaction;
    use rblib::reth::core::primitives::Recovered;

    fn create_test_user_op(sender: Address, nonce: u64) -> UserOperationRequest {
        UserOperationRequest {
            user_operation: VersionedUserOperation::PackedUserOperation(PackedUserOperation {
                sender,
                nonce: U256::from(nonce),
                call_data: Bytes::default(),
                call_gas_limit: U256::from(100000),
                verification_gas_limit: U256::from(500000),
                pre_verification_gas: U256::from(21000),
                max_fee_per_gas: U256::from(2000000000),
                max_priority_fee_per_gas: U256::from(1000000000),
                signature: Bytes::default(),
                factory: None,
                factory_data: None,
                paymaster: None,
                paymaster_verification_gas_limit: None,
                paymaster_post_op_gas_limit: None,
                paymaster_data: None,
            }),
            entry_point: address!("0x0000000071727De22E5E9d8BAf0edAc6f37da032"),
            chain_id: 10,
        }
    }

    #[test]
    fn test_user_op_bundle_creation() {
        let entry_point = address!("0x0000000071727De22E5E9d8BAf0edAc6f37da032");
        let beneficiary = address!("0x2222222222222222222222222222222222222222");

        let bundle = UserOpBundle::new(entry_point, beneficiary);

        assert_eq!(bundle.entry_point, entry_point);
        assert_eq!(bundle.beneficiary, beneficiary);
        assert_eq!(bundle.user_ops.len(), 0);
    }

    #[test]
    fn test_user_op_bundle_with_ops() {
        let entry_point = address!("0x0000000071727De22E5E9d8BAf0edAc6f37da032");
        let beneficiary = address!("0x2222222222222222222222222222222222222222");
        let sender = address!("0x3333333333333333333333333333333333333333");

        let user_op1 = create_test_user_op(sender, 0);
        let user_op2 = create_test_user_op(sender, 1);

        let bundle = UserOpBundle::new(entry_point, beneficiary)
            .with_user_op(user_op1)
            .with_user_op(user_op2);

        assert_eq!(bundle.user_ops.len(), 2);
    }

    #[test]
    fn test_build_handleops_calldata() {
        let entry_point = address!("0x0000000071727De22E5E9d8BAf0edAc6f37da032");
        let beneficiary = address!("0x2222222222222222222222222222222222222222");
        let sender = address!("0x3333333333333333333333333333333333333333");

        let user_op = create_test_user_op(sender, 0);

        let bundle = UserOpBundle::new(entry_point, beneficiary).with_user_op(user_op);

        let calldata = bundle.build_handleops_calldata();
        assert!(calldata.is_some());

        let calldata = calldata.unwrap();
        assert!(!calldata.is_empty());
        assert_eq!(&calldata[0..4], &[0x76, 0x5e, 0x82, 0x7f]);
    }

    #[test]
    fn test_empty_bundle_no_calldata() {
        let entry_point = address!("0x0000000071727De22E5E9d8BAf0edAc6f37da032");
        let beneficiary = address!("0x2222222222222222222222222222222222222222");

        let bundle = UserOpBundle::new(entry_point, beneficiary);

        let calldata = bundle.build_handleops_calldata();
        assert!(calldata.is_none());
    }

    #[test]
    fn test_create_bundle_transaction() {
        let entry_point = address!("0x0000000071727De22E5E9d8BAf0edAc6f37da032");
        let beneficiary = address!("0x2222222222222222222222222222222222222222");
        let bundler_address = address!("0x1111111111111111111111111111111111111111");
        let sender = address!("0x3333333333333333333333333333333333333333");

        let user_op = create_test_user_op(sender, 0);

        let bundle = UserOpBundle::new(entry_point, beneficiary).with_user_op(user_op);

        let tx = bundle.create_bundle_transaction(bundler_address, 0, 10, 1000000000);

        assert!(tx.is_some());
        let tx = tx.unwrap();
        assert_eq!(tx.signer(), bundler_address);
    }

    #[test]
    fn test_insert_userop_bundle_initialization() {
        let bundler_address = address!("0x1111111111111111111111111111111111111111");

        let step = InsertUserOpBundle::new(bundler_address);

        assert_eq!(step.bundler_address, bundler_address);
    }

    #[test]
    fn test_insert_userop_bundle_add_bundle() {
        let bundler_address = address!("0x1111111111111111111111111111111111111111");
        let entry_point = address!("0x0000000071727De22E5E9d8BAf0edAc6f37da032");
        let beneficiary = address!("0x2222222222222222222222222222222222222222");

        let step = InsertUserOpBundle::new(bundler_address);
        let bundle = UserOpBundle::new(entry_point, beneficiary);

        step.add_bundle(bundle);

        let bundles = step.userops_pool.lock().unwrap();
        assert_eq!(bundles.len(), 1);
    }

    #[test]
    fn test_transaction_collector_initialization() {
        let bundler_address = address!("0x1111111111111111111111111111111111111111");
        let userops_step = InsertUserOpBundle::new(bundler_address);

        let collector = TransactionCollector::new(userops_step);

        assert_eq!(collector.transactions.len(), 0);
        assert!(!collector.midpoint_reached);
        assert!(!collector.userops_inserted);
    }

    #[test]
    fn test_bundle_hash() {
        let entry_point = address!("0x0000000071727De22E5E9d8BAf0edAc6f37da032");
        let beneficiary = address!("0x2222222222222222222222222222222222222222");
        let sender = address!("0x3333333333333333333333333333333333333333");

        let user_op = create_test_user_op(sender, 0);

        let bundle1 = UserOpBundle::new(entry_point, beneficiary).with_user_op(user_op.clone());
        let bundle2 = UserOpBundle::new(entry_point, beneficiary).with_user_op(user_op);

        assert_eq!(bundle1.hash(), bundle2.hash());
    }

    #[test]
    fn test_nonce_increment() {
        let bundler_address = address!("0x1111111111111111111111111111111111111111");
        let step = InsertUserOpBundle::new(bundler_address);

        let nonce1 = step.get_next_nonce();
        let nonce2 = step.get_next_nonce();
        let nonce3 = step.get_next_nonce();

        assert_eq!(nonce1, 0);
        assert_eq!(nonce2, 1);
        assert_eq!(nonce3, 2);
    }

    #[test]
    fn test_bundler_tx_inserted_at_midpoint() {
        let bundler_address = address!("0x1111111111111111111111111111111111111111");
        let entry_point = address!("0x0000000071727De22E5E9d8BAf0edAc6f37da032");
        let beneficiary = address!("0x2222222222222222222222222222222222222222");
        let sender = address!("0x3333333333333333333333333333333333333333");

        let userops_step = InsertUserOpBundle::new(bundler_address);

        let user_op1 = create_test_user_op(sender, 0);
        let user_op2 = create_test_user_op(sender, 1);
        let user_op3 = create_test_user_op(sender, 2);

        let bundle = UserOpBundle::new(entry_point, beneficiary)
            .with_user_op(user_op1)
            .with_user_op(user_op2)
            .with_user_op(user_op3);

        userops_step.add_bundle(bundle);

        let mut collector = TransactionCollector::new(userops_step.clone());

        let tx1 =
            create_dummy_transaction(address!("0x1000000000000000000000000000000000000001"), 0);
        let tx2 =
            create_dummy_transaction(address!("0x1000000000000000000000000000000000000002"), 1);
        let tx3 =
            create_dummy_transaction(address!("0x1000000000000000000000000000000000000003"), 2);
        let tx4 =
            create_dummy_transaction(address!("0x1000000000000000000000000000000000000004"), 3);
        let tx5 =
            create_dummy_transaction(address!("0x1000000000000000000000000000000000000005"), 4);
        let tx6 =
            create_dummy_transaction(address!("0x1000000000000000000000000000000000000006"), 5);

        collector.collect_transaction(tx1);
        collector.collect_transaction(tx2);
        collector.collect_transaction(tx3);

        assert_eq!(collector.transactions.len(), 3);
        assert!(!collector.midpoint_reached);
        assert!(!collector.userops_inserted);

        collector.collect_transaction(tx4);
        collector.collect_transaction(tx5);
        collector.collect_transaction(tx6);

        assert_eq!(collector.transactions.len(), 6);
    }

    #[test]
    fn test_midpoint_detection() {
        let bundler_address = address!("0x1111111111111111111111111111111111111111");
        let userops_step = InsertUserOpBundle::new(bundler_address);

        let mut collector = TransactionCollector::new(userops_step.clone());

        assert!(!collector.midpoint_reached);

        for i in 0..3 {
            let tx =
                create_dummy_transaction(address!("0x1000000000000000000000000000000000000001"), i);
            collector.collect_transaction(tx);
        }

        assert_eq!(collector.transactions.len(), 3);
        assert!(!collector.midpoint_reached);
    }

    #[test]
    fn test_userops_bundle_only_inserted_once() {
        let bundler_address = address!("0x1111111111111111111111111111111111111111");
        let entry_point = address!("0x0000000071727De22E5E9d8BAf0edAc6f37da032");
        let beneficiary = address!("0x2222222222222222222222222222222222222222");

        let userops_step = InsertUserOpBundle::new(bundler_address);
        let bundle = UserOpBundle::new(entry_point, beneficiary);
        userops_step.add_bundle(bundle);

        let mut collector = TransactionCollector::new(userops_step.clone());

        collector.userops_inserted = true;

        let bundles = userops_step.userops_pool.lock().unwrap();
        assert_eq!(bundles.len(), 1);
        drop(bundles);

        assert!(collector.userops_inserted);
    }

    fn create_dummy_transaction(
        from: Address,
        nonce: u64,
    ) -> Recovered<types::Transaction<Optimism>> {
        use rblib::alloy::consensus::{Signed, TxEip1559};
        use rblib::alloy::primitives::{Signature, TxKind, U256};

        let tx_eip1559 = TxEip1559 {
            chain_id: 10,
            nonce,
            gas_limit: 21000,
            max_fee_per_gas: 2000000000,
            max_priority_fee_per_gas: 1000000000,
            to: TxKind::Call(address!("0x0000000000000000000000000000000000000000")),
            value: U256::ZERO,
            access_list: Default::default(),
            input: Bytes::default(),
        };

        let signature = Signature::from_scalars_and_parity(
            alloy_primitives::B256::ZERO,
            alloy_primitives::B256::ZERO,
            false,
        );

        let hash = tx_eip1559.signature_hash();
        let signed_tx = Signed::new_unchecked(tx_eip1559, signature, hash);
        let tx = types::Transaction::<Optimism>::Eip1559(signed_tx);

        Recovered::new_unchecked(tx, from)
    }
}
