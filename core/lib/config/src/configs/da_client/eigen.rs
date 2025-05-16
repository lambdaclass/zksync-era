use serde::Deserialize;
use zksync_basic_types::{secrets::PrivateKey, url::SensitiveUrl, Address};

pub const EIGEN_V1_CLIENT_NAME: &str = "V1";
pub const EIGEN_V2_CLIENT_NAME: &str = "V2";

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
pub enum PolynomialForm {
    Coeff,
    Eval,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub enum PointsSource {
    Path(String),
    /// g1_url, g2_url
    Url((String, String)),
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub enum VersionSpecificConfig {
    V1(V1Config),
    V2(V2Config),
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct V1Config {
    /// Block height needed to reach in order to consider the blob finalized
    /// a value less or equal to 0 means that the disperser will not wait for finalization
    pub settlement_layer_confirmation_depth: u32,
    /// Address of the service manager contract
    pub eigenda_svc_manager_address: Address,
    /// Wait for the blob to be finalized before returning the response
    pub wait_for_finalization: bool,
    /// Points source
    pub points_source: PointsSource,
    /// Custom quorum numbers
    pub custom_quorum_numbers: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct V2Config {
    /// Address of the EigenDA cert verifier
    pub cert_verifier_addr: Address,
    /// Blob version
    pub blob_version: u16,
    /// Polynomial form to disperse the blobs
    pub polynomial_form: PolynomialForm,
}

/// Configuration for the EigenDA remote disperser client.
///
/// This configuration is meant to be used by both the EigenDA V1 and V2 client.
/// The M0 stands for Milestone 0, an insecure integration, where the dispersal is not verified.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct EigenConfig {
    // Shared fields between V1 and V2
    /// URL of the Disperser RPC server
    pub disperser_rpc: String,
    /// URL of the Ethereum RPC server
    pub eigenda_eth_rpc: Option<SensitiveUrl>,
    /// Authenticated dispersal
    pub authenticated: bool,
    /// Config specific to each version
    pub version_specific: VersionSpecificConfig,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EigenSecrets {
    pub private_key: PrivateKey,
}
