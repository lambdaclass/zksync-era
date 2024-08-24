use std::{cell::RefCell, collections::HashMap, rc::Rc};

use era_vm::{rollbacks::Rollbackable, store::StorageKey as EraStorageKey, EraVM, Execution};
use itertools::Itertools;
use zksync_state::{ReadStorage, StoragePtr};
use zksync_types::{
    event::extract_l2tol1logs_from_l1_messenger,
    l1::is_l1_tx_type,
    l2_to_l1_log::UserL2ToL1Log,
    utils::key_for_eth_balance,
    writes::{
        compression::compress_with_best_strategy, StateDiffRecord, BYTES_PER_DERIVED_KEY,
        BYTES_PER_ENUMERATION_INDEX,
    },
    AccountTreeId, StorageKey, StorageLog, StorageLogKind, StorageLogWithPreviousValue,
    Transaction, BOOTLOADER_ADDRESS, H160, KNOWN_CODES_STORAGE_ADDRESS, L1_MESSENGER_ADDRESS,
    L2_BASE_TOKEN_ADDRESS, U256,
};
use zksync_utils::{
    bytecode::{hash_bytecode, CompressedBytecodeInfo},
    h256_to_u256, u256_to_h256,
};

use super::{
    bootloader_state::BootloaderState,
    event::merge_events,
    initial_bootloader_memory::bootloader_initial_memory,
    logs::IntoSystemLog,
    snapshot::VmSnapshot,
    tracers::{
        dispatcher::TracerDispatcher, manager::VmTracerManager, refunds_tracer::RefundsTracer,
        traits::VmTracer,
    },
};
use crate::{
    era_vm::{bytecode::compress_bytecodes, transaction_data::TransactionData},
    interface::{tracer::TracerExecutionStatus, VmFactory, VmInterface, VmInterfaceHistoryEnabled},
    vm_latest::{
        constants::{
            get_vm_hook_position, get_vm_hook_start_position_latest, VM_HOOK_PARAMS_COUNT,
        },
        BootloaderMemory, CurrentExecutionState, ExecutionResult, L1BatchEnv, L2BlockEnv,
        SystemEnv, VmExecutionLogs, VmExecutionMode, VmExecutionResultAndLogs,
        VmExecutionStatistics,
    },
};

pub struct Vm<S: ReadStorage> {
    pub(crate) inner: EraVM,
    pub suspended_at: u16,
    pub gas_for_account_validation: u32,

    pub bootloader_state: BootloaderState,
    pub(crate) storage: StoragePtr<S>,

    // TODO: Maybe not necessary, check
    pub(crate) program_cache: Rc<RefCell<HashMap<U256, Vec<U256>>>>,

    // these two are only needed for tests so far
    pub(crate) batch_env: L1BatchEnv,
    pub(crate) system_env: SystemEnv,

    pub snapshot: Option<VmSnapshot>,
}

