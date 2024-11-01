use std::{str::FromStr, sync::Arc};

use async_trait::async_trait;
use secp256k1::SecretKey;
use subxt_signer::ExposeSecret;
use zksync_config::{configs::da_client::eigen::EigenSecrets, EigenConfig};
use zksync_da_client::{
    types::{DAError, DispatchResponse, InclusionData},
    DataAvailabilityClient,
};

use super::{memstore::MemStore, sdk::RawEigenClient, Disperser};
use crate::utils::to_non_retriable_da_error;

#[derive(Debug, Clone)]
pub struct EigenClient {
    client: Disperser,
}

impl EigenClient {
    pub async fn new(config: EigenConfig, secrets: EigenSecrets) -> anyhow::Result<Self> {
        let private_key = SecretKey::from_str(secrets.private_key.0.expose_secret().as_str())
            .map_err(|e| anyhow::anyhow!("Failed to parse private key: {}", e))?;

        let disperser: Disperser = match config.clone() {
            EigenConfig::Disperser(config) => {
                let client = RawEigenClient::new(private_key, config).await?;
                Disperser::Remote(Arc::new(client))
            }
            EigenConfig::MemStore(config) => Disperser::Memory(MemStore::new(config)),
        };
        Ok(Self { client: disperser })
    }
}

#[async_trait]
impl DataAvailabilityClient for EigenClient {
    async fn dispatch_blob(
        &self,
        _: u32, // batch number
        data: Vec<u8>,
    ) -> Result<DispatchResponse, DAError> {
        let blob_id = match &self.client {
            Disperser::Remote(remote_disperser) => remote_disperser
                .dispatch_blob(data)
                .await
                .map_err(to_non_retriable_da_error)?,
            Disperser::Memory(memstore) => memstore
                .clone()
                .put_blob(data)
                .await
                .map_err(to_non_retriable_da_error)?,
        };

        Ok(DispatchResponse::from(blob_id))
    }

    async fn get_inclusion_data(&self, _: &str) -> Result<Option<InclusionData>, DAError> {
        Ok(Some(InclusionData { data: vec![] }))
    }

    fn clone_boxed(&self) -> Box<dyn DataAvailabilityClient> {
        Box::new(self.clone())
    }

    fn blob_size_limit(&self) -> Option<usize> {
        Some(1920 * 1024) // 2mb - 128kb as a buffer
    }
}

#[cfg(test)]
impl EigenClient {
    pub async fn get_blob_data(&self, blob_id: &str) -> anyhow::Result<Option<Vec<u8>>, DAError> {
        match &self.client {
            Disperser::Remote(remote_client) => remote_client.get_blob_data(blob_id).await,
            Disperser::Memory(memstore) => memstore.clone().get_blob_data(blob_id).await,
        }
    }
}

pub fn to_retriable_error(error: anyhow::Error) -> DAError {
    DAError {
        error,
        is_retriable: true,
    }
}
#[cfg(test)]
mod tests {
    use zksync_config::configs::da_client::eigen::{DisperserConfig, MemStoreConfig};
    use zksync_types::secrets::PrivateKey;

    use super::*;
    use crate::eigen::blob_info::BlobInfo;

    #[tokio::test]
    async fn test_eigenda_memory_disperser() {
        let config = EigenConfig::MemStore(MemStoreConfig {
            max_blob_size_bytes: 2 * 1024 * 1024, // 2MB,
            blob_expiration: 60 * 2,
            get_latency: 0,
            put_latency: 0,
        });
        let secrets = EigenSecrets {
            private_key: PrivateKey::from_str(
                "d08aa7ae1bb5ddd46c3c2d8cdb5894ab9f54dec467233686ca42629e826ac4c6",
            )
            .unwrap(),
        };
        let client = EigenClient::new(config, secrets).await.unwrap();
        let data = vec![1u8; 100];
        let result = client.dispatch_blob(0, data.clone()).await.unwrap();

        let blob_info: BlobInfo =
            rlp::decode(&hex::decode(result.blob_id.clone()).unwrap()).unwrap();
        // TODO: once get inclusion data is added, check it

        let retrieved_data = client.get_blob_data(&result.blob_id).await.unwrap();
        assert_eq!(retrieved_data.unwrap(), data);
    }

    #[tokio::test]
    async fn test_non_auth_dispersal() {
        let config = EigenConfig::Disperser(DisperserConfig {
            custom_quorum_numbers: None,
            disperser_rpc: "https://disperser-holesky.eigenda.xyz:443".to_string(),
            eth_confirmation_depth: -1,
            eigenda_eth_rpc: String::default(),
            eigenda_svc_manager_address: "0xD4A7E1Bd8015057293f0D0A557088c286942e84b".to_string(),
            blob_size_limit: 2 * 1024 * 1024, // 2MB
            status_query_timeout: 1800,       // 30 minutes
            status_query_interval: 5,         // 5 seconds
            wait_for_finalization: false,
            authenticaded: false,
        });
        let secrets = EigenSecrets {
            private_key: PrivateKey::from_str(
                "d08aa7ae1bb5ddd46c3c2d8cdb5894ab9f54dec467233686ca42629e826ac4c6",
            )
            .unwrap(),
        };
        let client = EigenClient::new(config, secrets).await.unwrap();
        let data = vec![1; 20];
        let result = client.dispatch_blob(0, data.clone()).await.unwrap();
        let blob_info: BlobInfo =
            rlp::decode(&hex::decode(result.blob_id.clone()).unwrap()).unwrap();
        // TODO: once get inclusion data is added, check it
        let retrieved_data = client.get_blob_data(&result.blob_id).await.unwrap();
        assert_eq!(retrieved_data.unwrap(), data);
    }
    #[tokio::test]
    async fn test_auth_dispersal() {
        let config = EigenConfig::Disperser(DisperserConfig {
            custom_quorum_numbers: None,
            disperser_rpc: "https://disperser-holesky.eigenda.xyz:443".to_string(),
            eth_confirmation_depth: -1,
            eigenda_eth_rpc: String::default(),
            eigenda_svc_manager_address: "0xD4A7E1Bd8015057293f0D0A557088c286942e84b".to_string(),
            blob_size_limit: 2 * 1024 * 1024, // 2MB
            status_query_timeout: 1800,       // 30 minutes
            status_query_interval: 5,         // 5 seconds
            wait_for_finalization: false,
            authenticaded: true,
        });
        let secrets = EigenSecrets {
            private_key: PrivateKey::from_str(
                "d08aa7ae1bb5ddd46c3c2d8cdb5894ab9f54dec467233686ca42629e826ac4c6",
            )
            .unwrap(),
        };
        let client = EigenClient::new(config, secrets).await.unwrap();
        let data = vec![1; 20];
        let result = client.dispatch_blob(0, data.clone()).await.unwrap();
        let blob_info: BlobInfo =
            rlp::decode(&hex::decode(result.blob_id.clone()).unwrap()).unwrap();
        // TODO: once get inclusion data is added, check it
        let retrieved_data = client.get_blob_data(&result.blob_id).await.unwrap();
        assert_eq!(retrieved_data.unwrap(), data);
    }
}
