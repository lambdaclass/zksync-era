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
        _blob_id: &str,
    ) -> anyhow::Result<Option<types::InclusionData>, types::DAError> {
        // let request_id = hex::decode(blob_id).unwrap();
        // let blob_status_reply = self
        //     .disperser
        //     .lock()
        //     .await
        //     .get_blob_status(BlobStatusRequest { request_id })
        //     .await
        //     .unwrap()
        //     .into_inner();
        // let blob_status = blob_status_reply.status();
        // match blob_status {
        //     BlobStatus::Unknown => Err(to_retriable_error(anyhow::anyhow!(
        //         "Blob status is unknown"
        //     ))),
        //     BlobStatus::Processing => Err(to_retriable_error(anyhow::anyhow!(
        //         "Blob is being processed"
        //     ))),
        //     BlobStatus::Confirmed => Err(to_retriable_error(anyhow::anyhow!(
        //         "Blob is confirmed but not finalized"
        //     ))),
        //     BlobStatus::Failed => Err(to_non_retriable_error(anyhow::anyhow!("Blob has failed"))),
        //     BlobStatus::InsufficientSignatures => Err(to_non_retriable_error(anyhow::anyhow!(
        //         "Insufficient signatures for blob"
        //     ))),
        //     BlobStatus::Dispersing => Err(to_retriable_error(anyhow::anyhow!(
        //         "Blob is being dispersed"
        //     ))),
        //     BlobStatus::Finalized => Ok(Some(types::InclusionData {
        //         data: blob_status_reply
        //             .info
        //             .unwrap()
        //             .blob_verification_proof
        //             .unwrap()
        //             .inclusion_proof,
        //     })),
        // }
        Ok(Some(InclusionData { data: vec![] }))
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
