use std::{cell::RefCell, collections::HashMap, rc::Rc};

use era_vm::{
    state::{L2ToL1Log, VMState},
    store::{StorageError, StorageKey as EraStorageKey},
    vm::ExecutionOutput,
    EraVM, Execution,
};
use zksync_state::{ReadStorage, StoragePtr};
use zksync_types::{
    l1::is_l1_tx_type,
    utils::key_for_eth_balance,
    writes::{
        compression::compress_with_best_strategy, BYTES_PER_DERIVED_KEY,
        BYTES_PER_ENUMERATION_INDEX,
    },
    AccountTreeId, StorageKey, Transaction, BOOTLOADER_ADDRESS, H160, KNOWN_CODES_STORAGE_ADDRESS,
    L2_BASE_TOKEN_ADDRESS, U256,
};
use zksync_utils::{
    bytecode::{hash_bytecode, CompressedBytecodeInfo},
    h256_to_u256, u256_to_h256,
};

use super::{
    bootloader_state::{utils::apply_l2_block, BootloaderState},
    event::merge_events,
    hook::Hook,
    initial_bootloader_memory::bootloader_initial_memory,
    snapshot::VmSnapshot,
};
use crate::{
    era_vm::{bytecode::compress_bytecodes, transaction_data::TransactionData},
    interface::{
        Halt, TxRevertReason, VmFactory, VmInterface, VmInterfaceHistoryEnabled, VmRevertReason,
    },
    vm_latest::{
        constants::{
            get_vm_hook_position, get_vm_hook_start_position_latest, VM_HOOK_PARAMS_COUNT,
        },
        BootloaderMemory, CurrentExecutionState, ExecutionResult, L1BatchEnv, L2BlockEnv,
        SystemEnv, VmExecutionLogs, VmExecutionMode, VmExecutionResultAndLogs,
    },
};

pub struct Vm<S: ReadStorage> {
    pub(crate) inner: EraVM,
    suspended_at: u16,
    gas_for_account_validation: u32,
    last_tx_result: Option<ExecutionResult>,

    bootloader_state: BootloaderState,
    pub(crate) storage: StoragePtr<S>,

    // TODO: Maybe not necessary, check
    program_cache: Rc<RefCell<HashMap<U256, Vec<U256>>>>,

    // these two are only needed for tests so far
    pub(crate) batch_env: L1BatchEnv,
    pub(crate) system_env: SystemEnv,

    snapshots: Vec<VmSnapshot>, // TODO: Implement snapshots logic
}

/// Encapsulates creating VM instance based on the provided environment.
impl<S: ReadStorage + 'static> VmFactory<S> for Vm<S> {
    /// Creates a new VM instance.
    fn new(batch_env: L1BatchEnv, system_env: SystemEnv, storage: StoragePtr<S>) -> Self {
        let bootloader_code = system_env
            .base_system_smart_contracts
            .bootloader
            .code
            .clone();
        let vm_hook_position =
            get_vm_hook_position(crate::vm_latest::MultiVMSubversion::IncreasedBootloaderMemory)
                * 32;
        let vm_execution = Execution::new(
            bootloader_code.to_owned(),
            Vec::new(),
            BOOTLOADER_ADDRESS,
            H160::zero(),
            0_u128,
            system_env
                .base_system_smart_contracts
                .default_aa
                .hash
                .to_fixed_bytes(),
            system_env
                .base_system_smart_contracts
                .default_aa //TODO: Add real evm interpreter
                .hash
                .to_fixed_bytes(),
            vm_hook_position,
            true,
            10_000_000, //TODO: Maybe we want to pass in a parameter
        );
        let pre_contract_storage = Rc::new(RefCell::new(HashMap::new()));
        pre_contract_storage.borrow_mut().insert(
            h256_to_u256(system_env.base_system_smart_contracts.default_aa.hash),
            system_env
                .base_system_smart_contracts
                .default_aa
                .code
                .clone(),
        );
        let world_storage = World::new(storage.clone(), pre_contract_storage.clone());
        let mut vm = EraVM::new(vm_execution, Rc::new(RefCell::new(world_storage)));
        let bootloader_memory = bootloader_initial_memory(&batch_env);

        // The bootloader shouldn't pay for growing memory and it writes results
        // to the end of its heap, so it makes sense to preallocate it in its entirety.
        const BOOTLOADER_MAX_MEMORY_SIZE: u32 = 59000000;
        vm.execution
            .heaps
            .get_mut(era_vm::execution::FIRST_HEAP)
            .unwrap()
            .expand_memory(BOOTLOADER_MAX_MEMORY_SIZE);
        vm.execution
            .heaps
            .get_mut(era_vm::execution::FIRST_HEAP + 1)
            .unwrap()
            .expand_memory(BOOTLOADER_MAX_MEMORY_SIZE);

        let mut mv = Self {
            inner: vm,
            suspended_at: 0,
            gas_for_account_validation: system_env.default_validation_computational_gas_limit,
            last_tx_result: None,
            bootloader_state: BootloaderState::new(
                system_env.execution_mode.clone(),
                bootloader_initial_memory(&batch_env),
                batch_env.first_l2_block,
            ),
            program_cache: pre_contract_storage,
            storage,
            batch_env,
            system_env,
            snapshots: Vec::new(),
        };

        mv.write_to_bootloader_heap(bootloader_memory);
        mv
    }
}

