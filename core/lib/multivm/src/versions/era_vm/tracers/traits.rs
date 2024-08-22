pub use era_vm::tracers::tracer::Tracer;
use era_vm::{state::VMState, Execution, Opcode};

pub trait BootloaderTracer {
    fn before_bootloader_execution(
        &mut self,
        opcode: &Opcode,
        execution: &mut Execution,
        state: &mut VMState,
    );

    fn after_bootloader_execution(
        &mut self,
        opcode: &Opcode,
        execution: &mut Execution,
        state: &mut VMState,
    );
}

pub trait VmTracer: Tracer + BootloaderTracer {}
impl<T: Tracer + VmTracer> VmTracer for T {}
