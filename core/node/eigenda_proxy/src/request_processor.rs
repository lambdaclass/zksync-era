use axum::{extract::Path, Error, Json};
use crate::errors::RequestProcessorError;

#[derive(Clone)]
pub(crate) struct RequestProcessor {}

impl RequestProcessor {
    pub(crate) fn new() -> Self {
        Self {}
    }

    pub(crate) async fn get_blob_id(&self, Path(blob_id): Path<String>) -> Result<Json<String>, RequestProcessorError>{
        Ok(Json(blob_id))
    }

    #[tracing::instrument(skip_all)]
    pub(crate) async fn put_blob_id(&self, request: Path<u32>) -> Result<u32, Error> {
        Ok(request.0)
    }

    #[tracing::instrument(skip_all)]
    pub(crate) async fn foo(&self, Path(blob_id): Path<String>) -> () {
        ()
    }

}
