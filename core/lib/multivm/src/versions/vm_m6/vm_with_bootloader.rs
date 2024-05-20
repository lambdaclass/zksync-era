use std::collections::HashMap;

use circuit_sequencer_api_1_3_3::INITIAL_MONOTONIC_CYCLE_COUNTER;
use zk_evm_1_3_1::{
    abstractions::{MAX_HEAP_PAGE_SIZE_IN_WORDS, MAX_MEMORY_BYTES},
    aux_structures::{MemoryPage, Timestamp},
    block_properties::BlockProperties,
    vm_state::{CallStackEntry, PrimitiveValue, VmState},
    zkevm_opcode_defs::{
        system_params::INITIAL_FRAME_FORMAL_EH_LOCATION, FatPointer, BOOTLOADER_BASE_PAGE,
        BOOTLOADER_CALLDATA_PAGE, STARTING_BASE_PAGE, STARTING_TIMESTAMP,
    },
};
use zksync_contracts::BaseSystemContracts;
use zksync_system_constants::MAX_L2_TX_GAS_LIMIT;
use zksync_types::{
    fee_model::L1PeggedBatchFeeModelInput, Address, Transaction, BOOTLOADER_ADDRESS,
    L1_GAS_PER_PUBDATA_BYTE, MAX_NEW_FACTORY_DEPS, U256,
};
use zksync_utils::{
    address_to_u256,
    bytecode::{compress_bytecode, hash_bytecode, CompressedBytecodeInfo},
    bytes_to_be_words, ceil_div_u256, h256_to_u256,
};

use crate::{
    vm_latest::L1BatchEnv,
    vm_m6::{
        bootloader_state::BootloaderState,
        history_recorder::HistoryMode,
        storage::Storage,
        transaction_data::{TransactionData, L1_TX_TYPE},
        utils::{
            code_page_candidate_from_base, heap_page_from_base, BLOCK_GAS_LIMIT, INITIAL_BASE_PAGE,
        },
        vm_instance::{MultiVMSubversion, ZkSyncVmState},
        OracleTools, VmInstance,
    },
};

// TODO (SMA-1703): move these to config and make them programmatically generable.
// fill these values in the similar fashion as other overhead-related constants
pub const BLOCK_OVERHEAD_GAS: u32 = 1200000;
pub const BLOCK_OVERHEAD_L1_GAS: u32 = 1000000;
pub const BLOCK_OVERHEAD_PUBDATA: u32 = BLOCK_OVERHEAD_L1_GAS / L1_GAS_PER_PUBDATA_BYTE;

pub const MAX_BLOCK_MULTIINSTANCE_GAS_LIMIT: u32 = 300_000_000;

/// `BlockContext` is a structure that contains parameters for
/// a block that are used as input for the bootloader and not the VM per se.
///
/// These values are generally unique for each block (the exception is the operator's address).
#[derive(Clone, Debug, Copy)]
pub struct BlockContext {
    pub block_number: u32,
    pub block_timestamp: u64,
    pub operator_address: Address,
    pub l1_gas_price: U256,
    pub fair_l2_gas_price: U256,
}

impl BlockContext {
    pub fn block_gas_price_per_pubdata(&self) -> U256 {
        derive_base_fee_and_gas_per_pubdata(L1PeggedBatchFeeModelInput {
            l1_gas_price: self.l1_gas_price,
            fair_l2_gas_price: self.fair_l2_gas_price,
        })
        .1
    }
}

/// Besides the raw values from the `BlockContext`, contains the values that are to be derived
/// from the other values
#[derive(Debug, Copy, Clone)]
pub struct DerivedBlockContext {
    pub context: BlockContext,
    pub base_fee: U256,
}

pub(crate) fn eth_price_per_pubdata_byte(l1_gas_price: U256) -> U256 {
    // This value will typically be a lot less than u64
    // unless the gas price on L1 goes beyond tens of millions of gwei
    l1_gas_price * U256::from(L1_GAS_PER_PUBDATA_BYTE)
}

pub(crate) fn base_fee_to_gas_per_pubdata(l1_gas_price: U256, base_fee: U256) -> U256 {
    let eth_price_per_pubdata_byte = eth_price_per_pubdata_byte(l1_gas_price);

    ceil_div_u256(eth_price_per_pubdata_byte, base_fee)
}

pub(crate) fn derive_base_fee_and_gas_per_pubdata(
    fee_input: L1PeggedBatchFeeModelInput,
) -> (U256, U256) {
    let L1PeggedBatchFeeModelInput {
        l1_gas_price,
        fair_l2_gas_price,
    } = fee_input;

    let eth_price_per_pubdata_byte = eth_price_per_pubdata_byte(l1_gas_price);

    // The `baseFee` is set in such a way that it is always possible for a transaction to
    // publish enough public data while compensating us for it.
    let base_fee = std::cmp::max(
        fair_l2_gas_price,
        ceil_div_u256(
            eth_price_per_pubdata_byte,
            U256::from(MAX_GAS_PER_PUBDATA_BYTE),
        ),
    );

    (
        base_fee,
        base_fee_to_gas_per_pubdata(fee_input.l1_gas_price, base_fee),
    )
}