/// Encapsulates creating VM instance based on the provided environment.
impl<S: ReadStorage + 'static> VmFactory<S> for Vm<S> {
    /// Creates a new VM instance.
    fn new(batch_env: L1BatchEnv, system_env: SystemEnv, storage: StoragePtr<S>) -> Self {
        let bootloader_code = system_env
            .base_system_smart_contracts
            .bootloader
            .code
            .clone();
        let vm_hook_position =
            get_vm_hook_position(crate::vm_latest::MultiVMSubversion::IncreasedBootloaderMemory)
                * 32;
        let vm_execution = Execution::new(
            bootloader_code.to_owned(),
            Vec::new(),
            BOOTLOADER_ADDRESS,
            H160::zero(),
            0_u128,
            system_env
                .base_system_smart_contracts
                .default_aa
                .hash
                .to_fixed_bytes(),
            system_env
                .base_system_smart_contracts
                .default_aa //TODO: Add real evm interpreter
                .hash
                .to_fixed_bytes(),
            vm_hook_position,
            true,
            system_env.bootloader_gas_limit,
        );
        let pre_contract_storage = Rc::new(RefCell::new(HashMap::new()));
        pre_contract_storage.borrow_mut().insert(
            h256_to_u256(system_env.base_system_smart_contracts.default_aa.hash),
            system_env
                .base_system_smart_contracts
                .default_aa
                .code
                .clone(),
        );
        let world_storage = World::new(storage.clone(), pre_contract_storage.clone());
        let mut vm = EraVM::new(vm_execution, Rc::new(RefCell::new(world_storage)));
        let bootloader_memory = bootloader_initial_memory(&batch_env);

        // The bootloader shouldn't pay for growing memory and it writes results
        // to the end of its heap, so it makes sense to preallocate it in its entirety.
        const BOOTLOADER_MAX_MEMORY_SIZE: u32 = 59000000;
        vm.execution
            .heaps
            .get_mut(era_vm::execution::FIRST_HEAP)
            .unwrap()
            .expand_memory(BOOTLOADER_MAX_MEMORY_SIZE);
        vm.execution
            .heaps
            .get_mut(era_vm::execution::FIRST_HEAP + 1)
            .unwrap()
            .expand_memory(BOOTLOADER_MAX_MEMORY_SIZE);

        let mut mv = Self {
            inner: vm,
            suspended_at: 0,
            gas_for_account_validation: system_env.default_validation_computational_gas_limit,
            bootloader_state: BootloaderState::new(
                system_env.execution_mode.clone(),
                bootloader_initial_memory(&batch_env),
                batch_env.first_l2_block,
            ),
            program_cache: pre_contract_storage,
            storage,
            batch_env,
            system_env,
            snapshot: None,
        };

        mv.write_to_bootloader_heap(bootloader_memory);
        mv
    }
}

impl<S: ReadStorage + 'static> Vm<S> {
    pub fn run(&mut self, tracer: &mut impl VmTracer<S>) {
        tracer.before_bootloader_execution(self);
        loop {
            let output = self.inner.run_program_with_custom_bytecode(Some(tracer));

            let status = tracer.after_vm_run(self, output);

            if let TracerExecutionStatus::Stop(_) = status {
                break;
            }
        }
        tracer.after_bootloader_execution(self);
    }

    pub(crate) fn insert_bytecodes<'a>(&mut self, bytecodes: impl IntoIterator<Item = &'a [u8]>) {
        for code in bytecodes {
            let mut program_code = vec![];
            for raw_opcode_slice in code.chunks(32) {
                let mut raw_opcode_bytes: [u8; 32] = [0; 32];
                raw_opcode_bytes.copy_from_slice(&raw_opcode_slice[..32]);
                let raw_opcode_u256 = U256::from_big_endian(&raw_opcode_bytes);
                program_code.push(raw_opcode_u256);
            }
            self.program_cache.borrow_mut().insert(
                U256::from_big_endian(hash_bytecode(code).as_bytes()),
                program_code,
            );
        }
    }

    pub fn get_hook_params(&self) -> [U256; 3] {
        let vm_hooks_param_start = get_vm_hook_start_position_latest();
        (vm_hooks_param_start..vm_hooks_param_start + VM_HOOK_PARAMS_COUNT)
            .map(|word| {
                let res = self.read_heap_word(word as usize);
                res
            })
            .collect::<Vec<_>>()
            .try_into()
            .unwrap()
    }

    /// Typically used to read the bootloader heap. We know that we're in the bootloader
    /// when a hook occurs, as they are only enabled when preprocessing bootloader code.
    pub fn read_heap_word(&self, word: usize) -> U256 {
        let heap = self
            .inner
            .execution
            .heaps
            .get(self.inner.execution.current_context().unwrap().heap_id)
            .unwrap();
        heap.read((word * 32) as u32)
    }

    pub fn write_to_bootloader_heap(&mut self, memory: impl IntoIterator<Item = (usize, U256)>) {
        assert!(self.inner.execution.running_contexts.len() == 1); // No on-going far calls
        if let Some(heap) = &mut self
            .inner
            .execution
            .heaps
            .get_mut(self.inner.execution.current_context().unwrap().heap_id)
        {
            for (slot, value) in memory {
                let end = (slot + 1) * 32;
                heap.expand_memory(end as u32);
                heap.store((slot * 32) as u32, value);
            }
        }
    }

    pub fn get_storage_diff(&mut self) -> Vec<StateDiffRecord> {
        self.inner
            .state
            .get_storage_changes()
            .iter()
            .filter_map(|(storage_key, initial_value, value)| {
                let address = storage_key.address;

                if address == L1_MESSENGER_ADDRESS {
                    return None;
                }

                let key = storage_key.key;

                let diff = StateDiffRecord {
                    key,
                    address,
                    derived_key:
                        zk_evm_1_5_0::aux_structures::LogQuery::derive_final_address_for_params(
                            &address, &key,
                        ),
                    enumeration_index: self
                        .storage
                        .borrow_mut()
                        .get_enumeration_index(&StorageKey::new(
                            AccountTreeId::new(address),
                            u256_to_h256(key),
                        ))
                        .unwrap_or_default(),
                    initial_value: initial_value.unwrap_or_default(),
                    final_value: value.clone(),
                };

                Some(diff)
            })
            // the compressor expects the storage diff to be sorted
            .sorted_by(|a, b| a.address.cmp(&b.address).then_with(|| a.key.cmp(&b.key)))
            .collect()
    }

    pub fn push_transaction_inner(&mut self, tx: Transaction, refund: u64, with_compression: bool) {
        let tx: TransactionData = tx.into();
        let overhead = tx.overhead_gas();

        self.insert_bytecodes(tx.factory_deps.iter().map(|dep| &dep[..]));

        let compressed_bytecodes = if is_l1_tx_type(tx.tx_type) || !with_compression {
            // L1 transactions do not need compression
            vec![]
        } else {
            compress_bytecodes(&tx.factory_deps, |hash| {
                self.inner
                    .state
                    .storage_changes()
                    .get(&EraStorageKey::new(
                        KNOWN_CODES_STORAGE_ADDRESS,
                        h256_to_u256(hash),
                    ))
                    .map(|x| !x.is_zero())
                    .unwrap_or_else(|| self.storage.is_bytecode_known(&hash))
            })
        };

        let trusted_ergs_limit = tx.trusted_ergs_limit();

        let memory = self.bootloader_state.push_tx(
            tx,
            overhead,
            refund,
            compressed_bytecodes,
            trusted_ergs_limit,
            self.system_env.chain_id,
        );

        self.write_to_bootloader_heap(memory);
    }
}

