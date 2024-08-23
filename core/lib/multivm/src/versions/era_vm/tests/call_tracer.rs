use std::sync::Arc;

use once_cell::sync::OnceCell;
use zksync_types::{Address, Execute};

use crate::{
    era_vm::{
        tests::{
            tester::VmTesterBuilder,
            utils::{read_max_depth_contract, read_test_contract},
        },
        tracers::dispatcher::TracerDispatcher,
    },
    interface::{TxExecutionMode, VmExecutionMode, VmInterface},
    tracers::CallTracer,
    vm_latest::constants::BATCH_COMPUTATIONAL_GAS_LIMIT,
};

// This test is ultra slow, so it's ignored by default.
#[test]
#[ignore]
fn test_max_depth() {
    let contarct = read_max_depth_contract();
    let address = Address::random();
    let mut vm = VmTesterBuilder::new()
        .with_empty_in_memory_storage()
        .with_random_rich_accounts(1)
        .with_deployer()
        .with_bootloader_gas_limit(BATCH_COMPUTATIONAL_GAS_LIMIT)
        .with_execution_mode(TxExecutionMode::VerifyExecute)
        .with_custom_contracts(vec![(contarct, address, true)])
        .build();

    let account = &mut vm.rich_accounts[0];
    let tx = account.get_l2_tx_for_execute(
        Execute {
            contract_address: address,
            calldata: vec![],
            value: Default::default(),
            factory_deps: vec![],
        },
        None,
    );

    let result = Arc::new(OnceCell::new());
    let call_tracer = Box::new(CallTracer::new(result.clone()));
    vm.vm.push_transaction(tx);
    let res = vm.vm.inspect(
        TracerDispatcher::new(vec![call_tracer]),
        VmExecutionMode::OneTx,
    );
    assert!(result.get().is_some());
    assert!(res.result.is_failed());
}

#[test]
fn test_basic_behavior() {
    let contarct = read_test_contract();
    let address = Address::random();
    let mut vm = VmTesterBuilder::new()
        .with_empty_in_memory_storage()
        .with_random_rich_accounts(1)
        .with_deployer()
        .with_bootloader_gas_limit(BATCH_COMPUTATIONAL_GAS_LIMIT)
        .with_execution_mode(TxExecutionMode::VerifyExecute)
        .with_custom_contracts(vec![(contarct, address, true)])
        .build();

    let increment_by_6_calldata =
        "7cf5dab00000000000000000000000000000000000000000000000000000000000000006";

    let account = &mut vm.rich_accounts[0];
    let tx = account.get_l2_tx_for_execute(
        Execute {
            contract_address: address,
            calldata: hex::decode(increment_by_6_calldata).unwrap(),
            value: Default::default(),
            factory_deps: vec![],
        },
        None,
    );

    let result = Arc::new(OnceCell::new());
    let call_tracer = Box::new(CallTracer::new(result.clone()));
    vm.vm.push_transaction(tx);
    let res = vm.vm.inspect(
        TracerDispatcher::new(vec![call_tracer]),
        VmExecutionMode::OneTx,
    );

    let call_tracer_result = result.get().unwrap();

    assert_eq!(call_tracer_result.len(), 1);
    // Expect that there are a plenty of subcalls underneath.
    let subcall = &call_tracer_result[0].calls;
    assert!(subcall.len() > 10);
    assert!(!res.result.is_failed());
}
