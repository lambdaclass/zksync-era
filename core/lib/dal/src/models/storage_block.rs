use std::{convert::TryInto, str::FromStr};

use bigdecimal::{BigDecimal, ToPrimitive};
use sqlx::types::chrono::{DateTime, NaiveDateTime, Utc};
use thiserror::Error;
use zksync_contracts::BaseSystemContractsHashes;
use zksync_types::{
    api,
    block::{L1BatchHeader, L2BlockHeader, UnsealedL1BatchHeader},
    commitment::{L1BatchCommitmentMode, L1BatchMetaParameters, L1BatchMetadata, PubdataParams},
    fee_model::{BatchFeeInput, L1PeggedBatchFeeModelInput, PubdataIndependentBatchFeeModelInput},
    l2_to_l1_log::{L2ToL1Log, SystemL2ToL1Log, UserL2ToL1Log},
    Address, Bloom, L1BatchNumber, L2BlockNumber, ProtocolVersionId, H256,
};

/// This is the gas limit that was used inside blocks before we started saving block gas limit into the database.
pub(crate) const LEGACY_BLOCK_GAS_LIMIT: u32 = u32::MAX;

/// App-level error fetching L1 batch metadata. For now, there's only one kind of such errors:
/// incomplete metadata.
#[derive(Debug, Error)]
pub enum L1BatchMetadataError {
    #[error("incomplete L1 batch metadata: missing `{}` field", _0)]
    Incomplete(&'static str),
}

/// L1 batch header with optional metadata.
#[derive(Debug)]
pub struct L1BatchWithOptionalMetadata {
    pub header: L1BatchHeader,
    pub metadata: Result<L1BatchMetadata, L1BatchMetadataError>,
}

/// Projection of the `l1_batches` table corresponding to [`L1BatchHeader`].
#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct StorageL1BatchHeader {
    pub number: i64,
    pub timestamp: i64,
    pub l1_tx_count: i32,
    pub l2_tx_count: i32,
    pub l2_to_l1_messages: Vec<Vec<u8>>,
    pub bloom: Vec<u8>,
    pub priority_ops_onchain_data: Vec<Vec<u8>>,
    pub used_contract_hashes: serde_json::Value,
    pub bootloader_code_hash: Option<Vec<u8>>,
    pub default_aa_code_hash: Option<Vec<u8>>,
    pub evm_emulator_code_hash: Option<Vec<u8>>,
    pub protocol_version: Option<i32>,

    // `system_logs` are introduced as part of boojum and will be absent in all batches generated prior to boojum.
    // System logs are logs generated by the VM execution, rather than directly from user transactions,
    // that facilitate sending information required for committing a batch to l1. In a given batch there
    // will be exactly 7 (or 8 in the event of a protocol upgrade) system logs.
    pub system_logs: Vec<Vec<u8>>,
    pub pubdata_input: Option<Vec<u8>>,
    pub fee_address: Vec<u8>,
}

impl StorageL1BatchHeader {
    pub fn into_l1_batch_header_with_logs(
        self,
        l2_to_l1_logs: Vec<UserL2ToL1Log>,
    ) -> L1BatchHeader {
        let priority_ops_onchain_data: Vec<_> = self
            .priority_ops_onchain_data
            .into_iter()
            .map(|raw_data| raw_data.into())
            .collect();

        let system_logs = convert_l2_to_l1_logs(self.system_logs);

        L1BatchHeader {
            number: L1BatchNumber(self.number as u32),
            timestamp: self.timestamp as u64,
            priority_ops_onchain_data,
            l1_tx_count: self.l1_tx_count as u16,
            l2_tx_count: self.l2_tx_count as u16,
            l2_to_l1_logs,
            l2_to_l1_messages: self.l2_to_l1_messages,

            bloom: Bloom::from_slice(&self.bloom),
            used_contract_hashes: serde_json::from_value(self.used_contract_hashes)
                .expect("invalid value for used_contract_hashes in the DB"),
            base_system_contracts_hashes: convert_base_system_contracts_hashes(
                self.bootloader_code_hash,
                self.default_aa_code_hash,
                self.evm_emulator_code_hash,
            ),
            system_logs: system_logs.into_iter().map(SystemL2ToL1Log).collect(),
            protocol_version: self
                .protocol_version
                .map(|v| (v as u16).try_into().unwrap()),
            pubdata_input: self.pubdata_input,
            fee_address: Address::from_slice(&self.fee_address),
        }
    }
}

