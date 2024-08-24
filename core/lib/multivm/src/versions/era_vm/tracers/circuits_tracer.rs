use circuit_sequencer_api_1_5_0::{geometry_config::get_geometry_config, toolset::GeometryConfig};
use era_vm::{
    opcode::{Opcode, Variant},
    statistics::VmStatistics,
};
use zkevm_opcode_defs::{LogOpcode, UMAOpcode};
use zksync_state::ReadStorage;
use zksync_types::circuit::CircuitStatistic;

use super::traits::{Tracer, VmTracer};

const GEOMETRY_CONFIG: GeometryConfig = get_geometry_config();

// "Rich addressing" opcodes are opcodes that can write their return value/read the input onto the stack
// and so take 1-2 RAM permutations more than an average opcode.
// In the worst case, a rich addressing may take 3 ram permutations
// (1 for reading the opcode, 1 for writing input value, 1 for writing output value).
pub(crate) const RICH_ADDRESSING_OPCODE_RAM_CYCLES: u32 = 3;

pub(crate) const AVERAGE_OPCODE_RAM_CYCLES: u32 = 1;

pub(crate) const STORAGE_READ_RAM_CYCLES: u32 = 1;
pub(crate) const STORAGE_READ_LOG_DEMUXER_CYCLES: u32 = 1;
pub(crate) const STORAGE_READ_STORAGE_SORTER_CYCLES: u32 = 1;

pub(crate) const TRANSIENT_STORAGE_READ_RAM_CYCLES: u32 = 1;
pub(crate) const TRANSIENT_STORAGE_READ_LOG_DEMUXER_CYCLES: u32 = 1;
pub(crate) const TRANSIENT_STORAGE_READ_TRANSIENT_STORAGE_CHECKER_CYCLES: u32 = 1;

pub(crate) const EVENT_RAM_CYCLES: u32 = 1;
pub(crate) const EVENT_LOG_DEMUXER_CYCLES: u32 = 2;
pub(crate) const EVENT_EVENTS_SORTER_CYCLES: u32 = 2;

pub(crate) const STORAGE_WRITE_RAM_CYCLES: u32 = 1;
pub(crate) const STORAGE_WRITE_LOG_DEMUXER_CYCLES: u32 = 2;
pub(crate) const STORAGE_WRITE_STORAGE_SORTER_CYCLES: u32 = 2;

pub(crate) const TRANSIENT_STORAGE_WRITE_RAM_CYCLES: u32 = 1;
pub(crate) const TRANSIENT_STORAGE_WRITE_LOG_DEMUXER_CYCLES: u32 = 2;
pub(crate) const TRANSIENT_STORAGE_WRITE_TRANSIENT_STORAGE_CHECKER_CYCLES: u32 = 2;

pub(crate) const FAR_CALL_RAM_CYCLES: u32 = 1;
pub(crate) const FAR_CALL_STORAGE_SORTER_CYCLES: u32 = 1;
pub(crate) const FAR_CALL_CODE_DECOMMITTER_SORTER_CYCLES: u32 = 1;
pub(crate) const FAR_CALL_LOG_DEMUXER_CYCLES: u32 = 1;

// 5 RAM permutations, because: 1 to read opcode + 2 reads + 2 writes.
// 2 reads and 2 writes are needed because unaligned access is implemented with
// aligned queries.
pub(crate) const UMA_WRITE_RAM_CYCLES: u32 = 5;

// 3 RAM permutations, because: 1 to read opcode + 2 reads.
// 2 reads are needed because unaligned access is implemented with aligned queries.
pub(crate) const UMA_READ_RAM_CYCLES: u32 = 3;

pub(crate) const PRECOMPILE_RAM_CYCLES: u32 = 1;
pub(crate) const PRECOMPILE_LOG_DEMUXER_CYCLES: u32 = 1;

