use super::traits::{BootloaderTracer, Tracer};

pub struct CircuitsTracer {}

impl Tracer for CircuitsTracer {
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

impl BootloaderTracer for CircuitsTracer {
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