fn convert_l2_to_l1_logs(raw_logs: Vec<Vec<u8>>) -> Vec<L2ToL1Log> {
    raw_logs
        .into_iter()
        .map(|raw_log| L2ToL1Log::from_slice(&raw_log))
        .collect()
}

// TODO (SMA-1635): Make these fields non optional in database
fn convert_base_system_contracts_hashes(
    bootloader_code_hash: Option<Vec<u8>>,
    default_aa_code_hash: Option<Vec<u8>>,
    evm_emulator_code_hash: Option<Vec<u8>>,
) -> BaseSystemContractsHashes {
    BaseSystemContractsHashes {
        bootloader: bootloader_code_hash
            .map(|hash| H256::from_slice(&hash))
            .expect("should not be none"),
        default_aa: default_aa_code_hash
            .map(|hash| H256::from_slice(&hash))
            .expect("should not be none"),
        evm_emulator: evm_emulator_code_hash.map(|hash| H256::from_slice(&hash)),
    }
}

/// Projection of the columns corresponding to [`L1BatchHeader`] + [`L1BatchMetadata`].
#[derive(Debug, Clone)]
pub(crate) struct StorageL1Batch {
    pub number: i64,
    pub timestamp: i64,
    pub l1_tx_count: i32,
    pub l2_tx_count: i32,
    pub bloom: Vec<u8>,
    pub priority_ops_onchain_data: Vec<Vec<u8>>,

    pub hash: Option<Vec<u8>>,
    pub commitment: Option<Vec<u8>>,
    pub meta_parameters_hash: Option<Vec<u8>>,
    pub pass_through_data_hash: Option<Vec<u8>>,
    pub aux_data_hash: Option<Vec<u8>>,

    pub rollup_last_leaf_index: Option<i64>,
    pub zkporter_is_available: Option<bool>,
    pub bootloader_code_hash: Option<Vec<u8>>,
    pub default_aa_code_hash: Option<Vec<u8>>,
    pub evm_emulator_code_hash: Option<Vec<u8>>,

    pub l2_to_l1_messages: Vec<Vec<u8>>,
    pub l2_l1_merkle_root: Option<Vec<u8>>,
    pub compressed_initial_writes: Option<Vec<u8>>,
    pub compressed_repeated_writes: Option<Vec<u8>>,

    pub used_contract_hashes: serde_json::Value,
    pub system_logs: Vec<Vec<u8>>,
    pub compressed_state_diffs: Option<Vec<u8>>,
    pub protocol_version: Option<i32>,
    pub events_queue_commitment: Option<Vec<u8>>,
    pub bootloader_initial_content_commitment: Option<Vec<u8>>,
    pub pubdata_input: Option<Vec<u8>>,
    pub blob_id: Option<String>,
    pub fee_address: Vec<u8>,
    pub aggregation_root: Option<Vec<u8>>,
    pub local_root: Option<Vec<u8>>,
    pub state_diff_hash: Option<Vec<u8>>,
    pub inclusion_data: Option<Vec<u8>>,
}

