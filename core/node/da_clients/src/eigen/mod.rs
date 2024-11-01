mod blob_info;
mod client;
mod memstore;
mod sdk;

pub use self::client::EigenClient;

#[allow(clippy::all)]
pub(crate) mod disperser {
    include!("generated/disperser.rs");
}

#[allow(clippy::all)]
pub(crate) mod common {
    include!("generated/common.rs");
}
