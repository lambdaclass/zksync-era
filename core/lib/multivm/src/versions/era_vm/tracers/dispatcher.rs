use era_vm::{state::VMState, tracers::tracer::Tracer, Execution, Opcode};

use super::traits::{BootloaderTracer, VmTracer};

#[derive(Default)]
// dispatcher calls to other tracers
pub struct TracerDispatcher {
    tracers: Vec<Box<dyn VmTracer>>,
}

impl TracerDispatcher {
    pub fn new(tracers: Vec<Box<dyn VmTracer>>) -> Self {
        Self { tracers }
    }
}

impl Tracer for TracerDispatcher {
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

impl BootloaderTracer for TracerDispatcher {
    fn before_bootloader_execution(
        &mut self,
        opcode: &Opcode,
        execution: &mut Execution,
        state: &mut VMState,
    ) {
        for tracer in self.tracers.iter_mut() {
            tracer.before_bootloader_execution(opcode, execution, state);
        }
    }

    fn after_bootloader_execution(
        &mut self,
        opcode: &Opcode,
        execution: &mut Execution,
        state: &mut VMState,
    ) {
        for tracer in self.tracers.iter_mut() {
            tracer.after_bootloader_execution(opcode, execution, state);
        }
    }
}
