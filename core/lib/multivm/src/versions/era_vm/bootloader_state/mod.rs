pub mod l2_block;
mod snapshot;
mod state;
pub mod tx;

pub(crate) mod utils;
pub(crate) use snapshot::BootloaderStateSnapshot;
pub use state::BootloaderState;
