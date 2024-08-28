use std::{cell::RefCell, collections::HashMap, rc::Rc};

use era_vm::{
    rollbacks::Rollbackable, store::StorageKey as EraStorageKey, value::FatPointer,
    vm::ExecutionOutput, EraVM, Execution,
};
use itertools::Itertools;
use zksync_state::{ReadStorage, StoragePtr};
use zksync_types::{
    circuit::CircuitStatistic,
    event::extract_l2tol1logs_from_l1_messenger,
    l1::is_l1_tx_type,
    l2_to_l1_log::UserL2ToL1Log,
    utils::key_for_eth_balance,
    writes::{
        compression::compress_with_best_strategy, BYTES_PER_DERIVED_KEY,
        BYTES_PER_ENUMERATION_INDEX,
    },
    AccountTreeId, StorageKey, StorageLog, StorageLogKind, StorageLogWithPreviousValue,
    Transaction, BOOTLOADER_ADDRESS, H160, KNOWN_CODES_STORAGE_ADDRESS, L2_BASE_TOKEN_ADDRESS,
    U256,
};
use zksync_utils::{
    bytecode::{hash_bytecode, CompressedBytecodeInfo},
    h256_to_u256, u256_to_h256,
};

use super::{
    bootloader_state::{utils::apply_l2_block, BootloaderState},
    event::merge_events,
    hook::Hook,
    initial_bootloader_memory::bootloader_initial_memory,
    logs::IntoSystemLog,
    snapshot::VmSnapshot,
    tracers::{
        dispatcher::TracerDispatcher,
        manager::VmTracerManager,
        pubdata_tracer::PubdataTracer,
        refunds_tracer::{Refunds, RefundsTracer},
        traits::VmTracer,
    },
    transaction::ParallelTransaction,
};
use crate::{
    era_vm::{bytecode::compress_bytecodes, transaction_data::TransactionData},
    interface::{
        tracer::{TracerExecutionStatus, TracerExecutionStopReason},
        Halt, TxRevertReason, VmFactory, VmInterface, VmInterfaceHistoryEnabled, VmRevertReason,
    },
    vm_1_4_1::FinishedL1Batch,
    vm_latest::{
        constants::{
            get_result_success_first_slot, get_vm_hook_position, get_vm_hook_start_position_latest,
            VM_HOOK_PARAMS_COUNT,
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

    pub transaction_to_execute: Vec<ParallelTransaction>,
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
            transaction_to_execute: vec![],
        };

        mv.write_to_bootloader_heap(bootloader_memory);
        mv
    }
}

