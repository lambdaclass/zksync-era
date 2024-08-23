pub use era_vm::tracers::tracer::Tracer;
use zksync_state::ReadStorage;
use zksync_types::U256;

use crate::era_vm::hook::Hook;
pub use crate::{era_vm::vm::Vm, vm_latest::ExecutionResult};

pub trait VmTracer<S: ReadStorage>: Tracer {
    fn before_bootloader_execution(&mut self, _state: &mut Vm<S>) {}

    fn after_bootloader_execution(&mut self, _state: &mut Vm<S>, _stop_reason: ExecutionResult) {}

    fn bootloader_hook_call(&mut self, _state: &mut Vm<S>, _hook: Hook, _hook_params: &[U256; 3]) {}
}