impl<S: ReadStorage + 'static> VmInterface for Vm<S> {
    type TracerDispatcher = TracerDispatcher<S>;

    fn push_transaction(&mut self, tx: Transaction) {
        self.push_transaction_inner(tx, 0, true);
    }

    fn inspect(
        &mut self,
        tracer: Self::TracerDispatcher,
        execution_mode: VmExecutionMode,
    ) -> VmExecutionResultAndLogs {
        let mut track_refunds = false;
        if let VmExecutionMode::OneTx = execution_mode {
            // Move the pointer to the next transaction
            self.bootloader_state.move_tx_to_execute_pointer();
            track_refunds = true;
        }

        let refund_tracer = if track_refunds {
            Some(RefundsTracer::new())
        } else {
            None
        };
        let mut tracer =
            VmTracerManager::new(execution_mode, self.storage.clone(), tracer, refund_tracer);
        let snapshot = self.inner.state.snapshot();

        self.run(&mut tracer);
        // it is actually safe to unwrap here, since we always expect a result
        // the reason we use an option is because we really can't set an initial value in the result tracer
        let result = tracer.result_tracer.result.unwrap();

        let ignore_world_diff = matches!(execution_mode, VmExecutionMode::OneTx)
            && matches!(result, ExecutionResult::Halt { .. });

        let logs = if ignore_world_diff {
            VmExecutionLogs::default()
        } else {
            let events = merge_events(
                self.inner.state.get_events_after_snapshot(snapshot.events),
                self.batch_env.number,
            );
            let user_l2_to_l1_logs = extract_l2tol1logs_from_l1_messenger(&events)
                .into_iter()
                .map(Into::into)
                .map(UserL2ToL1Log)
                .collect();
            let system_l2_to_l1_logs = self
                .inner
                .state
                .get_l2_to_l1_logs_after_snapshot(snapshot.l2_to_l1_logs)
                .iter()
                .map(|log| log.into_system_log())
                .collect();
            let storage_logs: Vec<StorageLogWithPreviousValue> = self
                .inner
                .state
                .get_storage_changes_from_snapshot(snapshot.storage_changes)
                .iter()
                .map(|(storage_key, previos_value, value, is_initial)| {
                    let key = StorageKey::new(
                        AccountTreeId::new(storage_key.address),
                        u256_to_h256(storage_key.key),
                    );

                    StorageLogWithPreviousValue {
                        log: StorageLog {
                            key,
                            value: u256_to_h256(*value),
                            kind: if *is_initial {
                                StorageLogKind::InitialWrite
                            } else {
                                StorageLogKind::RepeatedWrite
                            },
                        },
                        previous_value: u256_to_h256(previos_value.unwrap_or_default()),
                    }
                })
                .collect();

            VmExecutionLogs {
                storage_logs,
                events,
                user_l2_to_l1_logs,
                system_l2_to_l1_logs,
                total_log_queries_count: 0, // This field is unused
            }
        };

        VmExecutionResultAndLogs {
            result,
            logs,
            statistics: VmExecutionStatistics {
                contracts_used: 0,
                cycles_used: 0,
                gas_used: 0,
                gas_remaining: 0,
                computational_gas_used: 0,
                total_log_queries: 0,
                pubdata_published: tracer.pubdata_tracer.pubdata_published,
                circuit_statistic: tracer
                    .circuits_tracer
                    .circuit_statistics(&self.inner.statistics),
            },
            refunds: tracer.refund_tracer.unwrap_or_default().into(),
        }
    }

    fn get_bootloader_memory(&self) -> BootloaderMemory {
        self.bootloader_state.bootloader_memory()
    }

    fn get_last_tx_compressed_bytecodes(&self) -> Vec<CompressedBytecodeInfo> {
        self.bootloader_state.get_last_tx_compressed_bytecodes()
    }

    fn start_new_l2_block(&mut self, l2_block_env: L2BlockEnv) {
        self.bootloader_state.start_new_l2_block(l2_block_env)
    }

    fn get_current_execution_state(&self) -> CurrentExecutionState {
        let state = &self.inner.state;
        let events = merge_events(state.events(), self.batch_env.number);

        let user_l2_to_l1_logs = extract_l2tol1logs_from_l1_messenger(&events)
            .into_iter()
            .map(Into::into)
            .map(UserL2ToL1Log)
            .collect();

        CurrentExecutionState {
            events,
            deduplicated_storage_logs: state
                .storage_changes()
                .iter()
                .map(|(storage_key, value)| StorageLog {
                    key: StorageKey::new(
                        AccountTreeId::new(storage_key.address),
                        u256_to_h256(storage_key.key),
                    ),
                    value: u256_to_h256(*value),
                    kind: StorageLogKind::RepeatedWrite,
                })
                .collect(),
            used_contract_hashes: state.decommitted_hashes().iter().cloned().collect(),
            system_logs: state
                .l2_to_l1_logs()
                .iter()
                .map(|log| log.into_system_log())
                .collect(),
            user_l2_to_l1_logs,
            storage_refunds: state.refunds().clone(),
            pubdata_costs: state.pubdata_costs().clone(),
        }
    }

    fn inspect_transaction_with_bytecode_compression(
        &mut self,
        tracer: Self::TracerDispatcher,
        tx: zksync_types::Transaction,
        with_compression: bool,
    ) -> (
        Result<(), crate::interface::BytecodeCompressionError>,
        VmExecutionResultAndLogs,
    ) {
        todo!()
    }

    fn record_vm_memory_metrics(&self) -> crate::vm_1_4_1::VmMemoryMetrics {
        todo!()
    }

    fn gas_remaining(&self) -> u32 {
        self.inner.execution.current_frame().unwrap().gas_left.0
    }
}

