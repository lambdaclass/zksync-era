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

use super::ValidationTracer;
use crate::{
    era_vm::tracers::traits::{Tracer, VmTracer},
    interface::VmRevertReason,
};

//TODO: Implement the Tracer trait for ValidationTracer
impl<H> Tracer for ValidationTracer<H> {}

impl<S: ReadStorage, H> VmTracer<S> for ValidationTracer<H> {}