impl StorageL1Batch {
    pub fn into_l1_batch_header_with_logs(
        self,
        l2_to_l1_logs: Vec<UserL2ToL1Log>,
    ) -> L1BatchHeader {
        let priority_ops_onchain_data: Vec<_> = self
            .priority_ops_onchain_data
            .into_iter()
            .map(Vec::into)
            .collect();

        let system_logs = convert_l2_to_l1_logs(self.system_logs);

        L1BatchHeader {
            number: L1BatchNumber(self.number as u32),
            timestamp: self.timestamp as u64,
            priority_ops_onchain_data,
            l1_tx_count: self.l1_tx_count as u16,
            l2_tx_count: self.l2_tx_count as u16,
            l2_to_l1_logs,
            l2_to_l1_messages: self.l2_to_l1_messages,

            bloom: Bloom::from_slice(&self.bloom),
            used_contract_hashes: serde_json::from_value(self.used_contract_hashes)
                .expect("invalid value for used_contract_hashes in the DB"),
            base_system_contracts_hashes: convert_base_system_contracts_hashes(
                self.bootloader_code_hash,
                self.default_aa_code_hash,
                self.evm_emulator_code_hash,
            ),
            system_logs: system_logs.into_iter().map(SystemL2ToL1Log).collect(),
            protocol_version: self
                .protocol_version
                .map(|v| (v as u16).try_into().unwrap()),
            pubdata_input: self.pubdata_input,
            fee_address: Address::from_slice(&self.fee_address),
        }
    }
}

impl TryFrom<StorageL1Batch> for L1BatchMetadata {
    type Error = L1BatchMetadataError;

    fn try_from(batch: StorageL1Batch) -> Result<Self, Self::Error> {
        Ok(Self {
            root_hash: H256::from_slice(
                &batch.hash.ok_or(L1BatchMetadataError::Incomplete("hash"))?,
            ),
            rollup_last_leaf_index: batch
                .rollup_last_leaf_index
                .ok_or(L1BatchMetadataError::Incomplete("rollup_last_leaf_index"))?
                as u64,
            initial_writes_compressed: batch.compressed_initial_writes,
            repeated_writes_compressed: batch.compressed_repeated_writes,
            l2_l1_merkle_root: H256::from_slice(
                &batch
                    .l2_l1_merkle_root
                    .ok_or(L1BatchMetadataError::Incomplete("l2_l1_merkle_root"))?,
            ),
            aux_data_hash: H256::from_slice(
                &batch
                    .aux_data_hash
                    .ok_or(L1BatchMetadataError::Incomplete("aux_data_hash"))?,
            ),
            meta_parameters_hash: H256::from_slice(
                &batch
                    .meta_parameters_hash
                    .ok_or(L1BatchMetadataError::Incomplete("meta_parameters_hash"))?,
            ),
            pass_through_data_hash: H256::from_slice(
                &batch
                    .pass_through_data_hash
                    .ok_or(L1BatchMetadataError::Incomplete("pass_through_data_hash"))?,
            ),
            commitment: H256::from_slice(
                &batch
                    .commitment
                    .ok_or(L1BatchMetadataError::Incomplete("commitment"))?,
            ),
            block_meta_params: L1BatchMetaParameters {
                zkporter_is_available: batch
                    .zkporter_is_available
                    .ok_or(L1BatchMetadataError::Incomplete("zkporter_is_available"))?,
                bootloader_code_hash: H256::from_slice(
                    &batch
                        .bootloader_code_hash
                        .ok_or(L1BatchMetadataError::Incomplete("bootloader_code_hash"))?,
                ),
                default_aa_code_hash: H256::from_slice(
                    &batch
                        .default_aa_code_hash
                        .ok_or(L1BatchMetadataError::Incomplete("default_aa_code_hash"))?,
                ),
                evm_emulator_code_hash: batch
                    .evm_emulator_code_hash
                    .as_deref()
                    .map(H256::from_slice),
                protocol_version: batch
                    .protocol_version
                    .map(|v| (v as u16).try_into().unwrap()),
            },
            state_diffs_compressed: batch.compressed_state_diffs.unwrap_or_default(),
            events_queue_commitment: batch.events_queue_commitment.map(|v| H256::from_slice(&v)),
            bootloader_initial_content_commitment: batch
                .bootloader_initial_content_commitment
                .map(|v| H256::from_slice(&v)),
            da_blob_id: batch.blob_id.map(|s| s.into_bytes()),
            state_diff_hash: batch.state_diff_hash.map(|v| H256::from_slice(&v)),
            local_root: batch.local_root.map(|v| H256::from_slice(&v)),
            aggregation_root: batch.aggregation_root.map(|v| H256::from_slice(&v)),
            da_inclusion_data: batch.inclusion_data,
        })
    }
}