impl<S: ReadStorage + 'static> VmInterfaceHistoryEnabled for Vm<S> {
    fn make_snapshot(&mut self) {
        assert!(
            self.snapshot.is_none(),
            "cannot create a VM snapshot until a previous snapshot is rolled back to or popped"
        );

        self.snapshot = Some(VmSnapshot {
            vm_snapshot: self.inner.snapshot(),
            suspended_at: self.suspended_at,
            gas_for_account_validation: self.gas_for_account_validation,
            bootloader_snapshot: self.bootloader_state.get_snapshot(),
        });
    }

    fn rollback_to_the_latest_snapshot(&mut self) {
        let VmSnapshot {
            vm_snapshot,
            suspended_at,
            gas_for_account_validation,
            bootloader_snapshot,
        } = self.snapshot.take().expect("no snapshots to rollback to");

        self.inner.rollback(vm_snapshot);
        self.bootloader_state.apply_snapshot(bootloader_snapshot);
        self.suspended_at = suspended_at;
        self.gas_for_account_validation = gas_for_account_validation;
    }

    fn pop_snapshot_no_rollback(&mut self) {
        self.snapshot = None;
    }
}

#[derive(Debug, Clone)]
pub struct World<S: ReadStorage> {
    pub storage: StoragePtr<S>,
    pub contract_storage: Rc<RefCell<HashMap<U256, Vec<U256>>>>,
}

