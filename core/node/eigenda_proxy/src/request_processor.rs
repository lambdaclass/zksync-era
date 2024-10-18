use axum::{extract::Path, Json};

use crate::errors::RequestProcessorError;

#[derive(Clone)]
pub(crate) struct RequestProcessor {}

fn read_version_and_commitment(blob_id: String) -> (String, u64) {
    if blob_id.len() < 4 {
        panic!("Blob ID too short");
    }

    let mut blob_id = blob_id;
    let prefix: String = blob_id.drain(..2).collect();
    if prefix != "0x" {
        panic!("Invalid prefix");
    }

    let version: u64 = blob_id.drain(..1).collect::<String>().parse().unwrap();
    match version {
        0 => (),
        1 => (),
        _ => panic!("Invalid version {:?}", version),
    }
    println!("version: {:?}", version);

    let commitment = blob_id;
    println!("commitment: {:?}", commitment);

    (commitment, version)
}

impl RequestProcessor {
    pub(crate) fn new() -> Self {
        Self {}
    }

    pub(crate) async fn get_blob_id(
        &self,
        Path(blob_id): Path<String>,
    ) -> Result<Json<String>, RequestProcessorError> {
        // Read commitment and mode version
        let (commitment, version) = read_version_and_commitment(blob_id); // TODO: Implement

        // Request commitment to dispatcher?

        Ok(Json(commitment))
    }

    #[tracing::instrument(skip_all)]
    pub(crate) async fn put_blob_id(
        &self,
        Path(blob_id): Path<String>,
    ) -> Result<Json<String>, RequestProcessorError> {
        Ok(Json(blob_id))
    }
}
