use super::traits::{BootloaderTracer, Tracer, VmTracer};

pub struct ResultTracer {}

impl Tracer for ResultTracer {
    fn before_decoding(
        &mut self,
        execution: &mut era_vm::Execution,
        state: &mut era_vm::state::VMState,
    ) {
    }

    fn after_decoding(
        &mut self,
        opcode: &era_vm::Opcode,
        execution: &mut era_vm::Execution,
        state: &mut era_vm::state::VMState,
    ) {
    }

    fn before_execution(
        &mut self,
        opcode: &era_vm::Opcode,
        execution: &mut era_vm::Execution,
        state: &mut era_vm::state::VMState,
    ) {
    }

    fn after_execution(
        &mut self,
        opcode: &era_vm::Opcode,
        execution: &mut era_vm::Execution,
        state: &mut era_vm::state::VMState,
    ) {
    }
}

impl BootloaderTracer for ResultTracer {
    fn before_bootloader_execution(
        &mut self,
        opcode: &era_vm::Opcode,
        execution: &mut era_vm::Execution,
        state: &mut era_vm::state::VMState,
    ) {
    }

    fn after_bootloader_execution(
        &mut self,
        opcode: &era_vm::Opcode,
        execution: &mut era_vm::Execution,
        state: &mut era_vm::state::VMState,
    ) {
    }
}
