use zksync_test_account::Account;
use zksync_types::{K256PrivateKey, Transaction};

use super::tester::VmTester;
use crate::{
    era_vm::tests::tester::{TxType, VmTesterBuilder},
    interface::{VmExecutionMode, VmInterface},
};

fn prepare_test(is_parallel: bool) -> (VmTester, [Transaction; 3]) {
    let bytes = [1; 32];
    let account = Account::new(K256PrivateKey::from_bytes(bytes.into()).unwrap());
    dbg!(&account);
    let mut vm_tester = VmTesterBuilder::new()
        .with_empty_in_memory_storage()
        .with_deployer()
        .with_custom_account(account)
        .build();

    if is_parallel {
        vm_tester.deploy_test_contract();
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
    let normal_execution = {
        let (mut vm, txs) = prepare_test(false);
        let vm = &mut vm.vm;
        for tx in &txs {
            vm.push_transaction_inner(tx.clone(), 0, true);
        }
        vm.execute(VmExecutionMode::Batch)
    };

    let parallel_execution = {
        let (mut vm, txs) = prepare_test(true);
        let vm = &mut vm.vm;
        for tx in txs {
            vm.push_parallel_transaction(tx, 0, true);
        }
        vm.execute_parallel(VmExecutionMode::Batch)
    };

    assert_eq!(
        normal_execution.logs.storage_logs,
        parallel_execution.logs.storage_logs
    );
    assert_eq!(normal_execution.result, parallel_execution.result);
    assert_eq!(normal_execution.refunds, parallel_execution.refunds);
}
