use std::env;

use zksync_config::{
    configs::{
        da_client::{
            avail::{
                AvailClientConfig, AvailSecrets, AVAIL_FULL_CLIENT_NAME,
                AVAIL_GAS_RELAY_CLIENT_NAME,
            },
            celestia::CelestiaSecrets,
            eigen::{EigenSecrets, EIGEN_DISPERSER_CLIENT_NAME, EIGEN_MEMSTORE_CLIENT_NAME},
            DAClientConfig, AVAIL_CLIENT_CONFIG_NAME, CELESTIA_CLIENT_CONFIG_NAME,
            EIGEN_CLIENT_CONFIG_NAME, OBJECT_STORE_CLIENT_CONFIG_NAME,
        },
        secrets::DataAvailabilitySecrets,
        AvailConfig,
    },
    EigenConfig,
};

use crate::{envy_load, FromEnv};

impl FromEnv for DAClientConfig {
    fn from_env() -> anyhow::Result<Self> {
        let client_tag = env::var("DA_CLIENT")?;
        let config = match client_tag.as_str() {
            AVAIL_CLIENT_CONFIG_NAME => Self::Avail(AvailConfig {
                bridge_api_url: env::var("DA_BRIDGE_API_URL").ok().unwrap(),
                timeout_ms: env::var("DA_TIMEOUT_MS")?.parse()?,
                config: match env::var("DA_AVAIL_CLIENT_TYPE")?.as_str() {
                    AVAIL_FULL_CLIENT_NAME => {
                        AvailClientConfig::FullClient(envy_load("da_avail_full_client", "DA_")?)
                    }
                    AVAIL_GAS_RELAY_CLIENT_NAME => {
                        AvailClientConfig::GasRelay(envy_load("da_avail_gas_relay", "DA_")?)
                    }
                    _ => anyhow::bail!("Unknown Avail DA client type"),
                },
            }),
            CELESTIA_CLIENT_CONFIG_NAME => Self::Celestia(envy_load("da_celestia_config", "DA_")?),
            EIGEN_CLIENT_CONFIG_NAME => match env::var("DA_EIGEN_CLIENT_TYPE")?.as_str() {
                EIGEN_DISPERSER_CLIENT_NAME => Self::Eigen(EigenConfig::Disperser(envy_load(
                    "da_eigen_config_disperser",
                    "DA_",
                )?)),
                EIGEN_MEMSTORE_CLIENT_NAME => Self::Eigen(EigenConfig::MemStore(envy_load(
                    "da_eigen_config_memstore",
                    "DA_",
                )?)),
                _ => anyhow::bail!("Unknown Eigen DA client type"),
            },
            OBJECT_STORE_CLIENT_CONFIG_NAME => {
                Self::ObjectStore(envy_load("da_object_store", "DA_")?)
            }
            _ => anyhow::bail!("Unknown DA client name: {}", client_tag),
        };

        Ok(config)
    }
}

impl FromEnv for DataAvailabilitySecrets {
    fn from_env() -> anyhow::Result<Self> {
        let client_tag = std::env::var("DA_CLIENT")?;
        let secrets = match client_tag.as_str() {
            AVAIL_CLIENT_CONFIG_NAME => {
                let seed_phrase: Option<zksync_basic_types::secrets::SeedPhrase> =
                    env::var("DA_SECRETS_SEED_PHRASE")
                        .ok()
                        .map(|s| s.parse().unwrap());
                let gas_relay_api_key: Option<zksync_basic_types::secrets::APIKey> =
                    env::var("DA_SECRETS_GAS_RELAY_API_KEY")
                        .ok()
                        .map(|s| s.parse().unwrap());
                if seed_phrase.is_none() && gas_relay_api_key.is_none() {
                    anyhow::bail!("No secrets provided for Avail DA client");
                }
                Self::Avail(AvailSecrets {
                    seed_phrase,
                    gas_relay_api_key,
                })
            }
            CELESTIA_CLIENT_CONFIG_NAME => {
                let private_key = env::var("DA_SECRETS_PRIVATE_KEY")
                    .map_err(|e| anyhow::format_err!("Celestia private key not found: {}", e))?
                    .parse()
                    .map_err(|e| anyhow::format_err!("failed to parse the private key: {}", e))?;
                Self::Celestia(CelestiaSecrets { private_key })
            }
            EIGEN_CLIENT_CONFIG_NAME => {
                let private_key = env::var("DA_SECRETS_PRIVATE_KEY")
                    .map_err(|e| anyhow::format_err!("Eigen private key not found: {}", e))?
                    .parse()
                    .map_err(|e| anyhow::format_err!("failed to parse the private key: {}", e))?;
                Self::Eigen(EigenSecrets { private_key })
            }

            _ => anyhow::bail!("Unknown DA client name: {}", client_tag),
        };

        Ok(secrets)
    }
}

