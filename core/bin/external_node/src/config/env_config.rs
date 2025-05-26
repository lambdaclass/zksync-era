//! Legacy env var parsing logic. Will be removed after the transition to `smart-config` is complete for EN.

use std::{env, num::ParseIntError, str::FromStr, time::Duration};

use anyhow::Context;
use serde::de::DeserializeOwned;
use zksync_config::{
    configs::{
        da_client::{
            avail::{AvailClientConfig, AvailSecrets},
            celestia::CelestiaSecrets,
            eigenda::EigenDASecrets,
        },
        DataAvailabilitySecrets,
    },
    AvailConfig, DAClientConfig, EigenDAConfig,
};
use zksync_types::{
    secrets::{APIKey, PrivateKey, SeedPhrase},
    url::SensitiveUrl,
    Address,
};

fn envy_load<T: DeserializeOwned>(name: &str, prefix: &str) -> anyhow::Result<T> {
    envy::prefixed(prefix)
        .from_env()
        .map_err(|e| anyhow::anyhow!("Failed to load {} from env: {}", name, e))
}

const AVAIL_CLIENT_CONFIG_NAME: &str = "Avail";
const CELESTIA_CLIENT_CONFIG_NAME: &str = "Celestia";
const EIGENDA_CLIENT_CONFIG_NAME: &str = "EigenDA";
const OBJECT_STORE_CLIENT_CONFIG_NAME: &str = "ObjectStore";
const NO_DA_CLIENT_CONFIG_NAME: &str = "NoDA";
const AVAIL_GAS_RELAY_CLIENT_NAME: &str = "GasRelay";
const AVAIL_FULL_CLIENT_NAME: &str = "FullClient";

const EIGENDA_VERSION_V1: &str = "V1";
const EIGENDA_VERSION_V2: &str = "V2";
const EIGENDA_POINTS_PATH: &str = "Path";
const EIGENDA_POINTS_URL: &str = "Url";