impl<S: ReadStorage + 'static> Vm<S> {
    pub fn run(&mut self, execution_mode: VmExecutionMode) -> ExecutionResult {
        loop {
            let (result, blob_tracer) = self.inner.run_program_with_custom_bytecode();
            let result = match result {
                ExecutionOutput::Ok(output) => return ExecutionResult::Success { output },
                ExecutionOutput::Revert(output) => match TxRevertReason::parse_error(&output) {
                    TxRevertReason::TxReverted(output) => {
                        return ExecutionResult::Revert { output }
                    }
                    TxRevertReason::Halt(reason) => return ExecutionResult::Halt { reason },
                },
                ExecutionOutput::Panic => {
                    return ExecutionResult::Halt {
                        reason: if self.inner.execution.gas_left().unwrap() == 0 {
                            Halt::BootloaderOutOfGas
                        } else {
                            Halt::VMPanic
                        },
                    }
                }
                ExecutionOutput::SuspendedOnHook {
                    hook,
                    pc_to_resume_from,
                } => {
                    self.suspended_at = pc_to_resume_from;
                    hook
                }
            };

            match Hook::from_u32(result) {
                Hook::FinalBatchInfo => {
                    // println!("FINAL BATCH INFO");
                    // set fictive l2 block
                    let txs_index = self.bootloader_state.free_tx_index();
                    let l2_block = self.bootloader_state.insert_fictive_l2_block();
                    let mut memory = vec![];
                    apply_l2_block(&mut memory, l2_block, txs_index);
                    self.write_to_bootloader_heap(memory);
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
                Hook::PostResult => {
                    // println!("POST RESULT");
                    let result = self.get_hook_params()[0];
                    // println!("RESULT: {:?}", result);

                    // TODO get latest return data
                    let return_data = vec![];

                    self.last_tx_result = Some(if result.is_zero() {
                        // println!("Reverted");
                        ExecutionResult::Revert {
                            output: VmRevertReason::from(return_data.as_slice()),
                        }
                    } else {
                        ExecutionResult::Success {
                            output: return_data,
                        }
                    });
                }
                Hook::NotifyAboutRefund => {
                    // println!("NOTIFY ABOUT REFUND");
                }
                Hook::AskOperatorForRefund => {
                    // println!("ASK OPERATOR FOR REFUND");
                }
                Hook::DebugLog => {
                    let heap = self
                        .inner
                        .execution
                        .heaps
                        .get(self.inner.execution.current_context().unwrap().heap_id)
                        .unwrap();
                    let vm_hook_params: Vec<_> = self
                        .get_vm_hook_params(heap)
                        .into_iter()
                        .map(u256_to_h256)
                        .collect();
                    let msg = vm_hook_params[0].as_bytes().to_vec();
                    let data = vm_hook_params[1].as_bytes().to_vec();

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

                    //println!("BOOTLOADER: {} {}", msg, data_str)
                }
                Hook::TxHasEnded => {
                    // println!("TX HAS ENDED");
                    if let (VmExecutionMode::OneTx, Some(result)) =
                        (execution_mode, self.last_tx_result.take())
                    {
                        return result;
                    }
                }
            }
            self.inner.execution.current_frame_mut().unwrap().pc = self.suspended_at as u64;
        }
    }

    fn get_vm_hook_params(&self, heap: &era_vm::execution::Heap) -> Vec<U256> {
        (get_vm_hook_start_position_latest()..get_vm_hook_start_position_latest() + 2)
            .map(|word| {
                let res = heap.read((word * 32) as u32);
                // println!("WORD: {:?} RES: {:?}", word, res);
                res
            })
            .collect()
    }

    pub(crate) fn insert_bytecodes<'a>(&mut self, bytecodes: impl IntoIterator<Item = &'a [u8]>) {
        for code in bytecodes {
            let mut program_code = vec![];
            for raw_opcode_slice in code.chunks(32) {
                let mut raw_opcode_bytes: [u8; 32] = [0; 32];
                raw_opcode_bytes.copy_from_slice(&raw_opcode_slice[..32]);
                let raw_opcode_u256 = U256::from_big_endian(&raw_opcode_bytes);
                program_code.push(raw_opcode_u256);
            }
            // println!("HASH: {:?}", U256::from_big_endian(hash_bytecode(code).as_bytes()));
            // println!("PROGRAM CODE: {:?}", program_code);
            self.program_cache.borrow_mut().insert(
                U256::from_big_endian(hash_bytecode(code).as_bytes()),
                program_code,
            );
        }
    }

    fn get_hook_params(&self) -> [U256; 3] {
        let vm_hooks_param_start = get_vm_hook_start_position_latest();
        (vm_hooks_param_start..vm_hooks_param_start + VM_HOOK_PARAMS_COUNT)
            .map(|word| {
                let res = self.read_heap_word(word as usize);
                // println!("WORD: {:?} RES: {:?}", word, res);
                res
            })
            .collect::<Vec<_>>()
            .try_into()
            .unwrap()
        // [U256::from_dec_str("29723826").unwrap() ,U256::zero() ,U256::from_dec_str("3400").unwrap()]
    }

    /// Typically used to read the bootloader heap. We know that we're in the bootloader
    /// when a hook occurs, as they are only enabled when preprocessing bootloader code.
    fn read_heap_word(&self, word: usize) -> U256 {
        let heap = self
            .inner
            .execution
            .heaps
            .get(self.inner.execution.current_context().unwrap().heap_id)
            .unwrap();
        heap.read((word * 32) as u32)
    }

    #[cfg(test)]
    /// Returns the current state of the VM in a format that can be compared for equality.
    pub(crate) fn dump_state(&self) -> Execution {
        self.inner.execution.clone()
    }

    fn write_to_bootloader_heap(&mut self, memory: impl IntoIterator<Item = (usize, U256)>) {
        assert!(self.inner.execution.running_contexts.len() == 1); // No on-going far calls
        if let Some(heap) = &mut self
            .inner
            .execution
            .heaps
            .get_mut(self.inner.execution.current_context().unwrap().heap_id)
        {
            for (slot, value) in memory {
                let end = (slot + 1) * 32;
                heap.expand_memory(end as u32);
                heap.store((slot * 32) as u32, value);
            }
        }
    }
}

