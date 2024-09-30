use std::fmt::Debug;

use async_trait::async_trait;
use rlp::decode;
use zksync_config::configs::da_client::eigen_da::EigenDAConfig;
use zksync_da_client::{
    types::{self, DAError, InclusionData},
    DataAvailabilityClient,
};
use zksync_eth_client::{
    clients::{DynClient, L1}, CallFunctionArgs, ContractCallError, EthInterface
};
use zksync_types::{blob, Address, U256};

use crate::blob_info::BlobInfo;

#[derive(Clone, Debug)]
pub struct EigenDAClient {
    client: reqwest::Client,
    config: EigenDAConfig,
    eth_client: Box<DynClient<L1>>,
    verifier_address: Address,
}

impl EigenDAClient {
    pub const BLOB_SIZE_LIMIT_IN_BYTES: usize = 2 * 1024 * 1024; // 2MB

    pub async fn new(config: EigenDAConfig, eth_client: Box<DynClient<L1>>, verifier_address: Address) -> anyhow::Result<Self> {
        Ok(Self {
            client: reqwest::Client::new(),
            config,
            eth_client,
            verifier_address,
        })
    }
}
impl EigenDAClient {
    pub async fn verify_blob(
        &self,
        commitment: String,
    ) -> Result<U256, types::DAError> {
        let data = &hex::decode(commitment).unwrap()[3..];

        let blob_info: BlobInfo = match decode(&data) {
            Ok(blob_info) => blob_info,
            Err(e) => panic!("Error decoding commitment: {}", e)
        };

        CallFunctionArgs::new("verifyBlob", blob_info)
            .for_contract(
                self.verifier_address, 
                &zksync_contracts::eigenda_verifier_contract(),
            )
            .call(&self.eth_client)
            .await.map_err(|e| DAError {
                error: e.into(),
                is_retriable: true,
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
        let response = self
            .client
            .post(format!("{}/put/", self.config.api_node_url))
            .header(http::header::CONTENT_TYPE, "application/octetstream")
            .body(blob_data)
            .send()
            .await
            .map_err(to_retriable_error)?;

        let request_id = response
            .bytes()
            .await
            .map_err(to_non_retriable_da_error)?
            .to_vec();

        self.verify_blob(
            hex::encode(request_id.clone()),
        ).await?;

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
            .map_err(to_retriable_error)?;
        let data = response
            .bytes()
            .await
            .map_err(to_non_retriable_da_error)?
            .to_vec();
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
fn to_retriable_error(error: impl Into<anyhow::Error>) -> DAError {
    DAError {
        error: error.into(),
        is_retriable: true,
    }
}

fn to_non_retriable_da_error(error: impl Into<anyhow::Error>) -> DAError {
    DAError {
        error: error.into(),
        is_retriable: false,
    }
}