pub(crate) const LOG_DECOMMIT_RAM_CYCLES: u32 = 1;
pub(crate) const LOG_DECOMMIT_DECOMMITTER_SORTER_CYCLES: u32 = 1;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct CircuitsTracer {
    rich_addressing_opcodes: u32,
    average_opcodes: u32,
    storage_reads: u32,
    storage_writes: u32,
    transient_storage_reads: u32,
    transient_storage_writes: u32,
    events: u32,
    precompile_calls: u32,
    decommits: u32,
    far_calls: u32,
    heap_writes: u32,
    heap_reads: u32,
}

impl CircuitsTracer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn circuit_statistics(&self, vm_statistics: &VmStatistics) -> CircuitStatistic {
        let VmStatistics {
            code_decommitter_cycles,
            ecrecover_cycles,
            keccak256_cycles,
            secp255r1_verify_cycles: secp256k1_verify_cycles,
            sha256_cycles,
            storage_application_cycles,
            ..
        } = *vm_statistics;

        CircuitStatistic {
            main_vm: (self.rich_addressing_opcodes
                + self.average_opcodes
                + self.storage_reads
                + self.storage_writes
                + self.transient_storage_reads
                + self.transient_storage_writes
                + self.events
                + self.precompile_calls
                + self.decommits
                + self.far_calls
                + self.heap_writes
                + self.heap_reads) as f32
                / GEOMETRY_CONFIG.cycles_per_vm_snapshot as f32,
            ram_permutation: (self.rich_addressing_opcodes * RICH_ADDRESSING_OPCODE_RAM_CYCLES
                + self.average_opcodes * AVERAGE_OPCODE_RAM_CYCLES
                + self.storage_reads * STORAGE_READ_RAM_CYCLES
                + self.storage_writes * STORAGE_WRITE_RAM_CYCLES
                + self.transient_storage_reads * TRANSIENT_STORAGE_READ_RAM_CYCLES
                + self.transient_storage_writes * TRANSIENT_STORAGE_WRITE_RAM_CYCLES
                + self.events * EVENT_RAM_CYCLES
                + self.precompile_calls * PRECOMPILE_RAM_CYCLES
                + self.decommits * LOG_DECOMMIT_RAM_CYCLES
                + self.far_calls * FAR_CALL_RAM_CYCLES
                + self.heap_writes * UMA_WRITE_RAM_CYCLES
                + self.heap_reads * UMA_READ_RAM_CYCLES) as f32
                / GEOMETRY_CONFIG.cycles_per_ram_permutation as f32,
            storage_application: storage_application_cycles as f32
                / GEOMETRY_CONFIG.cycles_per_storage_application as f32,
            storage_sorter: (self.storage_reads * STORAGE_READ_STORAGE_SORTER_CYCLES
                + self.storage_writes * STORAGE_WRITE_STORAGE_SORTER_CYCLES
                + self.transient_storage_reads
                    * TRANSIENT_STORAGE_READ_TRANSIENT_STORAGE_CHECKER_CYCLES
                + self.transient_storage_writes
                    * TRANSIENT_STORAGE_WRITE_TRANSIENT_STORAGE_CHECKER_CYCLES
                + self.far_calls * FAR_CALL_STORAGE_SORTER_CYCLES)
                as f32
                / GEOMETRY_CONFIG.cycles_per_storage_sorter as f32,
            code_decommitter: code_decommitter_cycles as f32
                / GEOMETRY_CONFIG.cycles_per_code_decommitter as f32,
            code_decommitter_sorter: (self.decommits * LOG_DECOMMIT_DECOMMITTER_SORTER_CYCLES
                + self.far_calls * FAR_CALL_CODE_DECOMMITTER_SORTER_CYCLES)
                as f32
                / GEOMETRY_CONFIG.cycles_code_decommitter_sorter as f32,
            log_demuxer: (self.storage_reads * STORAGE_READ_LOG_DEMUXER_CYCLES
                + self.storage_writes * STORAGE_WRITE_LOG_DEMUXER_CYCLES
                + self.transient_storage_reads * TRANSIENT_STORAGE_READ_LOG_DEMUXER_CYCLES
                + self.transient_storage_writes * TRANSIENT_STORAGE_WRITE_LOG_DEMUXER_CYCLES
                + self.events * EVENT_LOG_DEMUXER_CYCLES
                + self.precompile_calls * PRECOMPILE_LOG_DEMUXER_CYCLES
                + self.far_calls * FAR_CALL_LOG_DEMUXER_CYCLES) as f32
                / GEOMETRY_CONFIG.cycles_per_log_demuxer as f32,
            events_sorter: (self.events * EVENT_EVENTS_SORTER_CYCLES) as f32
                / GEOMETRY_CONFIG.cycles_per_events_or_l1_messages_sorter as f32,
            keccak256: keccak256_cycles as f32
                / GEOMETRY_CONFIG.cycles_per_keccak256_circuit as f32,
            ecrecover: ecrecover_cycles as f32
                / GEOMETRY_CONFIG.cycles_per_ecrecover_circuit as f32,
            sha256: sha256_cycles as f32 / GEOMETRY_CONFIG.cycles_per_sha256_circuit as f32,
            secp256k1_verify: secp256k1_verify_cycles as f32
                / GEOMETRY_CONFIG.cycles_per_secp256r1_verify_circuit as f32,
            transient_storage_checker: (self.transient_storage_reads
                * TRANSIENT_STORAGE_READ_TRANSIENT_STORAGE_CHECKER_CYCLES
                + self.transient_storage_writes
                    * TRANSIENT_STORAGE_WRITE_TRANSIENT_STORAGE_CHECKER_CYCLES)
                as f32
                / GEOMETRY_CONFIG.cycles_per_transient_storage_sorter as f32,
        }
    }
}

