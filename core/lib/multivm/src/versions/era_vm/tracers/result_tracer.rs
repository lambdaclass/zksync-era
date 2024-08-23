use era_vm::value::FatPointer;
use zksync_state::ReadStorage;

use super::traits::{ExecutionResult, Tracer, VmTracer};
use crate::{era_vm::hook::Hook, interface::VmRevertReason};

pub struct ResultTracer {
    pub last_tx_result: Option<ExecutionResult>,
}

impl ResultTracer {
    pub fn new() -> Self {
        Self {
            last_tx_result: None,
        }
    }
}

impl Tracer for ResultTracer {}

impl<S: ReadStorage> VmTracer<S> for ResultTracer {
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

            self.last_tx_result = Some(if result.is_zero() {
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
