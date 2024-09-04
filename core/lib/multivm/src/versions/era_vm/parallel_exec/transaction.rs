use zksync_types::Transaction;

use crate::era_vm::bootloader_state::l2_block::BootloaderL2Block;

#[derive(Clone, Debug)]
pub struct ParallelTransaction {
    pub tx: Transaction,
    pub refund: u64,
    pub with_compression: bool,
    // the l2 block this transaction belongs to
    pub l2_block: BootloaderL2Block,
}

impl ParallelTransaction {
    pub fn new(
        tx: Transaction,
        refund: u64,
        with_compression: bool,
        l2_block: BootloaderL2Block,
    ) -> Self {
        Self {
            tx,
            refund,
            with_compression,
            l2_block,
        }
    }
}
