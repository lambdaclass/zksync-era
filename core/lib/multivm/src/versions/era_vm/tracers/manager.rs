use era_vm::{
    execution::Execution, opcode::Opcode, state::VMState, tracers::tracer::Tracer,
    vm::ExecutionOutput,
};
use zksync_state::{ReadStorage, StoragePtr};

use super::{
    circuits_tracer::CircuitsTracer,
    dispatcher::TracerDispatcher,
    pubdata_tracer::PubdataTracer,
    refunds_tracer::RefundsTracer,
    result_tracer::ResultTracer,
    traits::{ExecutionResult, VmTracer},
};
use crate::{
    era_vm::{bootloader_state::utils::apply_l2_block, hook::Hook, vm::Vm},
    interface::{Halt, TxRevertReason},
    vm_1_4_1::VmExecutionMode,
};

// this tracer manager is the one that gets called when running the vm
// all the logic of hooks and results parsing is managed from here
// the most important tracers are: `result_tracer`, `refund_tracer`, `pubdata_tracer`,
pub struct VmTracerManager<S: ReadStorage> {
    execution_mode: VmExecutionMode,
    pub dispatcher: TracerDispatcher<S>,
    pub result_tracer: ResultTracer,
    // This tracer is designed specifically for calculating refunds and saves the results to `VmResultAndLogs`.
    pub refund_tracer: Option<RefundsTracer>,
    // The pubdata tracer is responsible for inserting the pubdata packing information into the bootloader
    // memory at the end of the batch. Its separation from the custom tracer
    // ensures static dispatch, enhancing performance by avoiding dynamic dispatch overhe
    pub pubdata_tracer: PubdataTracer,
    pub circuits_tracer: CircuitsTracer,
    storage: StoragePtr<S>,
}

impl<S: ReadStorage + 'static> VmTracerManager<S> {
    pub fn new(
        execution_mode: VmExecutionMode,
        storage: StoragePtr<S>,
        dispatcher: TracerDispatcher<S>,
        refund_tracer: Option<RefundsTracer>,
    ) -> Self {
        Self {
            execution_mode,
            dispatcher,
            refund_tracer,
            circuits_tracer: CircuitsTracer::new(),
            result_tracer: ResultTracer::new(),
            pubdata_tracer: PubdataTracer::new(execution_mode),
            storage,
        }
    }

    pub fn after_vm_run(
        &mut self,
        vm: &mut Vm<S>,
        output: ExecutionOutput,
    ) -> Option<ExecutionResult> {
        match output {
            ExecutionOutput::Ok(output) => Some(ExecutionResult::Success { output }),
            ExecutionOutput::Revert(output) => match TxRevertReason::parse_error(&output) {
                TxRevertReason::TxReverted(output) => (Some(ExecutionResult::Revert { output })),
                TxRevertReason::Halt(reason) => Some(ExecutionResult::Halt { reason }),
            },
            ExecutionOutput::Panic => Some(ExecutionResult::Halt {
                reason: if vm.inner.execution.gas_left().unwrap() == 0 {
                    Halt::BootloaderOutOfGas
                } else {
                    Halt::VMPanic
                },
            }),
            ExecutionOutput::SuspendedOnHook {
                hook,
                pc_to_resume_from,
            } => {
                vm.suspended_at = pc_to_resume_from;
                vm.inner.execution.current_frame_mut().unwrap().pc = vm.suspended_at as u64;
                let hook = Hook::from_u32(hook);
                match hook {
                    Hook::TxHasEnded => {
                        if let VmExecutionMode::OneTx = self.execution_mode {
                            return self.result_tracer.last_tx_result.take();
                        }
                    }
                    Hook::FinalBatchInfo => self.set_final_batch_info(vm),
                    _ => {}
                }
                self.bootloader_hook_call(vm, hook, &vm.get_hook_params());
                None
            }
        }
    }

    fn set_final_batch_info(&self, vm: &mut Vm<S>) {
        // set fictive l2 block
        let txs_index = vm.bootloader_state.free_tx_index();
        let l2_block = vm.bootloader_state.insert_fictive_l2_block();
        let mut memory = vec![];
        apply_l2_block(&mut memory, l2_block, txs_index);
        vm.write_to_bootloader_heap(memory);
    }
}