impl<S: ReadStorage + 'static> VmInterface for Vm<S> {
    type TracerDispatcher = ();

    fn push_transaction(&mut self, tx: Transaction) {
        let tx: TransactionData = tx.into();
        let overhead = tx.overhead_gas();

        self.insert_bytecodes(tx.factory_deps.iter().map(|dep| &dep[..]));

        let compressed_bytecodes = if is_l1_tx_type(tx.tx_type) {
            // L1 transactions do not need compression
            vec![]
        } else {
            compress_bytecodes(&tx.factory_deps, |hash| {
                self.inner
                    .state
                    .storage_changes()
                    .get(&EraStorageKey::new(
                        KNOWN_CODES_STORAGE_ADDRESS,
                        h256_to_u256(hash),
                    ))
                    .map(|x| !x.is_zero())
                    .unwrap_or_else(|| self.storage.is_bytecode_known(&hash))
            })
        };

        let trusted_ergs_limit = tx.trusted_ergs_limit();

        let memory = self.bootloader_state.push_tx(
            tx,
            overhead,
            0, //TODO: Is this correct?
            compressed_bytecodes,
            trusted_ergs_limit,
            self.system_env.chain_id,
        );

        self.write_to_bootloader_heap(memory);
    }

    fn inspect(
        &mut self,
        _tracer: Self::TracerDispatcher,
        execution_mode: VmExecutionMode,
    ) -> VmExecutionResultAndLogs {
        let mut enable_refund_tracer = false;
        if let VmExecutionMode::OneTx = execution_mode {
            // Move the pointer to the next transaction
            self.bootloader_state.move_tx_to_execute_pointer();
            enable_refund_tracer = true;
        }

        let result = self.run(execution_mode);

        VmExecutionResultAndLogs {
            result,
            logs: VmExecutionLogs {
                storage_logs: Default::default(),
                events: merge_events(self.inner.state.events(), self.batch_env.number),
                user_l2_to_l1_logs: Default::default(),
                system_l2_to_l1_logs: Default::default(),
                total_log_queries_count: 0, // This field is unused
            },
            statistics: Default::default(),
            refunds: Default::default(),
        }
    }

    fn get_bootloader_memory(&self) -> BootloaderMemory {
        self.bootloader_state.bootloader_memory()
    }

    fn get_last_tx_compressed_bytecodes(&self) -> Vec<CompressedBytecodeInfo> {
        self.bootloader_state.get_last_tx_compressed_bytecodes()
    }

    fn start_new_l2_block(&mut self, l2_block_env: L2BlockEnv) {
        self.bootloader_state.start_new_l2_block(l2_block_env)
    }

    fn get_current_execution_state(&self) -> CurrentExecutionState {
        todo!()
    }

    fn inspect_transaction_with_bytecode_compression(
        &mut self,
        tracer: Self::TracerDispatcher,
        tx: zksync_types::Transaction,
        with_compression: bool,
    ) -> (
        Result<(), crate::interface::BytecodeCompressionError>,
        VmExecutionResultAndLogs,
    ) {
        todo!()
    }

    fn record_vm_memory_metrics(&self) -> crate::vm_1_4_1::VmMemoryMetrics {
        todo!()
    }

    fn gas_remaining(&self) -> u32 {
        self.inner.execution.current_frame().unwrap().gas_left.0
    }
}