impl<S: ReadStorage> World<S> {
    pub fn new_empty(storage: StoragePtr<S>) -> Self {
        let contract_storage = Rc::new(RefCell::new(HashMap::new()));
        Self {
            contract_storage,
            storage,
        }
    }

    pub fn new(
        storage: StoragePtr<S>,
        contract_storage: Rc<RefCell<HashMap<U256, Vec<U256>>>>,
    ) -> Self {
        Self {
            storage,
            contract_storage,
        }
    }
}

impl<S: ReadStorage> era_vm::store::Storage for World<S> {
    fn decommit(&mut self, hash: U256) -> Option<Vec<U256>> {
        Some(
            self.contract_storage
                .borrow_mut()
                .entry(hash)
                .or_insert_with(|| {
                    let contract = self
                        .storage
                        .borrow_mut()
                        .load_factory_dep(u256_to_h256(hash))
                        .expect("Bytecode not found");
                    let mut program_code = vec![];
                    for raw_opcode_slice in contract.chunks(32) {
                        let mut raw_opcode_bytes: [u8; 32] = [0; 32];
                        raw_opcode_bytes.copy_from_slice(&raw_opcode_slice[..32]);

                        let raw_opcode_u256 = U256::from_big_endian(&raw_opcode_bytes);
                        program_code.push(raw_opcode_u256);
                    }
                    program_code
                })
                .clone(),
        )
    }

    fn storage_read(
        &mut self,
        storage_key: &era_vm::store::StorageKey,
    ) -> std::option::Option<U256> {
        let key = &StorageKey::new(
            AccountTreeId::new(storage_key.address),
            u256_to_h256(storage_key.key),
        );

        if self.storage.is_write_initial(&key) {
            None
        } else {
            Some(self.storage.borrow_mut().read_value(key).0.into())
        }
    }

    fn cost_of_writing_storage(
        &mut self,
        storage_key: &era_vm::store::StorageKey,
        value: U256,
    ) -> u32 {
        let initial_value = self.storage_read(storage_key);
        let is_initial = initial_value.is_none();
        let initial_value = initial_value.unwrap_or_default();

        if initial_value == value {
            return 0;
        }

        // Since we need to publish the state diffs onchain, for each of the updated storage slot
        // we basically need to publish the following pair: `(<storage_key, compressed_new_value>)`.
        // For key we use the following optimization:
        //   - The first time we publish it, we use 32 bytes.
        //         Then, we remember a 8-byte id for this slot and assign it to it. We call this initial write.
        //   - The second time we publish it, we will use the 4/5 byte representation of this 8-byte instead of the 32
        //     bytes of the entire key.
        // For value compression, we use a metadata byte which holds the length of the value and the operation from the
        // previous state to the new state, and the compressed value. The maximum for this is 33 bytes.
        // Total bytes for initial writes then becomes 65 bytes and repeated writes becomes 38 bytes.
        let compressed_value_size = compress_with_best_strategy(initial_value, value).len() as u32;

        if is_initial {
            (BYTES_PER_DERIVED_KEY as u32) + compressed_value_size
        } else {
            (BYTES_PER_ENUMERATION_INDEX as u32) + compressed_value_size
        }
    }

    fn is_free_storage_slot(&self, storage_key: &era_vm::store::StorageKey) -> bool {
        storage_key.address == zksync_system_constants::SYSTEM_CONTEXT_ADDRESS
            || storage_key.address == L2_BASE_TOKEN_ADDRESS
                && u256_to_h256(storage_key.key) == key_for_eth_balance(&BOOTLOADER_ADDRESS)
    }
}