pub(crate) fn get_batch_base_fee(l1_batch_env: &L1BatchEnv) -> U256 {
    if let Some(base_fee) = l1_batch_env.enforced_base_fee {
        return base_fee;
    }
    let (base_fee, _) =
        derive_base_fee_and_gas_per_pubdata(l1_batch_env.fee_input.into_l1_pegged());
    base_fee
}

impl From<BlockContext> for DerivedBlockContext {
    fn from(context: BlockContext) -> Self {
        let base_fee = derive_base_fee_and_gas_per_pubdata(L1PeggedBatchFeeModelInput {
            l1_gas_price: context.l1_gas_price,
            fair_l2_gas_price: context.fair_l2_gas_price,
        })
        .0;

        DerivedBlockContext { context, base_fee }
    }
}

/// The size of the bootloader memory in bytes which is used by the protocol.
/// While the maximal possible size is a lot higher, we restrict ourselves to a certain limit to reduce
/// the requirements on RAM.
pub(crate) const USED_BOOTLOADER_MEMORY_BYTES: usize = 1 << 24;
pub(crate) const USED_BOOTLOADER_MEMORY_WORDS: usize = USED_BOOTLOADER_MEMORY_BYTES / 32;

// This the number of pubdata such that it should be always possible to publish
// from a single transaction. Note, that these pubdata bytes include only bytes that are
// to be published inside the body of transaction (i.e. excluding of factory deps).
pub(crate) const GUARANTEED_PUBDATA_PER_L1_BATCH: u64 = 4000;

// The users should always be able to provide `MAX_GAS_PER_PUBDATA_BYTE` gas per pubdata in their
// transactions so that they are able to send at least `GUARANTEED_PUBDATA_PER_L1_BATCH` bytes per
// transaction.
pub(crate) const MAX_GAS_PER_PUBDATA_BYTE: u64 =
    MAX_L2_TX_GAS_LIMIT / GUARANTEED_PUBDATA_PER_L1_BATCH;

// The maximal number of transactions in a single batch
pub(crate) const MAX_TXS_IN_BLOCK: usize = 1024;

// The first 32 slots are reserved for debugging purposes
pub const DEBUG_SLOTS_OFFSET: usize = 8;
pub const DEBUG_FIRST_SLOTS: usize = 32;
// The next 33 slots are reserved for dealing with the paymaster context (1 slot for storing length + 32 slots for storing the actual context).
pub const PAYMASTER_CONTEXT_SLOTS: usize = 32 + 1;
// The next PAYMASTER_CONTEXT_SLOTS + 7 slots free slots are needed before each tx, so that the
// postOp operation could be encoded correctly.
pub const MAX_POSTOP_SLOTS: usize = PAYMASTER_CONTEXT_SLOTS + 7;

// Slots used to store the current L2 transaction's hash and the hash recommended
// to be used for signing the transaction's content.
const CURRENT_L2_TX_HASHES_SLOTS: usize = 2;

// Slots used to store the calldata for the KnownCodesStorage to mark new factory
// dependencies as known ones. Besides the slots for the new factory dependencies themselves
// another 4 slots are needed for: selector, marker of whether the user should pay for the pubdata,
// the offset for the encoding of the array as well as the length of the array.
pub const NEW_FACTORY_DEPS_RESERVED_SLOTS: usize = MAX_NEW_FACTORY_DEPS + 4;

// The operator can provide for each transaction the proposed minimal refund
pub const OPERATOR_REFUNDS_SLOTS: usize = MAX_TXS_IN_BLOCK;

pub const OPERATOR_REFUNDS_OFFSET: usize = DEBUG_SLOTS_OFFSET
    + DEBUG_FIRST_SLOTS
    + PAYMASTER_CONTEXT_SLOTS
    + CURRENT_L2_TX_HASHES_SLOTS
    + NEW_FACTORY_DEPS_RESERVED_SLOTS;

pub const TX_OVERHEAD_OFFSET: usize = OPERATOR_REFUNDS_OFFSET + OPERATOR_REFUNDS_SLOTS;
pub const TX_OVERHEAD_SLOTS: usize = MAX_TXS_IN_BLOCK;

pub const TX_TRUSTED_GAS_LIMIT_OFFSET: usize = TX_OVERHEAD_OFFSET + TX_OVERHEAD_SLOTS;
pub const TX_TRUSTED_GAS_LIMIT_SLOTS: usize = MAX_TXS_IN_BLOCK;

