use std::sync::Arc;

use axum::{
    extract::Path,
    http::{header, StatusCode},
    response::Response,
    Json,
};
use tokio::sync::Mutex;
use tonic::transport::Channel;

use crate::{
    disperser::{disperser_client::DisperserClient, BlobStatus, BlobStatusRequest},
    errors::RequestProcessorError,
};

#[derive(Clone)]
pub(crate) struct RequestProcessor {
    disperser: Arc<Mutex<DisperserClient<Channel>>>,
}

impl RequestProcessor {
    pub(crate) fn new(disperser: DisperserClient<Channel>) -> Self {
        Self {
            disperser: Arc::new(Mutex::new(disperser)),
        }
    }

    pub(crate) async fn get_blob_id(
        &self,
        Path(blob_id): Path<String>,
    ) -> axum::response::Response {
        let request_id = match hex::decode(blob_id) {
            Ok(request_id) => request_id,
            Err(_) => {
                return Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "application/octet-stream")
                    .body("invalid commitment mode: invalid hex string".into())
                    .unwrap()
            }
        };

        loop {
            let request_id = request_id.clone();
            let blob_status_reply = self
                .disperser
                .lock()
                .await
                .get_blob_status(BlobStatusRequest { request_id })
                .await
                .unwrap()
                .into_inner();
            let blob_status = blob_status_reply.status();
            // TODO: May be suitable to sleep between calls to avoid spamming the server
            match blob_status {
                BlobStatus::Unknown => panic!("Blob status is unknown"),
                BlobStatus::Processing => tracing::info!("Blob is processing"),
                BlobStatus::Confirmed => tracing::info!("Blob is confirmed, but not finalized"),
                BlobStatus::Failed => panic!("Blob has failed"),
                BlobStatus::InsufficientSignatures => panic!("Insufficient signatures for blob"),
                BlobStatus::Dispersing => tracing::info!("Blob is being dispersed"),
                BlobStatus::Finalized => {
                    return Response::builder()
                        .status(StatusCode::OK)
                        .header(header::CONTENT_TYPE, "application/octet-stream")
                        .body("foo".into())
                        .unwrap()
                }
            }
        }
    }

    #[tracing::instrument(skip_all)]
    pub(crate) async fn put_blob_id(
        &self,
        Path(blob_id): Path<String>,
    ) -> Result<Json<String>, RequestProcessorError> {
        Ok(Json(blob_id))
    }
}

#[cfg(test)]
mod tests {
    use axum::extract::Path;
    use tonic::transport::ClientTlsConfig;

    use super::*;

    #[tokio::test]
    async fn test_get_blob_id() {
        let endpoint = Channel::builder(
            "https://disperser-holesky.eigenda.xyz"
                .to_string()
                .parse()
                .unwrap(),
        )
        .tls_config(ClientTlsConfig::new().with_native_roots())
        .unwrap();
        let disperser = DisperserClient::connect(endpoint).await.unwrap();
        let request_processor = RequestProcessor::new(disperser);

        // We know for certain that this blob id exists in the holesky disperser
        let blob_id = "010000f901b3f850f842a025d5eb2ffb96e675f2980f741f32504bcb43c4e34659b2af3c80e606f9d4140fa000e362c18d14d8b58035fb292a1a56209a6cf631efa007aa029fff9fadf5e66d02cac480213701c401213701f9015e82cfd535f873eba03123acdd9d10fa60c8f32fd245e7da208ea3454f672452e978e1ac8a8d49e229820001826363832711e5a01e3362433b8731f03f723739052b34d88a988ab0d7946f05e216c818cda67cd80083271237a0403e9c917631ced33c2678f7a1f1ddbf0370d3cae81132e1f3a64f99048cb550b8e0bfb90db1357815edee0469887689e1ca86945a1f09d19c8c7bb771700a55c29c8852aa813b69b6d0f12b5cb3e672e2e0b9cfb076a446e392f447443d4070e73df150ba1d395de9fd8fc23e0c27a022ae23f7cb32b8801ff4eddaf913aa1f71d93bbf8a75e96e02668a63d9bdc14ccc9a587421eb9ddcbe9686406a1449014a3cbb47f93506db1c40fe3bf84ed9f551969ab894c49f6a30066a0bd19ae921d293d72de36ef624f8322be1d0296f00ab4f794c67d8a194f43c4defc40e7627e48940777960bf82437a7e57f007e19850f958231c6dd20f6e35c80a49de22e0587e820001".to_string();
        let response = request_processor.get_blob_id(Path(blob_id.clone())).await;
        assert_eq!(response.status(), StatusCode::OK);
    }
}
