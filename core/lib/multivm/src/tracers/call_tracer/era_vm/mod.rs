use era_vm::{
    opcode::{RetOpcode, Variant},
    value::FatPointer,
    Execution, Opcode,
};
use zksync_state::ReadStorage;
use zksync_types::{
    vm_trace::{Call, CallType},
    zk_evm_types::FarCallOpcode,
    CONTRACT_DEPLOYER_ADDRESS, U256,
};

use super::CallTracer;
use crate::{
    era_vm::tracers::traits::{Tracer, VmTracer},
    interface::VmRevertReason,
};

impl Tracer for CallTracer {
    fn after_execution(
        &mut self,
        opcode: &Opcode,
        execution: &mut era_vm::Execution,
        _state: &mut era_vm::state::VMState,
    ) {
        match opcode.variant {
            Variant::NearCall(_) => {
                self.increase_near_call_count();
            }
            Variant::FarCall(far_call) => {
                // We use parent gas for properly calculating gas used in the trace.
                let current_ergs = execution.gas_left().unwrap();
                let parent_gas = execution
                    .running_contexts
                    .last()
                    .map(|call| call.frame.gas_left.0.saturating_add(current_ergs))
                    .unwrap_or(current_ergs) as u64;

                // we need to to this cast because `Call` uses another library
                let far_call_variant = match far_call as u8 {
                    0 => FarCallOpcode::Normal,
                    1 => FarCallOpcode::Delegate,
                    2 => FarCallOpcode::Mimic,
                    _ => unreachable!(),
                };

                let mut current_call = Call {
                    r#type: CallType::Call(far_call_variant),
                    gas: 0,
                    parent_gas: parent_gas as u64,
                    ..Default::default()
                };

                self.handle_far_call_op_code_era(execution, &mut current_call);
                self.push_call_and_update_stats(current_call, 0);
            }
            Variant::Ret(ret_code) => {
                self.handle_ret_op_code_era(execution, ret_code);
            }
            _ => {}
        };
    }
}

impl<S: ReadStorage> VmTracer<S> for CallTracer {
    fn after_bootloader_execution(&mut self, _state: &mut crate::era_vm::vm::Vm<S>) {
        self.store_result();
    }
}

impl CallTracer {
    fn handle_far_call_op_code_era(&mut self, execution: &Execution, current_call: &mut Call) {
        // since this is a far_call, the current_context represents the current frame
        let current = execution.current_context().unwrap();
        // All calls from the actual users are mimic calls,
        // so we need to check that the previous call was to the deployer.
        // Actually it's a call of the constructor.
        // And at this stage caller is user and callee is deployed contract.
        let call_type = if let CallType::Call(far_call) = current_call.r#type {
            if matches!(far_call, FarCallOpcode::Mimic) {
                let previous_caller = execution
                    .running_contexts
                    .first()
                    .map(|call| call.caller)
                    // Actually it's safe to just unwrap here, because we have at least one call in the stack
                    // But i want to be sure that we will not have any problems in the future
                    .unwrap_or(current.caller);
                if previous_caller == CONTRACT_DEPLOYER_ADDRESS {
                    CallType::Create
                } else {
                    CallType::Call(far_call)
                }
            } else {
                CallType::Call(far_call)
            }
        } else {
            unreachable!()
        };
        let calldata = if current.heap_id == 0 || current.frame.gas_left.0 == 0 {
            vec![]
        } else {
            let packed_abi = execution.get_register(1);
            assert!(packed_abi.is_pointer);
            let pointer = FatPointer::decode(packed_abi.value);
            execution
                .heaps
                .get(pointer.page)
                .unwrap()
                .read_unaligned_from_pointer(&pointer)
                .unwrap_or_default()
        };

        current_call.input = calldata;
        current_call.r#type = call_type;
        current_call.from = current.caller;
        current_call.to = current.contract_address;
        current_call.value = U256::from(current.context_u128);
        current_call.gas = current.frame.gas_left.0 as u64;
    }

    fn save_output_era(
        &mut self,
        execution: &Execution,
        ret_opcode: RetOpcode,
        current_call: &mut Call,
    ) {
        let fat_data_pointer = execution.get_register(1);

        // if `fat_data_pointer` is not a pointer then there is no output
        let output = if fat_data_pointer.is_pointer {
            let fat_data_pointer = FatPointer::decode(fat_data_pointer.value);
            if fat_data_pointer.len == 0 && fat_data_pointer.offset == 0 {
                Some(
                    execution
                        .heaps
                        .get(fat_data_pointer.page)
                        .unwrap()
                        .read_unaligned_from_pointer(&fat_data_pointer)
                        .unwrap(),
                )
            } else {
                None
            }
        } else {
            None
        };

        match ret_opcode {
            RetOpcode::Ok => {
                current_call.output = output.unwrap_or_default();
            }
            RetOpcode::Revert => {
                if let Some(output) = output {
                    current_call.revert_reason =
                        Some(VmRevertReason::from(output.as_slice()).to_string());
                } else {
                    current_call.revert_reason = Some("Unknown revert reason".to_string());
                }
            }
            RetOpcode::Panic => {
                current_call.error = Some("Panic".to_string());
            }
        }
    }

    fn handle_ret_op_code_era(&mut self, execution: &Execution, ret_opcode: RetOpcode) {
        let Some(mut current_call) = self.stack.pop() else {
            return;
        };

        if current_call.near_calls_after > 0 {
            current_call.near_calls_after -= 1;
            self.push_call_and_update_stats(current_call.farcall, current_call.near_calls_after);
            return;
        }

        current_call.farcall.gas_used = current_call
            .farcall
            .parent_gas
            .saturating_sub(execution.gas_left().unwrap() as u64);

        self.save_output_era(execution, ret_opcode, &mut current_call.farcall);

        // If there is a parent call, push the current call to it
        // Otherwise, push the current call to the stack, because it's the top level call
        if let Some(parent_call) = self.stack.last_mut() {
            parent_call.farcall.calls.push(current_call.farcall);
        } else {
            self.push_call_and_update_stats(current_call.farcall, current_call.near_calls_after);
        }
    }
}
