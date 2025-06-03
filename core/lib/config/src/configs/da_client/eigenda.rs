use serde::{Deserialize, Serialize};
use smart_config::{
    de::{FromSecretString, Optional, Serde, WellKnown},
    DescribeConfig, DeserializeConfig,
};
use zksync_basic_types::{secrets::PrivateKey, url::SensitiveUrl, Address};

/// Describes the different ways a polynomial may be represented
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Default, Serialize)]
pub enum PolynomialForm {
    /// Coeff is short for polynomial "coefficient form".
    /// The field elements represent the coefficients of the polynomial.
    #[default]
    #[serde(rename = "coeff")]
    Coeff,
    /// Eval is short for polynomial "evaluation form".
    /// The field elements represent the evaluation of the polynomial at roots of unity.
    #[serde(rename = "eval")]
    Eval,
}

impl PolynomialForm {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Coeff => "coeff",
            Self::Eval => "eval",
        }
    }
}

impl WellKnown for PolynomialForm {
    type Deserializer = Serde![str];
    const DE: Self::Deserializer = Serde![str];
}

/// Configuration for the EigenDA remote disperser client.
///
/// This configuration is meant to be used by the EigenDA V2 client.
/// It is an insecure integration, where the dispersal is not verified.
#[derive(Clone, Debug, PartialEq, Deserialize, DescribeConfig, DeserializeConfig)]
pub struct EigenDAConfig {
    /// URL of the Disperser RPC server
    pub disperser_rpc: String,
    /// URL of the Ethereum RPC server
    #[config(secret, with = Optional(Serde![str]))]
    pub eigenda_eth_rpc: Option<SensitiveUrl>,
    /// Authenticated dispersal
    #[config(default_t = true)]
    pub authenticated: bool,
    /// Address of the EigenDA cert verifier
    pub cert_verifier_addr: Address,
    /// Blob version
    pub blob_version: u16,
    /// Polynomial form to disperse the blobs
    #[serde(default)]
    pub polynomial_form: PolynomialForm,
}

/// Configuration for the EigenDA secrets.
#[derive(Clone, Debug, DescribeConfig, DeserializeConfig)]
pub struct EigenDASecrets {
    /// Private key used for dispersing the blobs
    #[config(with = FromSecretString)]
    pub private_key: PrivateKey,
}