impl Tracer for CircuitsTracer {
    fn after_execution(
        &mut self,
        opcode: &Opcode,
        _execution: &mut era_vm::Execution,
        _state: &mut era_vm::state::VMState,
    ) {
        match opcode.variant {
            Variant::Nop(_)
            | Variant::Add(_)
            | Variant::Sub(_)
            | Variant::Mul(_)
            | Variant::Div(_)
            | Variant::Jump(_)
            | Variant::Shift(_)
            | Variant::Binop(_)
            | Variant::Ptr(_) => {
                self.rich_addressing_opcodes += 1;
            }
            Variant::Context(_) | Variant::Ret(_) | Variant::NearCall(_) => {
                self.average_opcodes += 1;
            }
            Variant::Log(LogOpcode::StorageRead) => {
                self.storage_reads += 1;
            }
            Variant::Log(LogOpcode::TransientStorageRead) => {
                self.transient_storage_reads += 1;
            }
            Variant::Log(LogOpcode::StorageWrite) => {
                self.storage_writes += 1;
            }
            Variant::Log(LogOpcode::TransientStorageWrite) => {
                self.transient_storage_writes += 1;
            }
            Variant::Log(LogOpcode::ToL1Message) | Variant::Log(LogOpcode::Event) => {
                self.events += 1;
            }
            Variant::Log(LogOpcode::PrecompileCall) => {
                self.precompile_calls += 1;
            }
            Variant::Log(LogOpcode::Decommit) => {
                self.decommits += 1;
            }
            Variant::FarCall(_) => {
                self.far_calls += 1;
            }
            Variant::UMA(
                UMAOpcode::AuxHeapWrite | UMAOpcode::HeapWrite | UMAOpcode::StaticMemoryWrite,
            ) => {
                self.heap_writes += 1;
            }
            Variant::UMA(
                UMAOpcode::AuxHeapRead
                | UMAOpcode::HeapRead
                | UMAOpcode::FatPointerRead
                | UMAOpcode::StaticMemoryRead,
            ) => {
                self.heap_reads += 1;
            }
            Variant::Invalid(_) => {}
        }
    }
}

impl<S: ReadStorage> VmTracer<S> for CircuitsTracer {}
