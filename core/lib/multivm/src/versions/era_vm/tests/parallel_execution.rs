use zksync_types::Transaction;

use super::tester::VmTester;
use crate::{
    era_vm::tests::tester::{TxType, VmTesterBuilder},
    interface::{VmExecutionMode, VmInterface},
};

fn prepare_test(is_parallel: bool) -> (VmTester, [Transaction; 3]) {
    let mut vm_tester = VmTesterBuilder::new()
        .with_empty_in_memory_storage()
        .with_deployer()
        .with_random_rich_accounts(1)
        .build();

    if is_parallel {
        let mut vm_tester_2 = VmTesterBuilder::new()
            .with_empty_in_memory_storage()
            .with_deployer()
            .with_random_rich_accounts(1)
            .build();
        vm_tester_2.deploy_test_contract();

        vm_tester.test_contract = vm_tester_2.test_contract;

        vm_tester.deploy_test_contract_parallel()
    } else {
        vm_tester.deploy_test_contract();
    }

    let account = &mut vm_tester.rich_accounts[0];

    let txs = [
        account.get_test_contract_transaction(
            vm_tester.test_contract.unwrap(),
            false,
            Default::default(),
            false,
            TxType::L1 { serial_id: 1 },
        ),
        account.get_test_contract_transaction(
            vm_tester.test_contract.unwrap(),
            true,
            Default::default(),
            false,
            TxType::L1 { serial_id: 1 },
        ),
        account.get_test_contract_transaction(
            vm_tester.test_contract.unwrap(),
            false,
            Default::default(),
            false,
            TxType::L1 { serial_id: 1 },
        ),
    ];

    (vm_tester, txs)
}

#[test]
fn parallel_execution() {
    // let normal_execution = {
    //     let (mut vm, txs) = prepare_test(false);
    //     let vm = &mut vm.vm;
    //     for tx in &txs {
    //         vm.push_transaction_inner(tx.clone(), 0, true);
    //     }
    //     vm.execute(VmExecutionMode::Batch)
    // };

    let parallel_execution = {
        let (mut vm, txs) = prepare_test(true);
        let vm = &mut vm.vm;
        for tx in txs {
            vm.push_parallel_transaction(tx, 0, true);
        }
        vm.execute_parallel()
    };
    println!("RESULT {:?}", parallel_execution.result);

    // we don't assert if statistics are equal since that would require
    // sharing the gas in parallel,
    assert!(1 == 2);
    // assert_eq!(
    //     normal_execution.logs.storage_logs,
    //     parallel_execution.logs.storage_logs
    // );
    // assert_eq!(normal_execution.result, parallel_execution.result);
    // assert_eq!(normal_execution.refunds, parallel_execution.refunds);
}
