use era_vm::{execution::Execution, opcode::Opcode, state::VMState, tracers::tracer::Tracer};
use zksync_state::{ReadStorage, StoragePtr};

use super::{
    circuits_tracer::CircuitsTracer,
    dispatcher::TracerDispatcher,
    pubdata_tracer::PubdataTracer,
    refunds_tracer::RefundsTracer,
    result_tracer::ResultTracer,
    traits::{BootloaderTracer, VmTracer},
};

// this tracer manager is the one that gets called when running the vm
pub struct VmTracerManager<S: ReadStorage> {
    dispatcher: TracerDispatcher,
    result_tracer: ResultTracer,
    refund_tracer: Option<RefundsTracer>,
    pubdata_tracer: Option<PubdataTracer>,
    circuits_tracer: CircuitsTracer,
    storage: StoragePtr<S>,
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
        if let Some(pubdata_tracer) = &mut self.pubdata_tracer {
            pubdata_tracer.before_decoding(execution, state);
        }
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
        if let Some(pubdata_tracer) = &mut self.pubdata_tracer {
            pubdata_tracer.after_decoding(opcode, execution, state);
        }
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
        if let Some(pubdata_tracer) = &mut self.pubdata_tracer {
            pubdata_tracer.before_execution(opcode, execution, state);
        }
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
        if let Some(pubdata_tracer) = &mut self.pubdata_tracer {
            pubdata_tracer.after_execution(opcode, execution, state);
        }
        self.circuits_tracer
            .after_execution(opcode, execution, state);
    }
}

impl<S: ReadStorage> BootloaderTracer for VmTracerManager<S> {
    fn before_bootloader_execution(
        &mut self,
        opcode: &Opcode,
        execution: &mut Execution,
        state: &mut VMState,
    ) {
        // Call the dispatcher to handle all the tracers added to it
        self.dispatcher
            .before_bootloader_execution(opcode, execution, state);

        // Individual tracers
        self.result_tracer
            .before_bootloader_execution(opcode, execution, state);
        if let Some(refunds_tracer) = &mut self.refund_tracer {
            refunds_tracer.before_bootloader_execution(opcode, execution, state);
        }
        if let Some(pubdata_tracer) = &mut self.pubdata_tracer {
            pubdata_tracer.before_bootloader_execution(opcode, execution, state);
        }
        self.circuits_tracer
            .before_bootloader_execution(opcode, execution, state);
    }

    fn after_bootloader_execution(
        &mut self,
        opcode: &Opcode,
        execution: &mut Execution,
        state: &mut VMState,
    ) {
        // Call the dispatcher to handle all the tracers added to it
        self.dispatcher
            .after_bootloader_execution(opcode, execution, state);

        // Individual tracers
        self.result_tracer
            .after_bootloader_execution(opcode, execution, state);
        if let Some(refunds_tracer) = &mut self.refund_tracer {
            refunds_tracer.after_bootloader_execution(opcode, execution, state);
        }
        if let Some(pubdata_tracer) = &mut self.pubdata_tracer {
            pubdata_tracer.after_bootloader_execution(opcode, execution, state);
        }
        self.circuits_tracer
            .after_bootloader_execution(opcode, execution, state);
    }
}