impl<S: ReadStorage + 'static> VmInterfaceHistoryEnabled for Vm<S> {
    fn make_snapshot(&mut self) {
        todo!()
    }

    fn rollback_to_the_latest_snapshot(&mut self) {
        todo!()
    }

    fn pop_snapshot_no_rollback(&mut self) {
        todo!()
    }
}

#[derive(Debug, Clone)]
pub struct World<S: ReadStorage> {
    pub storage: StoragePtr<S>,
    pub contract_storage: Rc<RefCell<HashMap<U256, Vec<U256>>>>,
}

impl<S: ReadStorage> World<S> {
    pub fn new_empty(storage: StoragePtr<S>) -> Self {
        let contract_storage = Rc::new(RefCell::new(HashMap::new()));
        Self {
            contract_storage,
            storage,
        }
    }

    pub fn new(
        storage: StoragePtr<S>,
        contract_storage: Rc<RefCell<HashMap<U256, Vec<U256>>>>,
    ) -> Self {
        Self {
            storage,
            contract_storage,
        }
    }
}

impl<S: ReadStorage> era_vm::store::Storage for World<S> {
    fn decommit(&mut self, hash: U256) -> Option<Vec<U256>> {
        Some(
            self.contract_storage
                .borrow_mut()
                .entry(hash)
                .or_insert_with(|| {
                    let contract = self
                        .storage
                        .borrow_mut()
                        .load_factory_dep(u256_to_h256(hash))
                        .expect("Bytecode not found");
                    let mut program_code = vec![];
                    for raw_opcode_slice in contract.chunks(32) {
                        let mut raw_opcode_bytes: [u8; 32] = [0; 32];
                        raw_opcode_bytes.copy_from_slice(&raw_opcode_slice[..32]);

                        let raw_opcode_u256 = U256::from_big_endian(&raw_opcode_bytes);
                        program_code.push(raw_opcode_u256);
                    }
                    program_code
                })
                .clone(),
        )
    }

