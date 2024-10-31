use serde::Deserialize;

#[derive(Clone, Debug, PartialEq, Deserialize)]

pub enum EigenDAConfig {
    MemStore(MemStoreConfig),
    Disperser(DisperserConfig),
}

#[derive(Clone, Debug, PartialEq, Deserialize, Default)]
pub struct MemStoreConfig {
    pub max_blob_size_bytes: u64,
    /// Blob expiration time in seconds
    pub blob_expiration: u64,
    /// Latency in milliseconds for get operations
    pub get_latency: u64,
    /// Latency in milliseconds for put operations
    pub put_latency: u64,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Default)]
pub struct DisperserConfig {
    pub custom_quorum_numbers: Option<Vec<u32>>,
    pub account_id: Option<String>,
    pub disperser_rpc: String,
    pub eth_confirmation_depth: i32,
    pub eigenda_eth_rpc: String,
    pub eigenda_svc_manager_address: String,
    pub blob_size_limit: u64,
    pub status_query_timeout: u64,
    pub status_query_interval: u64,
    pub wait_for_finalization: bool,
    pub authenticaded: bool,
}
