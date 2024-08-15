use era_vm::state::StateSnapshot;
use zksync_types::H256;

use super::bootloader_state::BootloaderStateSnapshot;

#[derive(Debug, Clone)]
pub(crate) struct L2BlockSnapshot {
    /// The rolling hash of all the transactions in the miniblock
    pub(crate) txs_rolling_hash: H256,
    /// The number of transactions in the last L2 block
    pub(crate) txs_len: usize,
}

pub struct VmSnapshot {
    pub execution: StateSnapshot,
    pub bootloader_snapshot: BootloaderStateSnapshot,
    pub suspended_at: u16,
    pub gas_for_account_validation: u32,
}