impl<S: ReadStorage> Tracer for VmTracerManager<S> {
    fn before_decoding(&mut self, execution: &mut Execution, state: &mut VMState) {
        // Call the dispatcher to handle all the tracers added to it
        self.dispatcher.before_decoding(execution, state);

        // Individual tracers
        self.result_tracer.before_decoding(execution, state);
        if let Some(refunds_tracer) = &mut self.refund_tracer {
            refunds_tracer.before_decoding(execution, state);
        }
        self.pubdata_tracer.before_decoding(execution, state);
        self.circuits_tracer.before_decoding(execution, state);
    }

    fn after_decoding(&mut self, opcode: &Opcode, execution: &mut Execution, state: &mut VMState) {
        // Call the dispatcher to handle all the tracers added to it
        self.dispatcher.after_decoding(opcode, execution, state);

        // Individual tracers
        self.result_tracer.after_decoding(opcode, execution, state);
        if let Some(refunds_tracer) = &mut self.refund_tracer {
            refunds_tracer.after_decoding(opcode, execution, state);
        }
        self.pubdata_tracer.after_decoding(opcode, execution, state);
        self.circuits_tracer
            .after_decoding(opcode, execution, state);
    }

    fn before_execution(
        &mut self,
        opcode: &Opcode,
        execution: &mut Execution,
        state: &mut VMState,
    ) {
        // Call the dispatcher to handle all the tracers added to it
        self.dispatcher.before_execution(opcode, execution, state);

        // Individual tracers
        self.result_tracer
            .before_execution(opcode, execution, state);
        if let Some(refunds_tracer) = &mut self.refund_tracer {
            refunds_tracer.before_execution(opcode, execution, state);
        }
        self.pubdata_tracer
            .before_execution(opcode, execution, state);
        self.circuits_tracer
            .before_execution(opcode, execution, state);
    }

    fn after_execution(&mut self, opcode: &Opcode, execution: &mut Execution, state: &mut VMState) {
        // Call the dispatcher to handle all the tracers added to it
        self.dispatcher.after_execution(opcode, execution, state);

        // Individual tracers
        self.result_tracer.after_execution(opcode, execution, state);
        if let Some(refunds_tracer) = &mut self.refund_tracer {
            refunds_tracer.after_execution(opcode, execution, state);
        }
        self.pubdata_tracer
            .after_execution(opcode, execution, state);
        self.circuits_tracer
            .after_execution(opcode, execution, state);
    }
}

impl<S: ReadStorage + 'static> VmTracer<S> for VmTracerManager<S> {
    fn before_bootloader_execution(&mut self, state: &mut Vm<S>) {
        // Call the dispatcher to handle all the tracers added to it
        self.dispatcher.before_bootloader_execution(state);

        // Individual tracers
        self.result_tracer.before_bootloader_execution(state);

        if let Some(refunds_tracer) = &mut self.refund_tracer {
            refunds_tracer.before_bootloader_execution(state);
        }
        self.pubdata_tracer.before_bootloader_execution(state);
        self.circuits_tracer.before_bootloader_execution(state);
    }

    fn after_bootloader_execution(&mut self, state: &mut Vm<S>, stop_reason: ExecutionResult) {
        // Call the dispatcher to handle all the tracers added to it
        self.dispatcher
            .after_bootloader_execution(state, stop_reason.clone());

        // Individual tracers
        self.result_tracer
            .after_bootloader_execution(state, stop_reason.clone());
        if let Some(refunds_tracer) = &mut self.refund_tracer {
            refunds_tracer.after_bootloader_execution(state, stop_reason.clone());
        }
        self.pubdata_tracer
            .after_bootloader_execution(state, stop_reason.clone());
        self.circuits_tracer
            .after_bootloader_execution(state, stop_reason.clone());
    }

    fn bootloader_hook_call(
        &mut self,
        state: &mut Vm<S>,
        hook: crate::era_vm::hook::Hook,
        hook_params: &[zksync_types::U256; 3],
    ) {
        // Call the dispatcher to handle all the tracers added to it
        self.dispatcher
            .bootloader_hook_call(state, hook.clone(), hook_params);

        // Individual tracers
        self.result_tracer
            .bootloader_hook_call(state, hook.clone(), hook_params);
        if let Some(refunds_tracer) = &mut self.refund_tracer {
            refunds_tracer.bootloader_hook_call(state, hook.clone(), hook_params);
        }
        self.pubdata_tracer
            .bootloader_hook_call(state, hook.clone(), hook_params);
        self.circuits_tracer
            .bootloader_hook_call(state, hook.clone(), hook_params);
    }
}
