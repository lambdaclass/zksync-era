use axum::{extract::Path, http::Response};

use crate::{eigenda_client::EigenDAClient, errors::RequestProcessorError};

#[derive(Clone)]
pub(crate) struct RequestProcessor {
    eigenda_client: EigenDAClient,
}

impl RequestProcessor {
    pub(crate) fn new(eigenda_client: EigenDAClient) -> Self {
        Self { eigenda_client }
    }

    pub(crate) async fn get_blob_id(
        &self,
        Path(blob_id): Path<String>,
    ) -> Result<axum::response::Response, RequestProcessorError> {
        let blob_id_bytes = hex::decode(blob_id).unwrap();
        let response = self
            .eigenda_client
            .get_blob(blob_id_bytes)
            .await
            .map_err(|e| RequestProcessorError::EigenDA(e))?;
        Ok(Response::new(response.into()))
    }

    pub(crate) async fn put_blob_id(
        &self,
        Path(data): Path<String>,
    ) -> Result<axum::response::Response, RequestProcessorError> {
        let data_bytes = hex::decode(data).unwrap();
        let response = self
            .eigenda_client
            .put_blob(data_bytes)
            .await
            .map_err(|e| RequestProcessorError::EigenDA(e))?;
        Ok(Response::new(response.into()))
    }
}
