use zksync_types::Transaction;

pub struct ParallelTransaction {
    pub tx: Transaction,
    pub refund: u64,
    pub with_compression: bool,
}

impl ParallelTransaction {
    pub fn new(tx: Transaction, refund: u64, with_compression: bool) -> Self {
        Self {
            tx,
            refund,
            with_compression,
        }
    }
}
