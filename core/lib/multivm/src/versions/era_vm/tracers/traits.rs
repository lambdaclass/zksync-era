pub use era_vm::tracers::tracer::Tracer;
use era_vm::vm::ExecutionOutput;
use zksync_state::ReadStorage;
use zksync_types::U256;

use crate::{era_vm::hook::Hook, interface::tracer::TracerExecutionStatus};
pub use crate::{era_vm::vm::Vm, vm_latest::ExecutionResult};

pub trait VmTracer<S: ReadStorage>: Tracer {
    fn before_bootloader_execution(&mut self, _state: &mut Vm<S>) {}

    fn after_bootloader_execution(&mut self, _state: &mut Vm<S>) {}

    fn bootloader_hook_call(&mut self, _state: &mut Vm<S>, _hook: Hook, _hook_params: &[U256; 3]) {}

    // runs after every vm execution or transaction
    fn after_vm_run(&mut self, _vm: &mut Vm<S>, _output: ExecutionOutput) -> TracerExecutionStatus {
        TracerExecutionStatus::Continue
    }
}
