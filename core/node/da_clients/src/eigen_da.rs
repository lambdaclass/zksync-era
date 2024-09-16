use std::fmt::Debug;

use async_trait::async_trait;
use zksync_config::configs::da_client::eigen_da::EigenDAConfig;
use zksync_da_client::{
    types::{self, InclusionData},
    DataAvailabilityClient,
};

#[derive(Clone, Debug)]
pub struct EigenDAClient {
    client: reqwest::Client,
    config: EigenDAConfig,
}

impl EigenDAClient {
    pub const BLOB_SIZE_LIMIT_IN_BYTES: usize = 10 * 1024 * 1024; // 10MB

    pub async fn new(config: EigenDAConfig) -> anyhow::Result<Self> {
        Ok(Self {
            client: reqwest::Client::new(),
            config,
        })
    }
}

#[async_trait]
impl DataAvailabilityClient for EigenDAClient {
    async fn dispatch_blob(
        &self,
        _batch_number: u32,
        blob_data: Vec<u8>,
    ) -> Result<types::DispatchResponse, types::DAError> {
        tracing::info!("Dispatching blob to Eigen DA");
        let response = self
            .client
            .post(format!("{}/put/", self.config.api_node_url))
            .header(http::header::CONTENT_TYPE, "application/octet-stream")
            .body(blob_data)
            .send()
            .await
            .unwrap();

        let request_id = response.bytes().await.unwrap().to_vec();
        Ok(types::DispatchResponse {
            blob_id: hex::encode(request_id),
        })
    }
    async fn get_inclusion_data(
        &self,
        blob_id: &str,
    ) -> anyhow::Result<Option<types::InclusionData>, types::DAError> {
        let response = self
            .client
            .get(format!("{}/get/0x{}", self.config.api_node_url, blob_id))
            .send()
            .await
            .unwrap();
        let data = response.bytes().await.unwrap().to_vec();
        Ok(Some(InclusionData { data }))
    }

    fn clone_boxed(&self) -> Box<dyn DataAvailabilityClient> {
        Box::new(self.clone())
    }

    fn blob_size_limit(&self) -> Option<usize> {
        Some(Self::BLOB_SIZE_LIMIT_IN_BYTES)
    }
}

// Note: This methods should be uncommented if the `get_inclusion_data` method
// implementation gets uncommented.
//
// fn to_retriable_error(error: anyhow::Error) -> types::DAError {
//     types::DAError {
//         error,
//         is_retriable: true,
//     }
// }

// fn to_non_retriable_error(error: anyhow::Error) -> types::DAError {
//     types::DAError {
//         error,
//         is_retriable: false,
//     }
// }
