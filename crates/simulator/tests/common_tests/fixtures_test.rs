use crate::common::fixtures::*;

#[test]
fn test_fixture_addresses() {
    assert_ne!(*addresses::ALICE, *addresses::BOB);
    assert_ne!(*addresses::CONTRACT_A, *addresses::CONTRACT_B);
}

#[test]
fn test_fixture_bundles() {
    let single = bundles::single_tx_bundle();
    assert_eq!(single.txs.len(), 1);

    let multi = bundles::multi_tx_bundle();
    assert_eq!(multi.txs.len(), 3);

    let large = bundles::large_bundle(100);
    assert_eq!(large.txs.len(), 100);
}

#[test]
fn test_fixture_scenarios() {
    let request = scenarios::basic_simulation();
    assert_eq!(request.block_number, blocks::BLOCK_18M);
    assert_eq!(request.bundle.txs.len(), 1);

    let interaction = scenarios::contract_interaction();
    assert_eq!(interaction.bundle.txs.len(), 3);

    let large_scenario = scenarios::large_bundle_scenario();
    assert_eq!(large_scenario.bundle.txs.len(), 100);
}