    fn storage_read(
        &mut self,
        storage_key: &era_vm::store::StorageKey,
    ) -> std::option::Option<U256> {
        let key = &StorageKey::new(
            AccountTreeId::new(storage_key.address),
            u256_to_h256(storage_key.key),
        );

        if self.storage.is_write_initial(&key) {
            None
        } else {
            Some(self.storage.borrow_mut().read_value(key).0.into())
        }
    }

    fn cost_of_writing_storage(
        &mut self,
        storage_key: &era_vm::store::StorageKey,
        value: U256,
    ) -> u32 {
        let initial_value = self.storage_read(storage_key);
        let is_initial = initial_value.is_none();
        let initial_value = initial_value.unwrap_or_default();

        if initial_value == value {
            return 0;
        }

        // Since we need to publish the state diffs onchain, for each of the updated storage slot
        // we basically need to publish the following pair: `(<storage_key, compressed_new_value>)`.
        // For key we use the following optimization:
        //   - The first time we publish it, we use 32 bytes.
        //         Then, we remember a 8-byte id for this slot and assign it to it. We call this initial write.
        //   - The second time we publish it, we will use the 4/5 byte representation of this 8-byte instead of the 32
        //     bytes of the entire key.
        // For value compression, we use a metadata byte which holds the length of the value and the operation from the
        // previous state to the new state, and the compressed value. The maximum for this is 33 bytes.
        // Total bytes for initial writes then becomes 65 bytes and repeated writes becomes 38 bytes.
        let compressed_value_size = compress_with_best_strategy(initial_value, value).len() as u32;

        if is_initial {
            (BYTES_PER_DERIVED_KEY as u32) + compressed_value_size
        } else {
            (BYTES_PER_ENUMERATION_INDEX as u32) + compressed_value_size
        }
    }

    fn is_free_storage_slot(&self, storage_key: &era_vm::store::StorageKey) -> bool {
        storage_key.address == zksync_system_constants::SYSTEM_CONTEXT_ADDRESS
            || storage_key.address == L2_BASE_TOKEN_ADDRESS
                && u256_to_h256(storage_key.key) == key_for_eth_balance(&BOOTLOADER_ADDRESS)
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, path::PathBuf, rc::Rc};

    use once_cell::sync::Lazy;
    use zksync_contracts::{deployer_contract, BaseSystemContracts};
    use zksync_state::{InMemoryStorage, StorageView};
    use zksync_types::{
        block::L2BlockHasher,
        ethabi::{encode, Token},
        fee::Fee,
        fee_model::BatchFeeInput,
        helpers::unix_timestamp_ms,
        l2::L2Tx,
        utils::storage_key_for_eth_balance,
        Address, K256PrivateKey, L1BatchNumber, L2BlockNumber, L2ChainId, Nonce, ProtocolVersionId,
        Transaction, CONTRACT_DEPLOYER_ADDRESS, H256, U256,
    };
    use zksync_utils::bytecode::hash_bytecode;

    use super::*;
    use crate::{
        era_vm::vm::Vm,
        interface::{L2BlockEnv, TxExecutionMode, VmExecutionMode, VmInterface},
        utils::get_max_gas_per_pubdata_byte,
        vm_latest::constants::BATCH_COMPUTATIONAL_GAS_LIMIT,
    };
    /// Bytecodes have consist of an odd number of 32 byte words
    /// This function "fixes" bytecodes of wrong length by cutting off their end.
    pub fn cut_to_allowed_bytecode_size(bytes: &[u8]) -> Option<&[u8]> {
        let mut words = bytes.len() / 32;
        if words == 0 {
            return None;
        }
        if words & 1 == 0 {
            words -= 1;
        }
        Some(&bytes[..32 * words])
    }