pub fn da_client_config_from_env(prefix: &str) -> anyhow::Result<DAClientConfig> {
    let client_tag = env::var(format!("{}CLIENT", prefix))?;
    let config = match client_tag.as_str() {
        AVAIL_CLIENT_CONFIG_NAME => DAClientConfig::Avail(AvailConfig {
            bridge_api_url: env::var(format!("{}BRIDGE_API_URL", prefix))?,
            timeout: Duration::from_millis(env::var(format!("{}TIMEOUT_MS", prefix))?.parse()?),
            config: match env::var(format!("{}AVAIL_CLIENT_TYPE", prefix))?.as_str() {
                AVAIL_FULL_CLIENT_NAME => {
                    AvailClientConfig::FullClient(envy_load("da_avail_full_client", prefix)?)
                }
                AVAIL_GAS_RELAY_CLIENT_NAME => {
                    AvailClientConfig::GasRelay(envy_load("da_avail_gas_relay", prefix)?)
                }
                _ => anyhow::bail!("Unknown Avail DA client type"),
            },
        }),
        CELESTIA_CLIENT_CONFIG_NAME => {
            DAClientConfig::Celestia(envy_load("da_celestia_config", prefix)?)
        }
        EIGENDA_CLIENT_CONFIG_NAME => DAClientConfig::EigenDA(EigenDAConfig {
            disperser_rpc: env::var(format!("{}DISPERSER_RPC", prefix))?,
            eigenda_eth_rpc: match env::var(format!("{}EIGENDA_ETH_RPC", prefix)) {
                // Use a specific L1 RPC URL for the EigenDA client.
                Ok(url) => Some(SensitiveUrl::from_str(&url)?),
                // Err means that the environment variable is not set.
                // Use zkSync default L1 RPC for the EigenDA client.
                Err(_) => None,
            },
            authenticated: env::var(format!("{}AUTHENTICATED", prefix))?.parse()?,
            version: match env::var(format!("{}VERSION", prefix))?.as_str() {
                EIGENDA_VERSION_V1 => zksync_config::configs::da_client::eigenda::Version::V1,
                EIGENDA_VERSION_V2 => zksync_config::configs::da_client::eigenda::Version::V2,
                _ => anyhow::bail!("Unknown EigenDA version"),
            },
            settlement_layer_confirmation_depth: env::var(format!(
                "{}SETTLEMENT_LAYER_CONFIRMATION_DEPTH",
                prefix
            ))?
            .parse()?,
            eigenda_svc_manager_address: Address::from_str(&env::var(format!(
                "{}EIGENDA_SVC_MANAGER_ADDRESS",
                prefix
            ))?)?,
            wait_for_finalization: env::var(format!("{}WAIT_FOR_FINALIZATION", prefix))?.parse()?,
            points: match env::var(format!("{}POINTS_SOURCE", prefix))?.as_str() {
                EIGENDA_POINTS_PATH => {
                    zksync_config::configs::da_client::eigenda::PointsSource::Path {
                        path: env::var(format!("{}POINTS_PATH", prefix))?,
                    }
                }
                EIGENDA_POINTS_URL => {
                    zksync_config::configs::da_client::eigenda::PointsSource::Url {
                        g1_url: env::var(format!("{}POINTS_LINK_G1", prefix))?,
                        g2_url: env::var(format!("{}POINTS_LINK_G2", prefix))?,
                    }
                }
                _ => anyhow::bail!("Unknown Eigen points type"),
            },
            custom_quorum_numbers: match env::var(format!("{}CUSTOM_QUORUM_NUMBERS", prefix)) {
                Ok(numbers) => numbers
                    .split(',')
                    .map(|s| s.parse().map_err(|e: ParseIntError| anyhow::anyhow!(e)))
                    .collect::<anyhow::Result<Vec<_>>>()?,
                Err(_) => vec![],
            },
            cert_verifier_addr: Address::from_str(&env::var(format!(
                "{}EIGENDA_CERT_VERIFIER_ADDRESS",
                prefix
            ))?)?,
            blob_version: env::var(format!("{}BLOB_VERSION", prefix))?
                .parse()
                .context("EigenDA blob version not found")?,
            polynomial_form: match env::var(format!("{}POLYNOMIAL_FORM", prefix))?.as_str() {
                "Coeff" => zksync_config::configs::da_client::eigenda::PolynomialForm::Coeff,
                "Eval" => zksync_config::configs::da_client::eigenda::PolynomialForm::Eval,
                _ => anyhow::bail!("Unknown Eigen polynomial form"),
            },
        }),
        OBJECT_STORE_CLIENT_CONFIG_NAME => {
            DAClientConfig::ObjectStore(envy_load("da_object_store", prefix)?)
        }
        NO_DA_CLIENT_CONFIG_NAME => DAClientConfig::NoDA,
        _ => anyhow::bail!("Unknown DA client name: {}", client_tag),
    };

    Ok(config)
}

pub fn da_client_secrets_from_env(prefix: &str) -> anyhow::Result<DataAvailabilitySecrets> {
    let client_tag = env::var(format!("{}CLIENT", prefix))?;
    let secrets = match client_tag.as_str() {
        AVAIL_CLIENT_CONFIG_NAME => {
            let seed_phrase = env::var(format!("{}SECRETS_SEED_PHRASE", prefix))
                .ok()
                .map(|s| SeedPhrase(s.into()));
            let gas_relay_api_key = env::var(format!("{}SECRETS_GAS_RELAY_API_KEY", prefix))
                .ok()
                .map(|s| APIKey(s.into()));
            if seed_phrase.is_none() && gas_relay_api_key.is_none() {
                anyhow::bail!("No secrets provided for Avail DA client");
            }
            DataAvailabilitySecrets::Avail(AvailSecrets {
                seed_phrase,
                gas_relay_api_key,
            })
        }
        CELESTIA_CLIENT_CONFIG_NAME => {
            let private_key = env::var(format!("{}SECRETS_PRIVATE_KEY", prefix))
                .context("Celestia private key not found")?;
            DataAvailabilitySecrets::Celestia(CelestiaSecrets {
                private_key: PrivateKey(private_key.into()),
            })
        }
        EIGENDA_CLIENT_CONFIG_NAME => {
            let private_key = env::var(format!("{}SECRETS_PRIVATE_KEY", prefix))
                .context("Eigen private key not found")?;
            DataAvailabilitySecrets::EigenDA(EigenDASecrets {
                private_key: PrivateKey(private_key.into()),
            })
        }

        _ => anyhow::bail!("Unknown DA client name: {}", client_tag),
    };

    Ok(secrets)
}