impl<S: ReadStorage + 'static> Vm<S> {
    pub fn run(
        &mut self,
        execution_mode: VmExecutionMode,
        tracer: &mut impl VmTracer<S>,
    ) -> ExecutionResult {
        tracer.before_bootloader_execution(self);
        let mut last_tx_result: Option<ExecutionResult> = None;
        let result = loop {
            let output = self
                .inner
                .run_program_with_custom_bytecode_and_tracer(tracer);
            let status = tracer.after_vm_run(self, output.clone());
            let (hook, hook_params) = match output {
                ExecutionOutput::Ok(output) => break ExecutionResult::Success { output },
                ExecutionOutput::Revert(output) => match TxRevertReason::parse_error(&output) {
                    TxRevertReason::TxReverted(output) => break ExecutionResult::Revert { output },
                    TxRevertReason::Halt(reason) => break ExecutionResult::Halt { reason },
                },
                ExecutionOutput::Panic => {
                    break ExecutionResult::Halt {
                        reason: if self.inner.execution.gas_left().unwrap() == 0 {
                            Halt::BootloaderOutOfGas
                        } else {
                            Halt::VMPanic
                        },
                    }
                }
                ExecutionOutput::SuspendedOnHook {
                    hook,
                    pc_to_resume_from,
                } => {
                    self.suspended_at = pc_to_resume_from;
                    self.inner.execution.current_frame_mut().unwrap().pc = self.suspended_at as u64;
                    (Hook::from_u32(hook), self.get_hook_params())
                }
            };

            tracer.bootloader_hook_call(self, hook.clone(), &self.get_hook_params());

            match hook {
                Hook::PostResult => {
                    let result = hook_params[0];
                    let value = hook_params[1];
                    let pointer = FatPointer::decode(value);
                    assert_eq!(pointer.offset, 0);

                    let return_data = self
                        .inner
                        .execution
                        .heaps
                        .get(pointer.page)
                        .unwrap()
                        .read_unaligned_from_pointer(&pointer)
                        .unwrap();

                    last_tx_result = Some(if result.is_zero() {
                        ExecutionResult::Revert {
                            output: VmRevertReason::from(return_data.as_slice()),
                        }
                    } else {
                        ExecutionResult::Success {
                            output: return_data,
                        }
                    });
                }
                Hook::TxHasEnded => {
                    if let VmExecutionMode::OneTx = execution_mode {
                        break last_tx_result
                            .expect("There should always be a result if we got this hook");
                    }
                }
                Hook::FinalBatchInfo => {
                    // set fictive l2 block
                    let txs_index = self.bootloader_state.free_tx_index();
                    let l2_block = self.bootloader_state.insert_fictive_l2_block();
                    let mut memory = vec![];
                    apply_l2_block(&mut memory, l2_block, txs_index);
                    self.write_to_bootloader_heap(memory);
                }
                _ => {}
            }

            if let TracerExecutionStatus::Stop(reason) = status {
                match reason {
                    TracerExecutionStopReason::Abort(halt) => {
                        break ExecutionResult::Halt { reason: halt }
                    }
                    TracerExecutionStopReason::Finish => {
                        if self.inner.execution.gas_left().unwrap() == 0 {
                            break ExecutionResult::Halt {
                                reason: Halt::BootloaderOutOfGas,
                            };
                        }
                        if last_tx_result.is_some() {
                            break last_tx_result.unwrap();
                        }
                        let has_failed =
                            self.tx_has_failed(self.bootloader_state.current_tx() as u32);
                        if has_failed {
                            break ExecutionResult::Revert {
                                output: crate::interface::VmRevertReason::General {
                                    msg: "Transaction reverted with empty reason. Possibly out of gas".to_string(),
                                    data: vec![],
                                },
                            };
                        } else {
                            break ExecutionResult::Success { output: vec![] };
                        }
                    }
                }
            }
        };
        tracer.after_bootloader_execution(self);
        result
    }

    fn tx_has_failed(&self, tx_id: u32) -> bool {
        let mem_slot = get_result_success_first_slot(
            crate::vm_latest::MultiVMSubversion::IncreasedBootloaderMemory,
        ) + tx_id;
        let mem_value = self.read_heap_word(mem_slot as usize);
        mem_value == U256::zero()
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

    pub fn push_parallel_transaction(
        &mut self,
        tx: Transaction,
        refund: u64,
        with_compression: bool,
    ) {
        self.transaction_to_execute
            .push(ParallelTransaction::new(tx, refund, with_compression));
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

    pub fn inspect_inner(
        &mut self,
        tracer: TracerDispatcher<S>,
        custom_pubdata_tracer: Option<PubdataTracer>,
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
            VmTracerManager::new(execution_mode, tracer, refund_tracer, custom_pubdata_tracer);
        let snapshot = self.inner.state.snapshot();

        let ergs_before = self.inner.execution.gas_left().unwrap();
        let monotonic_counter_before = self.inner.statistics.monotonic_counter;

        let result = self.run(execution_mode, &mut tracer);
        let ergs_after = self.inner.execution.gas_left().unwrap();

        let ignore_world_diff = matches!(execution_mode, VmExecutionMode::OneTx)
            && matches!(result, ExecutionResult::Halt { .. });

        let logs = if ignore_world_diff {
            VmExecutionLogs::default()
        } else {
            self.get_execution_logs(snapshot)
        };

        VmExecutionResultAndLogs {
            result,
            logs,
            statistics: self.get_execution_statistics(
                monotonic_counter_before,
                tracer.pubdata_tracer.pubdata_published,
                ergs_before,
                ergs_after,
                tracer
                    .circuits_tracer
                    .circuit_statistics(&self.inner.statistics),
            ),
            refunds: tracer.refund_tracer.unwrap_or_default().into(),
        }
    }

    fn get_execution_logs(&self, snapshot: era_vm::state::StateSnapshot) -> VmExecutionLogs {
        dbg!(&snapshot);
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
            .sorted_by(|a, b| {
                a.log
                    .key
                    .address()
                    .cmp(&b.log.key.address())
                    .then_with(|| a.log.key.key().cmp(&b.log.key.key()))
            })
            .collect();

        VmExecutionLogs {
            storage_logs,
            events,
            user_l2_to_l1_logs,
            system_l2_to_l1_logs,
            total_log_queries_count: 0, // This field is unused
        }
    }

    fn get_execution_statistics(
        &self,
        monotonic_counter_before: u32,
        pubdata_published: u32,
        ergs_before: u32,
        ergs_after: u32,
        circuit_statistic: CircuitStatistic,
    ) -> VmExecutionStatistics {
        VmExecutionStatistics {
            contracts_used: self.inner.state.decommitted_hashes().len(),
            cycles_used: self.inner.statistics.monotonic_counter - monotonic_counter_before,
            gas_used: (ergs_before - ergs_after) as u64,
            gas_remaining: ergs_after,
            computational_gas_used: ergs_before - ergs_after,
            total_log_queries: 0,
            pubdata_published,
            circuit_statistic,
        }
    }

    // Very basic parallel execution model, basically, we are spawning a new vm per transactions and then mergint the results
    // and creating the batch from the final merged state. We are not accounting for gas sharing, transactions that depend upon each other, etc
    // in the future, this would become a new mode of execution (i.e VmExecutionMode::Parallel)
    pub fn execute_parallel(&mut self) -> VmExecutionResultAndLogs {
        let transactions: Vec<ParallelTransaction> =
            self.transaction_to_execute.drain(..).collect();

        // we only care about the final VMState, since that is where the pubdata and L2 changes reside
        // we will merge this results later
        let mut final_states: Vec<era_vm::state::VMState> = vec![self.inner.state.clone()];

        // to run in parallel, we spin up new vms to process and run the transaction in their own bootloader
        for tx in transactions {
            let mut vm = Vm::new(
                self.batch_env.clone(),
                self.system_env.clone(),
                self.storage.clone(),
            );

            vm.push_transaction_inner(tx.tx, tx.refund, tx.with_compression);

            // in the future we don't want to call the bootloader for this, and instead crate a new era_vm and pass the
            // transaction bytecode directly, that would require to build all the validation logic for processing the transaction
            // in rust
            let result =
                vm.inspect_inner(TracerDispatcher::default(), None, VmExecutionMode::OneTx);

            // if one transaction fails, the whole batch fails
            if result.result.is_failed() {
                return result;
            }

            final_states.push(vm.inner.state);
        }

        // since no transactions have been pushed onto the bootloader, here it will only call the SetFictiveBlock and request the pubdata
        let mut tracer = VmTracerManager::new(
            VmExecutionMode::Batch,
            TracerDispatcher::default(),
            None,
            None,
        );
        let ergs_before = self.inner.execution.gas_left().unwrap();
        let monotonic_counter_before = self.inner.statistics.monotonic_counter;

        let snapshot = self.inner.state.snapshot();
        // finally, we need to merge the results to the current vm
        self.inner.state = self.merge_vm_states(final_states);
        let result = self.run(VmExecutionMode::Batch, &mut tracer);
        let ergs_after = self.inner.execution.gas_left().unwrap();

        VmExecutionResultAndLogs {
            result,
            logs: self.get_execution_logs(snapshot),
            // we should handle the statistics merge properly
            // for this first model, it isn't that important
            statistics: self.get_execution_statistics(
                monotonic_counter_before,
                tracer.pubdata_tracer.pubdata_published,
                ergs_before,
                ergs_after,
                tracer
                    .circuits_tracer
                    .circuit_statistics(&self.inner.statistics),
            ),
            refunds: tracer.refund_tracer.unwrap_or_default().into(),
        }
    }

    pub fn merge_vm_states(
        &self,
        vm_states: Vec<era_vm::state::VMState>,
    ) -> era_vm::state::VMState {
        let mut final_state = era_vm::state::VMState::new(self.inner.state.storage.clone());

        for state in vm_states {
            for log in state.l2_to_l1_logs() {
                final_state.record_l2_to_l1_log(log.clone());
            }
            for event in state.events() {
                final_state.record_event(event.clone());
            }
            for (key, value) in state.storage_changes().iter() {
                final_state.storage_write(key.clone(), value.clone());
            }
            final_state.add_pubdata(state.pubdata());
            for hash in state.decommitted_hashes().iter() {
                // this is to add the hash to the decommited hashes
                final_state.decommit(hash.clone());
            }
        }

        final_state
    }

    pub fn merge_statistics(&self) {}
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
        self.inspect_inner(tracer, None, execution_mode)
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

    fn finish_batch(&mut self) -> FinishedL1Batch {
        let result = self.execute(VmExecutionMode::Batch);
        let execution_state = self.get_current_execution_state();
        let bootloader_memory = self.get_bootloader_memory();

        FinishedL1Batch {
            block_tip_execution_result: result,
            final_execution_state: execution_state,
            final_bootloader_memory: Some(bootloader_memory),
            pubdata_input: Some(
                self.bootloader_state
                    .get_pubdata_information()
                    .clone()
                    .build_pubdata(false),
            ),
            state_diffs: Some(
                self.bootloader_state
                    .get_pubdata_information()
                    .state_diffs
                    .to_vec(),
            ),
        }
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
