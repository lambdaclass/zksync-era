use std::{error::Error, str::FromStr};

use eigenda_client_rs::{client::GetBlobData, EigenClient};
use subxt_signer::ExposeSecret;
use zksync_config::{configs::da_client::eigen::EigenSecrets, EigenConfig};
use zksync_da_client::{
    types::{DAError, DispatchResponse, InclusionData},
    DataAvailabilityClient,
};
use zksync_dal::{ConnectionPool, Core, CoreDal};

use crate::utils::to_retriable_da_error;

// We can't implement DataAvailabilityClient for an outside struct, so it is needed to defined this intermediate struct
#[derive(Debug, Clone)]
pub struct EigenDAClient {
    client: EigenClient,
}
impl EigenDAClient {
    pub async fn new(
        config: EigenConfig,
        secrets: EigenSecrets,
        pool: ConnectionPool<Core>,
    ) -> anyhow::Result<Self> {
        let eigen_config = eigenda_client_rs::config::EigenConfig {
            disperser_rpc: config.disperser_rpc,
            settlement_layer_confirmation_depth: config.settlement_layer_confirmation_depth,
            eigenda_eth_rpc: config.eigenda_eth_rpc.ok_or(anyhow::anyhow!(
                "eigenda_eth_rpc is required for EigenClient"
            ))?,
            eigenda_svc_manager_address: config.eigenda_svc_manager_address,
            wait_for_finalization: config.wait_for_finalization,
            authenticated: config.authenticated,
            g1_url: config.g1_url,
            g2_url: config.g2_url,
        };
        let private_key =
            eigenda_client_rs::config::PrivateKey::from_str(secrets.private_key.0.expose_secret())
                .map_err(|_| anyhow::anyhow!("Invalid private key"))?;
        let eigen_secrets = eigenda_client_rs::config::EigenSecrets { private_key };
        let get_blob_data = GetBlobFromDB { pool };
        let client = EigenClient::new(eigen_config, eigen_secrets, Box::new(get_blob_data))
            .await
            .map_err(|e| anyhow::anyhow!("Eigen client Error: {:?}", e))?;
        Ok(Self { client })
    }
}

#[derive(Debug, Clone)]
pub struct GetBlobFromDB {
    pool: ConnectionPool<Core>,
}

#[async_trait::async_trait]
impl GetBlobData for GetBlobFromDB {
    async fn get_blob_data(
        &self,
        input: &str,
    ) -> Result<Option<Vec<u8>>, Box<dyn Error + Send + Sync>> {
        let mut conn = self.pool.connection_tagged("eigen_client").await?;
        let batch = conn
            .data_availability_dal()
            .get_blob_data_by_blob_id(input)
            .await?;
        Ok(batch.map(|b| b.pubdata))
    }

    fn clone_boxed(&self) -> Box<dyn GetBlobData> {
        Box::new(self.clone())
    }
}

#[async_trait::async_trait]
impl DataAvailabilityClient for EigenDAClient {
    async fn dispatch_blob(
        &self,
        _: u32, // batch number
        data: Vec<u8>,
    ) -> Result<DispatchResponse, DAError> {
        let blob_id = self
            .client
            .dispatch_blob(data)
            .await
            .map_err(to_retriable_da_error)?;

        Ok(DispatchResponse::from(blob_id))
    }

    async fn get_inclusion_data(&self, blob_id: &str) -> Result<Option<InclusionData>, DAError> {
        let inclusion_data = self
            .client
            .get_inclusion_data(blob_id)
            .await
            .map_err(to_retriable_da_error)?;
        if let Some(inclusion_data) = inclusion_data {
            Ok(Some(InclusionData {
                data: inclusion_data,
            }))
        } else {
            Ok(None)
        }
    }

    fn clone_boxed(&self) -> Box<dyn DataAvailabilityClient> {
        Box::new(self.clone())
    }

    fn blob_size_limit(&self) -> Option<usize> {
        self.client.blob_size_limit()
    }
}