pub const COMPRESSED_BYTECODES_OFFSET: usize =
    TX_TRUSTED_GAS_LIMIT_OFFSET + TX_TRUSTED_GAS_LIMIT_SLOTS;
pub const COMPRESSED_BYTECODES_SLOTS: usize = 32768;

pub const BOOTLOADER_TX_DESCRIPTION_OFFSET: usize =
    COMPRESSED_BYTECODES_OFFSET + COMPRESSED_BYTECODES_SLOTS;

// The size of the bootloader memory dedicated to the encodings of transactions
pub(crate) const BOOTLOADER_TX_ENCODING_SPACE: u32 =
    (MAX_HEAP_PAGE_SIZE_IN_WORDS - TX_DESCRIPTION_OFFSET - MAX_TXS_IN_BLOCK) as u32;

// Size of the bootloader tx description in words
pub const BOOTLOADER_TX_DESCRIPTION_SIZE: usize = 2;

// The actual descriptions of transactions should start after the minor descriptions and a MAX_POSTOP_SLOTS
// free slots to allow postOp encoding.
pub const TX_DESCRIPTION_OFFSET: usize = BOOTLOADER_TX_DESCRIPTION_OFFSET
    + BOOTLOADER_TX_DESCRIPTION_SIZE * MAX_TXS_IN_BLOCK
    + MAX_POSTOP_SLOTS;

pub const TX_GAS_LIMIT_OFFSET: usize = 4;

pub(crate) const BOOTLOADER_HEAP_PAGE: u32 = heap_page_from_base(MemoryPage(INITIAL_BASE_PAGE)).0;
const BOOTLOADER_CODE_PAGE: u32 = code_page_candidate_from_base(MemoryPage(INITIAL_BASE_PAGE)).0;

/// Enum denoting the *in-server* execution mode for the bootloader transactions.
///
/// If `EthCall` mode is chosen, the bootloader will use `mimicCall` opcode
/// to simulate the call instead of using the standard `execute` method of account.
/// This is needed to be able to behave equivalently to Ethereum without much overhead for custom account builders.
/// With `VerifyExecute` mode, transaction will be executed normally.
/// With `EstimateFee`, the bootloader will be used that has the same behavior
/// as the full `VerifyExecute` block, but errors in the account validation will be ignored.
#[derive(Debug, Clone, Copy)]
pub enum TxExecutionMode {
    VerifyExecute,
    EstimateFee {
        missed_storage_invocation_limit: usize,
    },
    EthCall {
        missed_storage_invocation_limit: usize,
    },
}

impl TxExecutionMode {
    pub fn invocation_limit(&self) -> usize {
        match self {
            Self::VerifyExecute => usize::MAX,
            TxExecutionMode::EstimateFee {
                missed_storage_invocation_limit,
            } => *missed_storage_invocation_limit,
            TxExecutionMode::EthCall {
                missed_storage_invocation_limit,
            } => *missed_storage_invocation_limit,
        }
    }

