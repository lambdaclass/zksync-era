mod blob_info;
mod client;
mod eigendaservicemanager;
mod memstore;
mod sdk;
mod verifier;

use std::sync::Arc;

use memstore::MemStore;
use sdk::RawEigenClient;

pub use self::client::EigenClient;

#[allow(clippy::all)]
pub(crate) mod disperser {
    include!("generated/disperser.rs");
}

#[allow(clippy::all)]
pub(crate) mod common {
    include!("generated/common.rs");
}

#[derive(Clone, Debug)]
pub(crate) enum Disperser {
    Remote(Arc<RawEigenClient>),
    Memory(Arc<MemStore>),
}
