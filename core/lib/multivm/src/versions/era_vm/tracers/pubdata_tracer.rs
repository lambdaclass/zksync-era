use zksync_state::ReadStorage;
use zksync_types::{
    event::{
        extract_l2tol1logs_from_l1_messenger, extract_long_l2_to_l1_messages,
        L1_MESSENGER_BYTECODE_PUBLICATION_EVENT_SIGNATURE,
    },
    L1_MESSENGER_ADDRESS, U256,
};
use zksync_utils::u256_to_h256;

use super::traits::{Tracer, VmTracer};
use crate::{
    era_vm::{
        bootloader_state::utils::{apply_pubdata_to_memory, PubdataInput},
        event::merge_events,
        hook::Hook,
    },
    vm_1_4_1::VmExecutionMode,
};

pub struct PubdataTracer {
    execution_mode: VmExecutionMode,
    pubdata_before_run: i32,
    pub pubdata_published: u32,
}

impl PubdataTracer {
    pub fn new(execution_mode: VmExecutionMode) -> Self {
        Self {
            execution_mode,
            pubdata_before_run: 0,
            pubdata_published: 0,
        }
    }
}

impl Tracer for PubdataTracer {}

impl<S: ReadStorage + 'static> VmTracer<S> for PubdataTracer {
    fn before_bootloader_execution(&mut self, vm: &mut super::traits::Vm<S>) {
        self.pubdata_before_run = vm.inner.state.pubdata();
    }

    fn after_bootloader_execution(
        &mut self,
        vm: &mut super::traits::Vm<S>,
        _stop_reason: super::traits::ExecutionResult,
    ) {
        self.pubdata_published = (vm.inner.state.pubdata() - self.pubdata_before_run).max(0) as u32;
    }

    fn bootloader_hook_call(
        &mut self,
        vm: &mut super::traits::Vm<S>,
        hook: Hook,
        _hook_params: &[zksync_types::U256; 3],
    ) {
        if let Hook::PubdataRequested = hook {
            if !matches!(self.execution_mode, VmExecutionMode::Batch) {
                unreachable!("We do not provide the pubdata when executing the block tip or a single transaction");
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
                state_diffs: vm.get_storage_diff(),
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
