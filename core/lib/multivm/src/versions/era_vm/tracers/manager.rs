use era_vm::{execution::Execution, opcode::Opcode, state::VMState, tracers::tracer::Tracer};
use zksync_state::{ReadStorage, StoragePtr};

use super::{
    circuits_tracer::CircuitsTracer,
    dispatcher::TracerDispatcher,
    pubdata_tracer::PubdataTracer,
    refunds_tracer::RefundsTracer,
    result_tracer::ResultTracer,
    traits::{ExecutionResult, VmTracer},
};
use crate::{era_vm::vm::Vm, vm_1_4_1::VmExecutionMode};

// this tracer manager is the one that gets called when running the vm
pub struct VmTracerManager<S: ReadStorage> {
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

impl<S: ReadStorage> VmTracerManager<S> {
    pub fn new(
        execution_mode: VmExecutionMode,
        storage: StoragePtr<S>,
        dispatcher: TracerDispatcher<S>,
        refund_tracer: Option<RefundsTracer>,
    ) -> Self {
        Self {
            dispatcher,
            refund_tracer,
            circuits_tracer: CircuitsTracer::new(),
            result_tracer: ResultTracer::new(),
            pubdata_tracer: PubdataTracer::new(execution_mode),
            storage,
        }
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