    static PRIVATE_KEY: Lazy<K256PrivateKey> =
        Lazy::new(|| K256PrivateKey::from_bytes(H256([42; 32])).expect("invalid key bytes"));
    static SYSTEM_CONTRACTS: Lazy<BaseSystemContracts> =
        Lazy::new(BaseSystemContracts::load_from_disk);
    static STORAGE: Lazy<InMemoryStorage> = Lazy::new(|| {
        let mut storage = InMemoryStorage::with_system_contracts(hash_bytecode);

        // Give `PRIVATE_KEY` some money
        let key = storage_key_for_eth_balance(&PRIVATE_KEY.address());
        storage.set_value(key, zksync_utils::u256_to_h256(U256([0, 0, 1, 0])));

        storage
    });
    static CREATE_FUNCTION_SIGNATURE: Lazy<[u8; 4]> = Lazy::new(|| {
        deployer_contract()
            .function("create")
            .unwrap()
            .short_signature()
    });

    pub fn get_deploy_tx(code: &[u8]) -> Transaction {
        let params = [
            Token::FixedBytes(vec![0u8; 32]),
            Token::FixedBytes(hash_bytecode(code).0.to_vec()),
            Token::Bytes([].to_vec()),
        ];
        let calldata = CREATE_FUNCTION_SIGNATURE
            .iter()
            .cloned()
            .chain(encode(&params))
            .collect();

        let mut signed = L2Tx::new_signed(
            CONTRACT_DEPLOYER_ADDRESS,
            calldata,
            Nonce(0),
            Fee {
                gas_limit: U256::from(30000000u32),
                max_fee_per_gas: U256::from(250_000_000),
                max_priority_fee_per_gas: U256::from(0),
                gas_per_pubdata_limit: U256::from(get_max_gas_per_pubdata_byte(
                    ProtocolVersionId::latest().into(),
                )),
            },
            U256::zero(),
            L2ChainId::from(270),
            &PRIVATE_KEY,
            vec![code.to_vec()], // maybe not needed?
            Default::default(),
        )
        .expect("should create a signed execute transaction");

        signed.set_input(H256::random().as_bytes().to_vec(), H256::random());

        signed.into()
    }

    #[test]
    fn test_vm() {
        let path = PathBuf::from("./src/versions/era_vm/test_contract/storage");
        let test_contract = std::fs::read(&path).expect("failed to read file");
        let code = cut_to_allowed_bytecode_size(&test_contract).unwrap();
        let tx = get_deploy_tx(code);
        let timestamp = unix_timestamp_ms();
        let mut vm = Vm::new(
            crate::interface::L1BatchEnv {
                previous_batch_hash: None,
                number: L1BatchNumber(1),
                timestamp,
                fee_input: BatchFeeInput::l1_pegged(
                    50_000_000_000, // 50 gwei
                    250_000_000,    // 0.25 gwei
                ),
                fee_account: Address::random(),
                enforced_base_fee: None,
                first_l2_block: L2BlockEnv {
                    number: 1,
                    timestamp,
                    prev_block_hash: L2BlockHasher::legacy_hash(L2BlockNumber(0)),
                    max_virtual_blocks_to_create: 100,
                },
            },
            crate::interface::SystemEnv {
                zk_porter_available: false,
                version: ProtocolVersionId::latest(),
                base_system_smart_contracts: SYSTEM_CONTRACTS.clone(),
                bootloader_gas_limit: BATCH_COMPUTATIONAL_GAS_LIMIT,
                execution_mode: TxExecutionMode::VerifyExecute,
                default_validation_computational_gas_limit: BATCH_COMPUTATIONAL_GAS_LIMIT,
                chain_id: L2ChainId::from(270),
            },
            Rc::new(RefCell::new(StorageView::new(&*STORAGE))),
        );
        vm.push_transaction(tx);
        let a = vm.execute(VmExecutionMode::OneTx);
        println!("{:?}", a.result);
    }
}
