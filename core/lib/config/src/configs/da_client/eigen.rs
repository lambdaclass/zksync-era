use serde::Deserialize;
use zksync_basic_types::{secrets::PrivateKey, url::SensitiveUrl, Address};

/// Configuration for the EigenDA remote disperser client.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct EigenConfig {
    /// URL of the Disperser RPC server
    pub disperser_rpc: String,
    /// Block height needed to reach in order to consider the blob finalized
    /// a value less or equal to 0 means that the disperser will not wait for finalization
    pub settlement_layer_confirmation_depth: u32,
    /// URL of the Ethereum RPC server
    pub eigenda_eth_rpc: Option<SensitiveUrl>,
    /// Address of the service manager contract
    pub eigenda_svc_manager_address: Address,
    /// Wait for the blob to be finalized before returning the response
    pub wait_for_finalization: bool,
    /// Authenticated dispersal
    pub authenticated: bool,
    /// Optional path to downloaded points directory
    pub points_dir: Option<String>,
    /// Url to the file containing the G1 point used for KZG
    pub g1_url: String,
    /// Url to the file containing the G2 point used for KZG
    pub g2_url: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EigenSecrets {
    pub private_key: PrivateKey,
}
