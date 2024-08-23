use era_vm::{state::VMState, tracers::tracer::Tracer, Execution, Opcode};
use zksync_state::ReadStorage;

use super::traits::VmTracer;
use crate::{era_vm::vm::Vm, interface::tracer::TracerExecutionStatus};

// dispatcher calls to other tracers
pub struct TracerDispatcher<S: ReadStorage> {
    tracers: Vec<Box<dyn VmTracer<S>>>,
}

impl<S: ReadStorage> Default for TracerDispatcher<S> {
    fn default() -> Self {
        Self { tracers: vec![] }
    }
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

    fn after_bootloader_execution(&mut self, state: &mut Vm<S>) {
        for tracer in self.tracers.iter_mut() {
            tracer.after_bootloader_execution(state);
        }
    }

    fn bootloader_hook_call(
        &mut self,
        state: &mut Vm<S>,
        hook: crate::era_vm::hook::Hook,
        hook_params: &[zksync_types::U256; 3],
    ) {
        for tracer in self.tracers.iter_mut() {
            tracer.bootloader_hook_call(state, hook.clone(), &hook_params);
        }
    }

    fn after_vm_run(
        &mut self,
        vm: &mut Vm<S>,
        output: era_vm::vm::ExecutionOutput,
    ) -> crate::interface::tracer::TracerExecutionStatus {
        let mut result = TracerExecutionStatus::Continue;
        for tracer in self.tracers.iter_mut() {
            result = result.stricter(&tracer.after_vm_run(vm, output.clone()));
        }
        result
    }
}
