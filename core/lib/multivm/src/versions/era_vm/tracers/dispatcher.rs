use era_vm::{state::VMState, tracers::tracer::Tracer, vm::ExecutionOutput, Execution, Opcode};
use zksync_state::ReadStorage;

use super::traits::{ExecutionResult, VmTracer};
use crate::era_vm::vm::Vm;

#[derive(Default)]
// dispatcher calls to other tracers
pub struct TracerDispatcher<S: ReadStorage> {
    tracers: Vec<Box<dyn VmTracer<S>>>,
}

impl<S: ReadStorage> TracerDispatcher<S> {
    pub fn new(tracers: Vec<Box<dyn VmTracer<S>>>) -> Self {
        Self { tracers }
    }
}

impl<S: ReadStorage> Tracer for TracerDispatcher<S> {
    fn before_decoding(&mut self, execution: &mut Execution, state: &mut VMState) {
        for tracer in self.tracers.iter_mut() {
            tracer.before_decoding(execution, state);
        }
    }

    fn after_decoding(&mut self, opcode: &Opcode, execution: &mut Execution, state: &mut VMState) {
        for tracer in self.tracers.iter_mut() {
            tracer.after_decoding(opcode, execution, state);
        }
    }

    fn before_execution(
        &mut self,
        opcode: &Opcode,
        execution: &mut Execution,
        state: &mut VMState,
    ) {
        for tracer in self.tracers.iter_mut() {
            tracer.before_execution(opcode, execution, state);
        }
    }

    fn after_execution(&mut self, opcode: &Opcode, execution: &mut Execution, state: &mut VMState) {
        for tracer in self.tracers.iter_mut() {
            tracer.after_execution(opcode, execution, state);
        }
    }
}

impl<S: ReadStorage> VmTracer<S> for TracerDispatcher<S> {
    fn before_bootloader_execution(&mut self, state: &mut Vm<S>) {
        for tracer in self.tracers.iter_mut() {
            tracer.before_bootloader_execution(state);
        }
    }

    fn after_bootloader_execution(&mut self, state: &mut Vm<S>, stop_reason: ExecutionResult) {
        for tracer in self.tracers.iter_mut() {
            tracer.after_bootloader_execution(state, stop_reason.clone());
        }
    }
}