#[cfg(test)]
mod tests {
    use zksync_config::{
        configs::{
            da_client::{
                avail::{AvailClientConfig, AvailDefaultConfig},
                eigen::{DisperserConfig, MemStoreConfig},
                DAClientConfig::{self, ObjectStore},
            },
            object_store::ObjectStoreMode::GCS,
        },
        AvailConfig, CelestiaConfig, EigenConfig, ObjectStoreConfig,
    };

    use super::*;
    use crate::test_utils::EnvMutex;

    static MUTEX: EnvMutex = EnvMutex::new();

    fn expected_object_store_da_client_config(url: String, max_retries: u16) -> DAClientConfig {
        ObjectStore(ObjectStoreConfig {
            mode: GCS {
                bucket_base_url: url,
            },
            max_retries,
            local_mirror_path: None,
        })
    }

    #[test]
    fn from_env_object_store() {
        let mut lock = MUTEX.lock();
        let config = r#"
            DA_CLIENT="ObjectStore"

            DA_BUCKET_BASE_URL="sometestpath"
            DA_MODE="GCS"
            DA_MAX_RETRIES="5"
        "#;
        lock.set_env(config);
        let actual = DAClientConfig::from_env().unwrap();
        assert_eq!(
            actual,
            expected_object_store_da_client_config("sometestpath".to_string(), 5)
        );
    }

    fn expected_avail_da_layer_config(
        api_node_url: &str,
        bridge_api_url: &str,
        app_id: u32,
        timeout_ms: usize,
    ) -> DAClientConfig {
        DAClientConfig::Avail(AvailConfig {
            bridge_api_url: bridge_api_url.to_string(),
            timeout_ms,
            config: AvailClientConfig::FullClient(AvailDefaultConfig {
                api_node_url: api_node_url.to_string(),
                app_id,
            }),
        })
    }

    #[test]
    fn from_env_avail_client() {
        let mut lock = MUTEX.lock();
        let config = r#"
            DA_CLIENT="Avail"
            DA_AVAIL_CLIENT_TYPE="FullClient"

            DA_BRIDGE_API_URL="localhost:54321"
            DA_TIMEOUT_MS="2000"

            DA_API_NODE_URL="localhost:12345"
            DA_APP_ID="1"
        "#;

        lock.set_env(config);

        let actual = DAClientConfig::from_env().unwrap();
        assert_eq!(
            actual,
            expected_avail_da_layer_config(
                "localhost:12345",
                "localhost:54321",
                "1".parse::<u32>().unwrap(),
                "2000".parse::<usize>().unwrap(),
            )
        );
    }

    #[test]
    fn from_env_avail_secrets() {
        let mut lock = MUTEX.lock();
        let config = r#"
            DA_CLIENT="Avail"
            DA_SECRETS_SEED_PHRASE="bottom drive obey lake curtain smoke basket hold race lonely fit walk"
        "#;

        lock.set_env(config);

        let (actual_seed, actual_key) = match DataAvailabilitySecrets::from_env().unwrap() {
            DataAvailabilitySecrets::Avail(avail) => (avail.seed_phrase, avail.gas_relay_api_key),
            _ => {
                panic!("Avail config expected")
            }
        };
        assert_eq!(
            (actual_seed.unwrap(), actual_key),
            (
                "bottom drive obey lake curtain smoke basket hold race lonely fit walk"
                    .parse()
                    .unwrap(),
                None
            )
        );
    }