/// Partial projection of the columns corresponding to an unsealed [`L1BatchHeader`].
#[derive(Debug, Clone)]
pub(crate) struct UnsealedStorageL1Batch {
    pub number: i64,
    pub timestamp: i64,
    pub protocol_version: Option<i32>,
    pub fee_address: Vec<u8>,
    pub l1_gas_price: i64,
    pub l2_fair_gas_price: i64,
    pub fair_pubdata_price: Option<i64>,
}

impl From<UnsealedStorageL1Batch> for UnsealedL1BatchHeader {
    fn from(batch: UnsealedStorageL1Batch) -> Self {
        let protocol_version: Option<ProtocolVersionId> = batch
            .protocol_version
            .map(|v| (v as u16).try_into().unwrap());
        Self {
            number: L1BatchNumber(batch.number as u32),
            timestamp: batch.timestamp as u64,
            protocol_version,
            fee_address: Address::from_slice(&batch.fee_address),
            fee_input: BatchFeeInput::for_protocol_version(
                protocol_version.unwrap_or_else(ProtocolVersionId::last_potentially_undefined),
                batch.l2_fair_gas_price as u64,
                batch.fair_pubdata_price.map(|p| p as u64),
                batch.l1_gas_price as u64,
            ),
        }
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct StorageBlockDetails {
    pub number: i64,
    pub l1_batch_number: i64,
    pub timestamp: i64,
    pub l1_tx_count: i32,
    pub l2_tx_count: i32,
    pub root_hash: Option<Vec<u8>>,
    pub commit_tx_hash: Option<String>,
    pub committed_at: Option<NaiveDateTime>,
    pub prove_tx_hash: Option<String>,
    pub proven_at: Option<NaiveDateTime>,
    pub execute_tx_hash: Option<String>,
    pub executed_at: Option<NaiveDateTime>,
    // L1 gas price assumed in the corresponding batch
    pub l1_gas_price: i64,
    // L2 gas price assumed in the corresponding batch
    pub l2_fair_gas_price: i64,
    // Cost of publishing 1 byte (in wei).
    pub fair_pubdata_price: Option<i64>,
    pub bootloader_code_hash: Option<Vec<u8>>,
    pub default_aa_code_hash: Option<Vec<u8>>,
    pub evm_emulator_code_hash: Option<Vec<u8>>,
    pub fee_account_address: Vec<u8>,
    pub protocol_version: Option<i32>,
}

impl From<StorageBlockDetails> for api::BlockDetails {
    fn from(details: StorageBlockDetails) -> Self {
        let status = if details.number == 0 || details.execute_tx_hash.is_some() {
            api::BlockStatus::Verified
        } else {
            api::BlockStatus::Sealed
        };

        let base = api::BlockDetailsBase {
            timestamp: details.timestamp as u64,
            l1_tx_count: details.l1_tx_count as usize,
            l2_tx_count: details.l2_tx_count as usize,
            status,
            root_hash: details.root_hash.as_deref().map(H256::from_slice),
            commit_tx_hash: details
                .commit_tx_hash
                .as_deref()
                .map(|hash| H256::from_str(hash).expect("Incorrect commit_tx hash")),
            committed_at: details
                .committed_at
                .map(|committed_at| DateTime::from_naive_utc_and_offset(committed_at, Utc)),
            prove_tx_hash: details
                .prove_tx_hash
                .as_deref()
                .map(|hash| H256::from_str(hash).expect("Incorrect prove_tx hash")),
            proven_at: details
                .proven_at
                .map(|proven_at| DateTime::<Utc>::from_naive_utc_and_offset(proven_at, Utc)),
            execute_tx_hash: details
                .execute_tx_hash
                .as_deref()
                .map(|hash| H256::from_str(hash).expect("Incorrect execute_tx hash")),
            executed_at: details
                .executed_at
                .map(|executed_at| DateTime::<Utc>::from_naive_utc_and_offset(executed_at, Utc)),
            l1_gas_price: details.l1_gas_price as u64,
            l2_fair_gas_price: details.l2_fair_gas_price as u64,
            fair_pubdata_price: details.fair_pubdata_price.map(|x| x as u64),
            base_system_contracts_hashes: convert_base_system_contracts_hashes(
                details.bootloader_code_hash,
                details.default_aa_code_hash,
                details.evm_emulator_code_hash,
            ),
        };
        api::BlockDetails {
            base,
            number: L2BlockNumber(details.number as u32),
            l1_batch_number: L1BatchNumber(details.l1_batch_number as u32),
            operator_address: Address::from_slice(&details.fee_account_address),
            protocol_version: details
                .protocol_version
                .map(|v| (v as u16).try_into().unwrap()),
        }
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct StorageL1BatchDetails {
    pub number: i64,
    pub timestamp: i64,
    pub l1_tx_count: i32,
    pub l2_tx_count: i32,
    pub root_hash: Option<Vec<u8>>,
    pub commit_tx_hash: Option<String>,
    pub committed_at: Option<NaiveDateTime>,
    pub prove_tx_hash: Option<String>,
    pub proven_at: Option<NaiveDateTime>,
    pub execute_tx_hash: Option<String>,
    pub executed_at: Option<NaiveDateTime>,
    pub l1_gas_price: i64,
    pub l2_fair_gas_price: i64,
    pub fair_pubdata_price: Option<i64>,
    pub bootloader_code_hash: Option<Vec<u8>>,
    pub default_aa_code_hash: Option<Vec<u8>>,
    pub evm_emulator_code_hash: Option<Vec<u8>>,
}

impl From<StorageL1BatchDetails> for api::L1BatchDetails {
    fn from(details: StorageL1BatchDetails) -> Self {
        let status = if details.number == 0 || details.execute_tx_hash.is_some() {
            api::BlockStatus::Verified
        } else {
            api::BlockStatus::Sealed
        };

        let base = api::BlockDetailsBase {
            timestamp: details.timestamp as u64,
            l1_tx_count: details.l1_tx_count as usize,
            l2_tx_count: details.l2_tx_count as usize,
            status,
            root_hash: details.root_hash.as_deref().map(H256::from_slice),
            commit_tx_hash: details
                .commit_tx_hash
                .as_deref()
                .map(|hash| H256::from_str(hash).expect("Incorrect commit_tx hash")),
            committed_at: details
                .committed_at
                .map(|committed_at| DateTime::<Utc>::from_naive_utc_and_offset(committed_at, Utc)),
            prove_tx_hash: details
                .prove_tx_hash
                .as_deref()
                .map(|hash| H256::from_str(hash).expect("Incorrect prove_tx hash")),
            proven_at: details
                .proven_at
                .map(|proven_at| DateTime::<Utc>::from_naive_utc_and_offset(proven_at, Utc)),
            execute_tx_hash: details
                .execute_tx_hash
                .as_deref()
                .map(|hash| H256::from_str(hash).expect("Incorrect execute_tx hash")),
            executed_at: details
                .executed_at
                .map(|executed_at| DateTime::<Utc>::from_naive_utc_and_offset(executed_at, Utc)),
            l1_gas_price: details.l1_gas_price as u64,
            l2_fair_gas_price: details.l2_fair_gas_price as u64,
            fair_pubdata_price: details.fair_pubdata_price.map(|x| x as u64),
            base_system_contracts_hashes: convert_base_system_contracts_hashes(
                details.bootloader_code_hash,
                details.default_aa_code_hash,
                details.evm_emulator_code_hash,
            ),
        };
        api::L1BatchDetails {
            base,
            number: L1BatchNumber(details.number as u32),
        }
    }
}

pub(crate) struct StorageL2BlockHeader {
    pub number: i64,
    pub timestamp: i64,
    pub hash: Vec<u8>,
    pub l1_tx_count: i32,
    pub l2_tx_count: i32,
    pub fee_account_address: Vec<u8>,
    pub base_fee_per_gas: BigDecimal,
    pub l1_gas_price: i64,
    // L1 gas price assumed in the corresponding batch
    pub l2_fair_gas_price: i64,
    // L2 gas price assumed in the corresponding batch
    pub bootloader_code_hash: Option<Vec<u8>>,
    pub default_aa_code_hash: Option<Vec<u8>>,
    pub evm_emulator_code_hash: Option<Vec<u8>>,
    pub protocol_version: Option<i32>,

    pub fair_pubdata_price: Option<i64>,

    pub gas_per_pubdata_limit: i64,

    // The maximal number of virtual blocks that can be created with this miniblock.
    // If this value is greater than zero, then at least 1 will be created, but no more than
    // `min(virtual_blocks`, `miniblock_number - virtual_block_number`), i.e. making sure that virtual blocks
    // never go beyond the miniblock they are based on.
    pub virtual_blocks: i64,

    /// The formal value of the gas limit for the miniblock.
    /// This value should bound the maximal amount of gas that can be spent by transactions in the miniblock.
    pub gas_limit: Option<i64>,
    pub logs_bloom: Option<Vec<u8>>,
    pub l2_da_validator_address: Vec<u8>,
    pub pubdata_type: String,
}

impl From<StorageL2BlockHeader> for L2BlockHeader {
    fn from(row: StorageL2BlockHeader) -> Self {
        let protocol_version = row.protocol_version.map(|v| (v as u16).try_into().unwrap());

        let fee_input = protocol_version
            .filter(|version: &ProtocolVersionId| version.is_post_1_4_1())
            .map(|_| {
                BatchFeeInput::PubdataIndependent(PubdataIndependentBatchFeeModelInput {
                    fair_pubdata_price: row
                        .fair_pubdata_price
                        .expect("No fair pubdata price for 1.4.1 miniblock")
                        as u64,
                    fair_l2_gas_price: row.l2_fair_gas_price as u64,
                    l1_gas_price: row.l1_gas_price as u64,
                })
            })
            .unwrap_or_else(|| {
                BatchFeeInput::L1Pegged(L1PeggedBatchFeeModelInput {
                    fair_l2_gas_price: row.l2_fair_gas_price as u64,
                    l1_gas_price: row.l1_gas_price as u64,
                })
            });

        L2BlockHeader {
            number: L2BlockNumber(row.number as u32),
            timestamp: row.timestamp as u64,
            hash: H256::from_slice(&row.hash),
            l1_tx_count: row.l1_tx_count as u16,
            l2_tx_count: row.l2_tx_count as u16,
            fee_account_address: Address::from_slice(&row.fee_account_address),
            base_fee_per_gas: row.base_fee_per_gas.to_u64().unwrap(),
            batch_fee_input: fee_input,
            base_system_contracts_hashes: convert_base_system_contracts_hashes(
                row.bootloader_code_hash,
                row.default_aa_code_hash,
                row.evm_emulator_code_hash,
            ),
            gas_per_pubdata_limit: row.gas_per_pubdata_limit as u64,
            protocol_version,
            virtual_blocks: row.virtual_blocks as u32,
            gas_limit: row.gas_limit.unwrap_or(i64::from(LEGACY_BLOCK_GAS_LIMIT)) as u64,
            logs_bloom: row
                .logs_bloom
                .map(|b| Bloom::from_slice(&b))
                .unwrap_or_default(),
            pubdata_params: PubdataParams {
                l2_da_validator_address: Address::from_slice(&row.l2_da_validator_address),
                pubdata_type: L1BatchCommitmentMode::from_str(&row.pubdata_type).unwrap(),
            },
        }
    }
}

/// Information about L1 batch which a certain L2 block belongs to.
#[derive(Debug)]
pub struct ResolvedL1BatchForL2Block {
    /// L1 batch which the L2 block belongs to. `None` if the L2 block is not explicitly attached
    /// (i.e., its L1 batch is not sealed).
    pub block_l1_batch: Option<L1BatchNumber>,
    /// Pending (i.e., unsealed) L1 batch.
    pub pending_l1_batch: L1BatchNumber,
}

impl ResolvedL1BatchForL2Block {
    /// Returns the L1 batch number that the L2 block has now or will have in the future (provided
    /// that the node will operate correctly).
    pub fn expected_l1_batch(&self) -> L1BatchNumber {
        self.block_l1_batch.unwrap_or(self.pending_l1_batch)
    }
}
