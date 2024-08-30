use zksync_state::ReadStorage;
use zksync_types::U256;
use zksync_utils::u256_to_h256;

use super::traits::{Tracer, VmTracer};
use crate::era_vm::hook::Hook;

pub struct DebugTracer {}

impl Tracer for DebugTracer {}

impl DebugTracer {
    pub fn new() -> Self {
        Self {}
    }
}

impl<S: ReadStorage + 'static> VmTracer<S> for DebugTracer {
    fn bootloader_hook_call(
        &mut self,
        vm: &mut super::traits::Vm<S>,
        hook: crate::era_vm::hook::Hook,
        hook_params: &[U256; 3],
    ) {
        match hook {
            Hook::DebugLog => {
                let msg = u256_to_h256(hook_params[0]).as_bytes().to_vec();
                let data = u256_to_h256(hook_params[1]).as_bytes().to_vec();

                let msg = String::from_utf8(msg).expect("Invalid debug message");
                let data = U256::from_big_endian(&data);

                // For long data, it is better to use hex-encoding for greater readability
                let data_str = if data > U256::from(u64::max_value()) {
                    let mut bytes = [0u8; 32];
                    data.to_big_endian(&mut bytes);
                    format!("0x{}", hex::encode(bytes))
                } else {
                    data.to_string()
                };

                println!("======== BOOTLOADER DEBUG LOG ========");
                println!("MSG: {:?}", msg);
                println!("DATA: {}", data_str);
            }
            Hook::AccountValidationEntered => {
                // println!("ACCOUNT VALIDATION ENTERED");
            }
            Hook::ValidationStepEnded => {
                // println!("VALIDATION STEP ENDED");
            }
            Hook::AccountValidationExited => {
                // println!("ACCOUNT VALIDATION EXITED");
            }
            Hook::DebugReturnData => {
                // println!("DEBUG RETURN DATA");
            }
            Hook::NearCallCatch => {
                // println!("NOTIFY ABOUT NEAR CALL CATCH");
            }
            _ => {}
        };
    }
}
