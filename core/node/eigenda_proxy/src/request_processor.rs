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
    #[should_panic]
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
        let mut blob_id = hex::encode([
            102, 99, 98, 102, 97, 51, 51, 98, 99, 55, 98, 52, 50, 100, 102, 98, 52, 102, 102, 54,
            51, 98, 52, 97, 50, 54, 51, 56, 48, 55, 50, 53, 57, 53, 97, 48, 100, 51, 102, 54, 97,
            102, 97, 48, 57, 51, 100, 54, 98, 57, 101, 50, 102, 50, 49, 54, 57, 50, 97, 55, 48, 98,
            48, 54, 45, 51, 49, 51, 55, 51, 50, 51, 52, 51, 55, 51, 48, 51, 51, 51, 55, 51, 48, 51,
            54, 51, 48, 51, 50, 51, 48, 51, 50, 51, 50, 51, 53, 51, 51, 51, 49, 51, 55, 50, 102,
            51, 49, 50, 102, 51, 51, 51, 51, 50, 102, 51, 48, 50, 102, 51, 51, 51, 51, 50, 102,
            101, 51, 98, 48, 99, 52, 52, 50, 57, 56, 102, 99, 49, 99, 49, 52, 57, 97, 102, 98, 102,
            52, 99, 56, 57, 57, 54, 102, 98, 57, 50, 52, 50, 55, 97, 101, 52, 49, 101, 52, 54, 52,
            57, 98, 57, 51, 52, 99, 97, 52, 57, 53, 57, 57, 49, 98, 55, 56, 53, 50, 98, 56, 53, 53,
        ]);
        let response = request_processor.get_blob_id(Path(blob_id.clone())).await;
        assert_eq!(response.status(), StatusCode::OK);

        // We change the blob id to a non-existent one
        blob_id.push_str("fa");
        request_processor.get_blob_id(Path(blob_id)).await;
    }
}
