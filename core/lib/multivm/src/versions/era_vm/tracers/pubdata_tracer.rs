use itertools::Itertools;
use zksync_state::ReadStorage;
use zksync_types::{
    event::{
        extract_l2tol1logs_from_l1_messenger, extract_long_l2_to_l1_messages,
        L1_MESSENGER_BYTECODE_PUBLICATION_EVENT_SIGNATURE,
    },
    writes::StateDiffRecord,
    AccountTreeId, StorageKey, L1_MESSENGER_ADDRESS, U256,
};
use zksync_utils::u256_to_h256;

use super::traits::{Tracer, Vm, VmTracer};
use crate::{
    era_vm::{
        bootloader_state::utils::{apply_pubdata_to_memory, PubdataInput},
        event::merge_events,
        hook::Hook,
    },
    interface::tracer::{TracerExecutionStatus, TracerExecutionStopReason},
    vm_1_4_1::VmExecutionMode,
};

pub struct PubdataTracer {
    execution_mode: VmExecutionMode,
    pubdata_before_run: i32,
    should_stop: bool,
    pub pubdata_published: u32,
    // this field is to enforce a custom storage diff when setting the pubdata to the bootloader
    // this is meant to be used for testing purposes only.
    enforced_storage_diff: Option<Vec<StateDiffRecord>>,
}

impl PubdataTracer {
    pub fn new(execution_mode: VmExecutionMode) -> Self {
        Self {
            execution_mode,
            pubdata_before_run: 0,
            pubdata_published: 0,
            enforced_storage_diff: None,
            should_stop: false,
        }
    }

    pub fn new_with_forced_state_diffs(
        execution_mode: VmExecutionMode,
        diff: Vec<StateDiffRecord>,
    ) -> Self {
        Self {
            enforced_storage_diff: Some(diff),
            ..Self::new(execution_mode)
        }
    }

    fn get_storage_diff<S: ReadStorage>(&mut self, vm: &mut Vm<S>) -> Vec<StateDiffRecord> {
        vm.inner
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
                    enumeration_index: vm
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
}

impl Tracer for PubdataTracer {}

impl<S: ReadStorage + 'static> VmTracer<S> for PubdataTracer {
    fn before_bootloader_execution(&mut self, vm: &mut super::traits::Vm<S>) {
        self.pubdata_before_run = vm.inner.state.pubdata();
    }

    fn after_bootloader_execution(&mut self, vm: &mut super::traits::Vm<S>) {
        self.pubdata_published = (vm.inner.state.pubdata() - self.pubdata_before_run).max(0) as u32;
    }

    fn after_vm_run(
        &mut self,
        _vm: &mut Vm<S>,
        _output: era_vm::vm::ExecutionOutput,
    ) -> TracerExecutionStatus {
        if self.should_stop {
            return TracerExecutionStatus::Stop(TracerExecutionStopReason::Finish);
        }
        TracerExecutionStatus::Continue
    }

    fn bootloader_hook_call(
        &mut self,
        vm: &mut Vm<S>,
        hook: Hook,
        _hook_params: &[zksync_types::U256; 3],
    ) {
        if let Hook::PubdataRequested = hook {
            if !matches!(self.execution_mode, VmExecutionMode::Batch) {
                self.should_stop = true;
            };

            let state_diffs = if let Some(diff) = &self.enforced_storage_diff {
                diff.clone()
            } else {
                self.get_storage_diff(vm)
            };

            let events = merge_events(vm.inner.state.events(), vm.batch_env.number);

            let published_bytecodes: Vec<Vec<u8>> = events
                .iter()
                .filter(|event| {
                    // Filter events from the l1 messenger contract that match the expected signature.
                    event.address == L1_MESSENGER_ADDRESS
                        && !event.indexed_topics.is_empty()
                        && event.indexed_topics[0]
                            == *L1_MESSENGER_BYTECODE_PUBLICATION_EVENT_SIGNATURE
                })
                .map(|event| {
                    let hash = U256::from_big_endian(&event.value[..32]);
                    vm.storage
                        .load_factory_dep(u256_to_h256(hash))
                        .expect("published unknown bytecode")
                        .clone()
                })
                .collect();

            let pubdata_input = PubdataInput {
                user_logs: extract_l2tol1logs_from_l1_messenger(&events),
                l2_to_l1_messages: extract_long_l2_to_l1_messages(&events),
                published_bytecodes,
                state_diffs,
            };

            // Save the pubdata for the future initial bootloader memory building
            vm.bootloader_state.set_pubdata_input(pubdata_input.clone());

            // Apply the pubdata to the current memory
            let mut memory_to_apply = vec![];

            apply_pubdata_to_memory(&mut memory_to_apply, pubdata_input);
            vm.write_to_bootloader_heap(memory_to_apply);
        }
    }
}