    pub fn set_invocation_limit(&mut self, limit: usize) {
        match self {
            Self::VerifyExecute => {}
            TxExecutionMode::EstimateFee {
                missed_storage_invocation_limit,
            } => *missed_storage_invocation_limit = limit,
            TxExecutionMode::EthCall {
                missed_storage_invocation_limit,
            } => *missed_storage_invocation_limit = limit,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootloaderJobType {
    TransactionExecution,
    BlockPostprocessing,
}

impl Default for TxExecutionMode {
    fn default() -> Self {
        Self::VerifyExecute
    }
}

pub fn init_vm<S: Storage, H: HistoryMode>(
    vm_subversion: MultiVMSubversion,
    oracle_tools: OracleTools<false, S, H>,
    block_context: BlockContextMode,
    block_properties: BlockProperties,
    execution_mode: TxExecutionMode,
    base_system_contract: &BaseSystemContracts,
) -> VmInstance<S, H> {
    init_vm_with_gas_limit(
        vm_subversion,
        oracle_tools,
        block_context,
        block_properties,
        execution_mode,
        base_system_contract,
        BLOCK_GAS_LIMIT,
    )
}

pub fn init_vm_with_gas_limit<S: Storage, H: HistoryMode>(
    vm_subversion: MultiVMSubversion,
    oracle_tools: OracleTools<false, S, H>,
    block_context: BlockContextMode,
    block_properties: BlockProperties,
    execution_mode: TxExecutionMode,
    base_system_contract: &BaseSystemContracts,
    gas_limit: u32,
) -> VmInstance<S, H> {
    init_vm_inner(
        vm_subversion,
        oracle_tools,
        block_context,
        block_properties,
        gas_limit,
        base_system_contract,
        execution_mode,
    )
}

#[derive(Debug, Clone, Copy)]
// The `block.number` / `block.timestamp` data are stored in the `CONTEXT_SYSTEM_CONTRACT`.
// The bootloader can support execution in two modes:
// - `NewBlock` when the new block is created. It is enforced that the block.number is incremented by 1
//   and the timestamp is non-decreasing. Also, the L2->L1 message used to verify the correctness of the previous root hash is sent.
//   This is the mode that should be used in the state keeper.
// - `OverrideCurrent` when we need to provide custom `block.number` and `block.timestamp`. ONLY to be used in testing / `ethCalls`.
pub enum BlockContextMode {
    NewBlock(DerivedBlockContext, U256),
    OverrideCurrent(DerivedBlockContext),
}

impl BlockContextMode {
    const OPERATOR_ADDRESS_SLOT: usize = 0;
    const PREV_BLOCK_HASH_SLOT: usize = 1;
    const NEW_BLOCK_TIMESTAMP_SLOT: usize = 2;
    const NEW_BLOCK_NUMBER_SLOT: usize = 3;
    const L1_GAS_PRICE_SLOT: usize = 4;
    const FAIR_L2_GAS_PRICE_SLOT: usize = 5;
    const EXPECTED_BASE_FEE_SLOT: usize = 6;
    const SHOULD_SET_NEW_BLOCK_SLOT: usize = 7;

    // Returns the previous block hash and timestamp fields that should be used by the bootloader.
    // If the timestamp is 0, then the bootloader will not attempt to start a new block
    // and will continue using the existing block properties.
    fn bootloader_block_params(&self) -> Vec<(usize, U256)> {
        let DerivedBlockContext { context, base_fee } = self.inner_block_context();

        let mut base_params: HashMap<usize, U256> = vec![
            (
                Self::OPERATOR_ADDRESS_SLOT,
                address_to_u256(&context.operator_address),
            ),
            (Self::PREV_BLOCK_HASH_SLOT, Default::default()),
            (
                Self::NEW_BLOCK_TIMESTAMP_SLOT,
                U256::from(context.block_timestamp),
            ),
            (
                Self::NEW_BLOCK_NUMBER_SLOT,
                U256::from(context.block_number),
            ),
            (Self::L1_GAS_PRICE_SLOT, context.l1_gas_price),
            (Self::FAIR_L2_GAS_PRICE_SLOT, context.fair_l2_gas_price),
            (Self::EXPECTED_BASE_FEE_SLOT, base_fee),
            (Self::SHOULD_SET_NEW_BLOCK_SLOT, U256::from(0u32)),
        ]
        .into_iter()
        .collect();

        match *self {
            BlockContextMode::OverrideCurrent(_) => base_params.into_iter().collect(),
            BlockContextMode::NewBlock(_, prev_block_hash) => {
                base_params.insert(Self::PREV_BLOCK_HASH_SLOT, prev_block_hash);
                base_params.insert(Self::SHOULD_SET_NEW_BLOCK_SLOT, U256::from(1u32));
                base_params.into_iter().collect()
            }
        }
    }

    pub fn inner_block_context(&self) -> DerivedBlockContext {
        match *self {
            BlockContextMode::OverrideCurrent(props) => props,
            BlockContextMode::NewBlock(props, _) => props,
        }
    }

    pub fn timestamp(&self) -> u64 {
        self.inner_block_context().context.block_timestamp
    }
}

// This method accepts a custom bootloader code.
// It should be used only in tests.
pub fn init_vm_inner<S: Storage, H: HistoryMode>(
    vm_subversion: MultiVMSubversion,
    mut oracle_tools: OracleTools<false, S, H>,
    block_context: BlockContextMode,
    block_properties: BlockProperties,
    gas_limit: u32,
    base_system_contract: &BaseSystemContracts,
    execution_mode: TxExecutionMode,
) -> VmInstance<S, H> {
    oracle_tools.decommittment_processor.populate(
        vec![(
            h256_to_u256(base_system_contract.default_aa.hash),
            base_system_contract.default_aa.code.clone(),
        )],
        Timestamp(0),
    );

    oracle_tools.memory.populate(
        vec![(
            BOOTLOADER_CODE_PAGE,
            base_system_contract.bootloader.code.clone(),
        )],
        Timestamp(0),
    );

    oracle_tools.memory.populate_page(
        BOOTLOADER_HEAP_PAGE as usize,
        bootloader_initial_memory(&block_context),
        Timestamp(0),
    );

    let state = get_default_local_state(oracle_tools, block_properties, gas_limit);

    VmInstance {
        gas_limit,
        state,
        execution_mode,
        block_context: block_context.inner_block_context(),
        bootloader_state: BootloaderState::new(),
        snapshots: Vec::new(),
        vm_subversion,
    }
}

fn bootloader_initial_memory(block_properties: &BlockContextMode) -> Vec<(usize, U256)> {
    block_properties.bootloader_block_params()
}

pub fn get_bootloader_memory(
    vm_subversion: MultiVMSubversion,
    txs: Vec<TransactionData>,
    predefined_refunds: Vec<u32>,
    predefined_compressed_bytecodes: Vec<Vec<CompressedBytecodeInfo>>,
    execution_mode: TxExecutionMode,
    block_context: BlockContextMode,
) -> Vec<(usize, U256)> {
    match vm_subversion {
        MultiVMSubversion::V1 => get_bootloader_memory_v1(
            txs,
            predefined_refunds,
            predefined_compressed_bytecodes,
            execution_mode,
            block_context,
        ),
        MultiVMSubversion::V2 => get_bootloader_memory_v2(
            txs,
            predefined_refunds,
            predefined_compressed_bytecodes,
            execution_mode,
            block_context,
        ),
    }
}

// Initial version of the function.
// Contains a bug in bytecode compression.
fn get_bootloader_memory_v1(
    txs: Vec<TransactionData>,
    predefined_refunds: Vec<u32>,
    predefined_compressed_bytecodes: Vec<Vec<CompressedBytecodeInfo>>,
    execution_mode: TxExecutionMode,
    block_context: BlockContextMode,
) -> Vec<(usize, U256)> {
    let inner_context = block_context.inner_block_context().context;

    let block_gas_price_per_pubdata = inner_context.block_gas_price_per_pubdata();

    let mut memory = bootloader_initial_memory(&block_context);

    let mut previous_compressed: usize = 0;
    let mut already_included_txs_size = 0;
    for (tx_index_in_block, tx) in txs.into_iter().enumerate() {
        let compressed_bytecodes = predefined_compressed_bytecodes[tx_index_in_block].clone();

        let mut total_compressed_len = 0;
        for i in compressed_bytecodes.iter() {
            total_compressed_len += i.encode_call().len()
        }

        let memory_for_current_tx = get_bootloader_memory_for_tx(
            tx.clone(),
            tx_index_in_block,
            execution_mode,
            already_included_txs_size,
            predefined_refunds[tx_index_in_block],
            block_gas_price_per_pubdata.as_u32(),
            previous_compressed,
            compressed_bytecodes,
        );

        previous_compressed += total_compressed_len;

        memory.extend(memory_for_current_tx);
        let encoded_struct = tx.into_tokens();
        let encoding_length = encoded_struct.len();
        already_included_txs_size += encoding_length;
    }
    memory
}

// Version with the bug fixed
fn get_bootloader_memory_v2(
    txs: Vec<TransactionData>,
    predefined_refunds: Vec<u32>,
    predefined_compressed_bytecodes: Vec<Vec<CompressedBytecodeInfo>>,
    execution_mode: TxExecutionMode,
    block_context: BlockContextMode,
) -> Vec<(usize, U256)> {
    let inner_context = block_context.inner_block_context().context;

    let block_gas_price_per_pubdata = inner_context.block_gas_price_per_pubdata();

    let mut memory = bootloader_initial_memory(&block_context);

    let mut previous_compressed: usize = 0;
    let mut already_included_txs_size = 0;
    for (tx_index_in_block, tx) in txs.into_iter().enumerate() {
        let compressed_bytecodes = predefined_compressed_bytecodes[tx_index_in_block].clone();

        let mut total_compressed_len_words = 0;
        for i in compressed_bytecodes.iter() {
            total_compressed_len_words += i.encode_call().len() / 32;
        }

        let memory_for_current_tx = get_bootloader_memory_for_tx(
            tx.clone(),
            tx_index_in_block,
            execution_mode,
            already_included_txs_size,
            predefined_refunds[tx_index_in_block],
            block_gas_price_per_pubdata.as_u32(),
            previous_compressed,
            compressed_bytecodes,
        );

        previous_compressed += total_compressed_len_words;

        memory.extend(memory_for_current_tx);
        let encoded_struct = tx.into_tokens();
        let encoding_length = encoded_struct.len();
        already_included_txs_size += encoding_length;
    }
    memory
}

pub fn push_transaction_to_bootloader_memory<S: Storage, H: HistoryMode>(
    vm: &mut VmInstance<S, H>,
    tx: &Transaction,
    execution_mode: TxExecutionMode,
    explicit_compressed_bytecodes: Option<Vec<CompressedBytecodeInfo>>,
) {
    let tx: TransactionData = tx.clone().into();
    let block_gas_per_pubdata_byte = vm.block_context.context.block_gas_price_per_pubdata();
    let overhead = tx.overhead_gas(block_gas_per_pubdata_byte.as_u32());
    push_raw_transaction_to_bootloader_memory(
        vm,
        tx,
        execution_mode,
        overhead,
        explicit_compressed_bytecodes,
    );
}

pub fn push_raw_transaction_to_bootloader_memory<S: Storage, H: HistoryMode>(
    vm: &mut VmInstance<S, H>,
    tx: TransactionData,
    execution_mode: TxExecutionMode,
    predefined_overhead: u32,
    explicit_compressed_bytecodes: Option<Vec<CompressedBytecodeInfo>>,
) {
    match vm.vm_subversion {
        MultiVMSubversion::V1 => push_raw_transaction_to_bootloader_memory_v1(
            vm,
            tx,
            execution_mode,
            predefined_overhead,
            explicit_compressed_bytecodes,
        ),
        MultiVMSubversion::V2 => push_raw_transaction_to_bootloader_memory_v2(
            vm,
            tx,
            execution_mode,
            predefined_overhead,
            explicit_compressed_bytecodes,
        ),
    }
}

/// Contains a bug in the bytecode compression.
fn push_raw_transaction_to_bootloader_memory_v1<S: Storage, H: HistoryMode>(
    vm: &mut VmInstance<S, H>,
    tx: TransactionData,
    execution_mode: TxExecutionMode,
    predefined_overhead: u32,
    explicit_compressed_bytecodes: Option<Vec<CompressedBytecodeInfo>>,
) {
    let tx_index_in_block = vm.bootloader_state.free_tx_index();
    let already_included_txs_size = vm.bootloader_state.free_tx_offset();

    let timestamp = Timestamp(vm.state.local_state.timestamp);
    let codes_for_decommiter = tx
        .factory_deps
        .iter()
        .map(|dep| bytecode_to_factory_dep(dep.clone()))
        .collect();

    let compressed_bytecodes = explicit_compressed_bytecodes.unwrap_or_else(|| {
        if tx.tx_type == L1_TX_TYPE {
            // L1 transactions do not need compression
            return vec![];
        }

        tx.factory_deps
            .iter()
            .filter_map(|bytecode| {
                if vm.is_bytecode_exists(&hash_bytecode(bytecode)) {
                    return None;
                }

                compress_bytecode(bytecode)
                    .ok()
                    .map(|compressed| CompressedBytecodeInfo {
                        original: bytecode.clone(),
                        compressed,
                    })
            })
            .collect()
    });
    let compressed_len = compressed_bytecodes
        .iter()
        .map(|bytecode| bytecode.compressed.len())
        .sum();

    vm.state
        .decommittment_processor
        .populate(codes_for_decommiter, timestamp);

    let block_gas_price_per_pubdata = vm.block_context.context.block_gas_price_per_pubdata();
    let trusted_ergs_limit = tx.trusted_gas_limit(block_gas_price_per_pubdata.as_u32());
    let encoded_tx = tx.into_tokens();
    let encoded_tx_size = encoded_tx.len();

    let previous_bytecodes = vm.bootloader_state.get_compressed_bytecodes();

    let bootloader_memory = get_bootloader_memory_for_encoded_tx(
        encoded_tx,
        tx_index_in_block,
        execution_mode,
        already_included_txs_size,
        0,
        predefined_overhead,
        trusted_ergs_limit,
        previous_bytecodes,
        compressed_bytecodes,
    );

    vm.state.memory.populate_page(
        BOOTLOADER_HEAP_PAGE as usize,
        bootloader_memory,
        Timestamp(vm.state.local_state.timestamp),
    );
    vm.bootloader_state.add_tx_data(encoded_tx_size);
    vm.bootloader_state.add_compressed_bytecode(compressed_len);
}

// Bytecode compression bug fixed
fn push_raw_transaction_to_bootloader_memory_v2<S: Storage, H: HistoryMode>(
    vm: &mut VmInstance<S, H>,
    tx: TransactionData,
    execution_mode: TxExecutionMode,
    predefined_overhead: u32,
    explicit_compressed_bytecodes: Option<Vec<CompressedBytecodeInfo>>,
) {
    let tx_index_in_block = vm.bootloader_state.free_tx_index();
    let already_included_txs_size = vm.bootloader_state.free_tx_offset();

    let timestamp = Timestamp(vm.state.local_state.timestamp);
    let codes_for_decommiter = tx
        .factory_deps
        .iter()
        .map(|dep| bytecode_to_factory_dep(dep.clone()))
        .collect();

    let compressed_bytecodes = explicit_compressed_bytecodes.unwrap_or_else(|| {
        if tx.tx_type == L1_TX_TYPE {
            // L1 transactions do not need compression
            return vec![];
        }

        tx.factory_deps
            .iter()
            .filter_map(|bytecode| {
                if vm.is_bytecode_exists(&hash_bytecode(bytecode)) {
                    return None;
                }

                compress_bytecode(bytecode)
                    .ok()
                    .map(|compressed| CompressedBytecodeInfo {
                        original: bytecode.clone(),
                        compressed,
                    })
            })
            .collect()
    });
    let compressed_bytecodes_encoding_len_words = compressed_bytecodes
        .iter()
        .map(|bytecode| {
            let encoding_length_bytes = bytecode.encode_call().len();
            assert!(
                encoding_length_bytes % 32 == 0,
                "ABI encoding of bytecode is not 32-byte aligned"
            );

            encoding_length_bytes / 32
        })
        .sum();

    vm.state
        .decommittment_processor
        .populate(codes_for_decommiter, timestamp);

    let block_gas_price_per_pubdata = vm.block_context.context.block_gas_price_per_pubdata();
    let trusted_ergs_limit = tx.trusted_gas_limit(block_gas_price_per_pubdata.as_u32());
    let encoded_tx = tx.into_tokens();
    let encoded_tx_size = encoded_tx.len();

    let previous_bytecodes = vm.bootloader_state.get_compressed_bytecodes();

    let bootloader_memory = get_bootloader_memory_for_encoded_tx(
        encoded_tx,
        tx_index_in_block,
        execution_mode,
        already_included_txs_size,
        0,
        predefined_overhead,
        trusted_ergs_limit,
        previous_bytecodes,
        compressed_bytecodes,
    );

    vm.state.memory.populate_page(
        BOOTLOADER_HEAP_PAGE as usize,
        bootloader_memory,
        Timestamp(vm.state.local_state.timestamp),
    );
    vm.bootloader_state.add_tx_data(encoded_tx_size);
    vm.bootloader_state
        .add_compressed_bytecode(compressed_bytecodes_encoding_len_words);
}

#[allow(clippy::too_many_arguments)]
fn get_bootloader_memory_for_tx(
    tx: TransactionData,
    tx_index_in_block: usize,
    execution_mode: TxExecutionMode,
    already_included_txs_size: usize,
    predefined_refund: u32,
    block_gas_per_pubdata: u32,
    previous_compressed_bytecode_size: usize,
    compressed_bytecodes: Vec<CompressedBytecodeInfo>,
) -> Vec<(usize, U256)> {
    let overhead_gas = tx.overhead_gas(block_gas_per_pubdata);
    let trusted_gas_limit = tx.trusted_gas_limit(block_gas_per_pubdata);
    get_bootloader_memory_for_encoded_tx(
        tx.into_tokens(),
        tx_index_in_block,
        execution_mode,
        already_included_txs_size,
        predefined_refund,
        overhead_gas,
        trusted_gas_limit,
        previous_compressed_bytecode_size,
        compressed_bytecodes,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn get_bootloader_memory_for_encoded_tx(
    encoded_tx: Vec<U256>,
    tx_index_in_block: usize,
    execution_mode: TxExecutionMode,
    already_included_txs_size: usize,
    predefined_refund: u32,
    predefined_overhead: u32,
    trusted_gas_limit: u32,
    previous_compressed_bytecode_size: usize,
    compressed_bytecodes: Vec<CompressedBytecodeInfo>,
) -> Vec<(usize, U256)> {
    let mut memory: Vec<(usize, U256)> = Vec::default();
    let bootloader_description_offset =
        BOOTLOADER_TX_DESCRIPTION_OFFSET + BOOTLOADER_TX_DESCRIPTION_SIZE * tx_index_in_block;

    let tx_description_offset = TX_DESCRIPTION_OFFSET + already_included_txs_size;

    // Marking that this transaction should be executed.
    memory.push((
        bootloader_description_offset,
        assemble_tx_meta(execution_mode, true),
    ));
    memory.push((
        bootloader_description_offset + 1,
        U256::from_big_endian(&(32 * tx_description_offset).to_be_bytes()),
    ));

    let refund_offset = OPERATOR_REFUNDS_OFFSET + tx_index_in_block;
    memory.push((refund_offset, predefined_refund.into()));

    let overhead_offset = TX_OVERHEAD_OFFSET + tx_index_in_block;
    memory.push((overhead_offset, predefined_overhead.into()));

    let trusted_gas_limit_offset = TX_TRUSTED_GAS_LIMIT_OFFSET + tx_index_in_block;
    memory.push((trusted_gas_limit_offset, trusted_gas_limit.into()));

    // Now we need to actually put the transaction description:
    let encoding_length = encoded_tx.len();
    memory.extend((tx_description_offset..tx_description_offset + encoding_length).zip(encoded_tx));

    // Note, +1 is moving for pointer
    let compressed_bytecodes_offset =
        COMPRESSED_BYTECODES_OFFSET + 1 + previous_compressed_bytecode_size;

    let memory_addition: Vec<_> = compressed_bytecodes
        .into_iter()
        .flat_map(|x| x.encode_call())
        .collect();

    let memory_addition = bytes_to_be_words(memory_addition);

    memory.extend(
        (compressed_bytecodes_offset..compressed_bytecodes_offset + memory_addition.len())
            .zip(memory_addition),
    );

    memory
}

fn get_default_local_state<S: Storage, H: HistoryMode>(
    tools: OracleTools<false, S, H>,
    block_properties: BlockProperties,
    gas_limit: u32,
) -> ZkSyncVmState<S, H> {
    let mut vm = VmState::empty_state(
        tools.storage,
        tools.memory,
        tools.event_sink,
        tools.precompiles_processor,
        tools.decommittment_processor,
        tools.witness_tracer,
        block_properties,
    );
    // Override ergs limit for the initial frame.
    vm.local_state.callstack.current.ergs_remaining = gas_limit;

    let initial_context = CallStackEntry {
        this_address: BOOTLOADER_ADDRESS,
        msg_sender: Address::zero(),
        code_address: BOOTLOADER_ADDRESS,
        base_memory_page: MemoryPage(BOOTLOADER_BASE_PAGE),
        code_page: MemoryPage(BOOTLOADER_CODE_PAGE),
        sp: 0,
        pc: 0,
        // Note, that since the results are written at the end of the memory
        // it is needed to have the entire heap available from the beginning
        heap_bound: MAX_MEMORY_BYTES as u32,
        aux_heap_bound: MAX_MEMORY_BYTES as u32,
        exception_handler_location: INITIAL_FRAME_FORMAL_EH_LOCATION,
        ergs_remaining: gas_limit,
        this_shard_id: 0,
        caller_shard_id: 0,
        code_shard_id: 0,
        is_static: false,
        is_local_frame: false,
        context_u128_value: 0,
    };

    // We consider the contract that is being run as a bootloader
    vm.push_bootloader_context(INITIAL_MONOTONIC_CYCLE_COUNTER - 1, initial_context);
    vm.local_state.timestamp = STARTING_TIMESTAMP;
    vm.local_state.memory_page_counter = STARTING_BASE_PAGE;
    vm.local_state.monotonic_cycle_counter = INITIAL_MONOTONIC_CYCLE_COUNTER;
    vm.local_state.current_ergs_per_pubdata_byte = 0;
    vm.local_state.registers[0] = formal_calldata_abi();

    // Deleting all the historical records brought by the initial
    // initialization of the VM to make them permanent.
    vm.decommittment_processor.delete_history();
    vm.event_sink.delete_history();
    vm.storage.delete_history();
    vm.memory.delete_history();
    vm.precompiles_processor.delete_history();

    vm
}

fn formal_calldata_abi() -> PrimitiveValue {
    let fat_pointer = FatPointer {
        offset: 0,
        memory_page: BOOTLOADER_CALLDATA_PAGE,
        start: 0,
        length: 0,
    };

    PrimitiveValue {
        value: fat_pointer.to_u256(),
        is_pointer: true,
    }
}

pub(crate) fn bytecode_to_factory_dep(bytecode: Vec<u8>) -> (U256, Vec<U256>) {
    let bytecode_hash = hash_bytecode(&bytecode);
    let bytecode_hash = U256::from_big_endian(bytecode_hash.as_bytes());

    let bytecode_words = bytes_to_be_words(bytecode);

    (bytecode_hash, bytecode_words)
}

/// Forms a word that contains meta information for the transaction execution.
///
/// # Current layout
///
/// - 0 byte (MSB): server-side tx execution mode
///     In the server, we may want to execute different parts of the transaction in the different context
///     For example, when checking validity, we don't want to actually execute transaction and have side effects.
///
///     Possible values:
///     - 0x00: validate & execute (normal mode)
///     - 0x01: validate but DO NOT execute
///     - 0x02: execute but DO NOT validate
///
/// - 31 byte (LSB): whether to execute transaction or not (at all).
fn assemble_tx_meta(execution_mode: TxExecutionMode, execute_tx: bool) -> U256 {
    let mut output = [0u8; 32];

    // Set 0 byte (execution mode)
    output[0] = match execution_mode {
        TxExecutionMode::VerifyExecute => 0x00,
        TxExecutionMode::EstimateFee { .. } => 0x00,
        TxExecutionMode::EthCall { .. } => 0x02,
    };

    // Set 31 byte (marker for tx execution)
    output[31] = u8::from(execute_tx);

    U256::from_big_endian(&output)
}