    fn expected_celestia_da_layer_config(
        api_node_url: &str,
        namespace: &str,
        chain_id: &str,
        timeout_ms: u64,
    ) -> DAClientConfig {
        DAClientConfig::Celestia(CelestiaConfig {
            api_node_url: api_node_url.to_string(),
            namespace: namespace.to_string(),
            chain_id: chain_id.to_string(),
            timeout_ms,
        })
    }

    #[test]
    fn from_env_celestia_client() {
        let mut lock = MUTEX.lock();
        let config = r#"
            DA_CLIENT="Celestia"
            DA_API_NODE_URL="localhost:12345"
            DA_NAMESPACE="0x1234567890abcdef"
            DA_CHAIN_ID="mocha-4"
            DA_TIMEOUT_MS="7000"
        "#;
        lock.set_env(config);

        let actual = DAClientConfig::from_env().unwrap();
        assert_eq!(
            actual,
            expected_celestia_da_layer_config(
                "localhost:12345",
                "0x1234567890abcdef",
                "mocha-4",
                7000
            )
        );
    }

    #[test]
    fn from_env_eigen_client() {
        let mut lock = MUTEX.lock();
        let config = r#"
            DA_CLIENT="Eigen"
            DA_EIGEN_CLIENT_TYPE="MemStore"
            DA_MAX_BLOB_SIZE_BYTES=10
            DA_BLOB_EXPIRATION=20
            DA_GET_LATENCY=30
            DA_PUT_LATENCY=40
        "#;
        lock.set_env(config);

        let actual = DAClientConfig::from_env().unwrap();
        assert_eq!(
            actual,
            DAClientConfig::Eigen(EigenConfig::MemStore(MemStoreConfig {
                max_blob_size_bytes: 10,
                blob_expiration: 20,
                get_latency: 30,
                put_latency: 40,
            }))
        );
    }

    #[test]
    fn from_env_eigen_client_remote() {
        let mut lock = MUTEX.lock();
        let config = r#"
            DA_CLIENT="Eigen"
            DA_EIGEN_CLIENT_TYPE="Disperser"
            DA_DISPERSER_RPC="http://localhost:8080"
            DA_ETH_CONFIRMATION_DEPTH=0
            DA_EIGENDA_ETH_RPC="http://localhost:8545"
            DA_EIGENDA_SVC_MANAGER_ADDRESS="0x123"
            DA_BLOB_SIZE_LIMIT=1000
            DA_STATUS_QUERY_TIMEOUT=2
            DA_STATUS_QUERY_INTERVAL=3
            DA_WAIT_FOR_FINALIZATION=true
            DA_AUTHENTICATED=false
        "#;
        lock.set_env(config);

        let actual = DAClientConfig::from_env().unwrap();
        assert_eq!(
            actual,
            DAClientConfig::Eigen(EigenConfig::Disperser(DisperserConfig {
                custom_quorum_numbers: None,
                disperser_rpc: "http://localhost:8080".to_string(),
                eth_confirmation_depth: 0,
                eigenda_eth_rpc: "http://localhost:8545".to_string(),
                eigenda_svc_manager_address: "0x123".to_string(),
                blob_size_limit: 1000,
                status_query_timeout: 2,
                status_query_interval: 3,
                wait_for_finalization: true,
                authenticated: false,
            }))
        );
    }

    #[test]
    fn from_env_celestia_secrets() {
        let mut lock = MUTEX.lock();
        let config = r#"
            DA_CLIENT="Celestia"
            DA_SECRETS_PRIVATE_KEY="f55baf7c0e4e33b1d78fbf52f069c426bc36cff1aceb9bc8f45d14c07f034d73"
        "#;

        lock.set_env(config);

        let DataAvailabilitySecrets::Celestia(actual) =
            DataAvailabilitySecrets::from_env().unwrap()
        else {
            panic!("expected Celestia config")
        };
        assert_eq!(
            actual.private_key,
            "f55baf7c0e4e33b1d78fbf52f069c426bc36cff1aceb9bc8f45d14c07f034d73"
                .parse()
                .unwrap()
        );
    }
}
