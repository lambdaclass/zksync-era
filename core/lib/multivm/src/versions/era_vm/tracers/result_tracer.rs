use era_vm::{value::FatPointer, vm::ExecutionOutput};
use zksync_state::ReadStorage;

use super::traits::{ExecutionResult, Tracer, VmTracer};
use crate::{
    era_vm::hook::Hook,
    interface::{tracer::TracerExecutionStatus, Halt, TxRevertReason, VmRevertReason},
};

pub struct ResultTracer {
    pub result: Option<ExecutionResult>,
}

impl ResultTracer {
    pub fn new() -> Self {
        Self { result: None }
    }
}

impl Tracer for ResultTracer {}

impl<S: ReadStorage> VmTracer<S> for ResultTracer {
    fn after_vm_run(
        &mut self,
        vm: &mut super::traits::Vm<S>,
        output: era_vm::vm::ExecutionOutput,
    ) -> TracerExecutionStatus {
        let result = match output {
            ExecutionOutput::Ok(output) => Some(ExecutionResult::Success { output }),
            ExecutionOutput::Revert(output) => match TxRevertReason::parse_error(&output) {
                TxRevertReason::TxReverted(output) => Some(ExecutionResult::Revert { output }),
                TxRevertReason::Halt(reason) => Some(ExecutionResult::Halt { reason }),
            },
            ExecutionOutput::Panic => Some(ExecutionResult::Halt {
                reason: if vm.inner.execution.gas_left().unwrap() == 0 {
                    Halt::BootloaderOutOfGas
                } else {
                    Halt::VMPanic
                },
            }),
            ExecutionOutput::SuspendedOnHook { .. } => None,
        };

        // if the result is none, it means the execution has been suspended
        // and we don't want to remove the previous value
        if result.is_some() {
            self.result = result;
        }

        TracerExecutionStatus::Continue
    }

    fn bootloader_hook_call(
        &mut self,
        vm: &mut super::traits::Vm<S>,
        hook: Hook,
        hook_params: &[zksync_types::U256; 3],
    ) {
        if let Hook::PostResult = hook {
            let result = hook_params[0];
            let value = hook_params[1];
            let pointer = FatPointer::decode(value);
            assert_eq!(pointer.offset, 0);

            let return_data = vm
                .inner
                .execution
                .heaps
                .get(pointer.page)
                .unwrap()
                .read_unaligned_from_pointer(&pointer)
                .unwrap();

            self.result = Some(if result.is_zero() {
                ExecutionResult::Revert {
                    output: VmRevertReason::from(return_data.as_slice()),
                }
            } else {
                ExecutionResult::Success {
                    output: return_data,
                }
            });
        };
    }
}
