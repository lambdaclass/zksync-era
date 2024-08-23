pub use crate::{era_vm::vm::Vm, vm_latest::ExecutionResult};
pub use era_vm::tracers::tracer::Tracer;
use zksync_state::ReadStorage;

pub trait VmTracer<S: ReadStorage>: Tracer {
    fn before_bootloader_execution(&mut self, _state: &mut Vm<S>) {}

    fn after_bootloader_execution(&mut self, _state: &mut Vm<S>, _stop_reason: ExecutionResult) {}
}
