use zksync_state::{ReadStorage, StoragePtr};

use super::ParallelTransaction;

// this struct is a partial replacemnt of the bootloader transaction processing and system context contract
pub struct ParallelExecutor<S: ReadStorage> {
    storage: StoragePtr<S>,
}

impl<S: ReadStorage> ParallelExecutor<S> {
    pub fn new(storage: StoragePtr<S>) -> Self {
        Self { storage }
    }

    pub fn append_transaction(&self, txs: Vec<ParallelTransaction>) {}

    pub fn process_transaction(&self, tx: ParallelTransaction) {
        self.set_l2_block();
        // bla bla
    }

    fn process_l2_transaction(&self, tx: ParallelTransaction) {}

    fn process_l1_transaction(&self, tx: ParallelTransaction) {
        todo!();
    }

    fn set_l2_block(&self) {}

    /// finalizes transaction processing by commiting final state changes to storage
    pub fn finalize(&self) {}
}
