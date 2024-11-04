use std::{str::FromStr, time::Duration};

use secp256k1::{ecdsa::RecoverableSignature, SecretKey};
use tokio::{sync::mpsc, time::Instant};
use tokio_stream::{wrappers::ReceiverStream, StreamExt};
use tonic::{
    transport::{Channel, ClientTlsConfig, Endpoint},
    Streaming,
};
use zksync_config::configs::da_client::eigen::DisperserConfig;
#[cfg(test)]
use zksync_da_client::types::DAError;

use super::disperser::BlobInfo;
use crate::eigen::{
    blob_info,
    disperser::{
        self,
        authenticated_request::Payload::{AuthenticationData, DisperseRequest},
        disperser_client::DisperserClient,
        AuthenticatedReply, BlobAuthHeader, DisperseBlobReply,
    },
};

#[derive(Debug, Clone)]
pub struct RawEigenClient {
    client: DisperserClient<Channel>,
    private_key: SecretKey,
    config: DisperserConfig,
}

pub(crate) const DATA_CHUNK_SIZE: usize = 32;

impl RawEigenClient {
    pub(crate) const BUFFER_SIZE: usize = 1000;

    pub async fn new(private_key: SecretKey, config: DisperserConfig) -> anyhow::Result<Self> {
        let endpoint = Endpoint::from_str(&config.disperser_rpc.as_str())?
            .tls_config(ClientTlsConfig::new())?;
        let client = DisperserClient::connect(endpoint)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to connect to Disperser server: {}", e))?;

        Ok(RawEigenClient {
            client,
            private_key,
            config,
        })
    }

    async fn dispatch_blob_non_authenticated(&self, data: Vec<u8>) -> anyhow::Result<String> {
        let padded_data = convert_by_padding_empty_byte(&data);
        let request = disperser::DisperseBlobRequest {
            data: padded_data,
            custom_quorum_numbers: vec![],
            account_id: String::default(), // Account Id is not used in non-authenticated mode
        };

        let mut client_clone = self.client.clone();
        let disperse_reply = client_clone.disperse_blob(request).await?.into_inner();

        let blob_info = self
            .await_for_inclusion(client_clone, disperse_reply)
            .await?;

        let verification_proof = blob_info
            .blob_verification_proof
            .clone()
            .ok_or_else(|| anyhow::anyhow!("No blob verification proof in response"))?;
        let blob_id = format!(
            "{}:{}",
            verification_proof.batch_id, verification_proof.blob_index
        );
        tracing::info!("Blob dispatch confirmed, blob id: {}", blob_id);

        let blob_id = blob_info::BlobInfo::try_from(blob_info)
            .map_err(|e| anyhow::anyhow!("Failed to convert blob info: {}", e))?;

        Ok(hex::encode(rlp::encode(&blob_id)))
    }

    async fn dispatch_blob_authenticated(&self, data: Vec<u8>) -> anyhow::Result<String> {
        let mut client_clone = self.client.clone();
        let (tx, rx) = mpsc::channel(Self::BUFFER_SIZE);

        let response_stream = client_clone.disperse_blob_authenticated(ReceiverStream::new(rx));
        let padded_data = convert_by_padding_empty_byte(&data);

        // 1. send DisperseBlobRequest
        self.disperse_data(padded_data, &tx).await?;

        // this await is blocked until the first response on the stream, so we only await after sending the `DisperseBlobRequest`
        let mut response_stream = response_stream.await?.into_inner();

        // 2. receive BlobAuthHeader
        let blob_auth_header = self.receive_blob_auth_header(&mut response_stream).await?;

        // 3. sign and send BlobAuthHeader
        self.submit_authentication_data(blob_auth_header.clone(), &tx)
            .await?;

        // 4. receive DisperseBlobReply
        let reply = response_stream
            .next()
            .await
            .ok_or_else(|| anyhow::anyhow!("No response from server"))?
            .unwrap()
            .payload
            .ok_or_else(|| anyhow::anyhow!("No payload in response"))?;

        let disperser::authenticated_reply::Payload::DisperseReply(disperse_reply) = reply else {
            return Err(anyhow::anyhow!("Unexpected response from server"));
        };

        // 5. poll for blob status until it reaches the Confirmed state
        let blob_info = self
            .await_for_inclusion(client_clone, disperse_reply)
            .await?;

        let verification_proof = blob_info
            .blob_verification_proof
            .clone()
            .ok_or_else(|| anyhow::anyhow!("No blob verification proof in response"))?;
        let blob_id = format!(
            "{}:{}",
            verification_proof.batch_id, verification_proof.blob_index
        );
        tracing::info!("Blob dispatch confirmed, blob id: {}", blob_id);

        let blob_id = blob_info::BlobInfo::try_from(blob_info)
            .map_err(|e| anyhow::anyhow!("Failed to convert blob info: {}", e))?;
        Ok(hex::encode(rlp::encode(&blob_id)))
    }

    pub async fn dispatch_blob(&self, data: Vec<u8>) -> anyhow::Result<String> {
        match self.config.authenticaded {
            true => self.dispatch_blob_authenticated(data).await,
            false => self.dispatch_blob_non_authenticated(data).await,
        }
    }

    async fn disperse_data(
        &self,
        data: Vec<u8>,
        tx: &mpsc::Sender<disperser::AuthenticatedRequest>,
    ) -> anyhow::Result<()> {
        let req = disperser::AuthenticatedRequest {
            payload: Some(DisperseRequest(disperser::DisperseBlobRequest {
                data,
                custom_quorum_numbers: vec![],
                account_id: get_account_id(&self.private_key),
            })),
        };

        tx.send(req)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send DisperseBlobRequest: {}", e))
    }

    async fn submit_authentication_data(
        &self,
        blob_auth_header: BlobAuthHeader,
        tx: &mpsc::Sender<disperser::AuthenticatedRequest>,
    ) -> anyhow::Result<()> {
        // TODO: replace challenge_parameter with actual auth header when it is available
        let digest = zksync_basic_types::web3::keccak256(
            &blob_auth_header.challenge_parameter.to_be_bytes(),
        );
        let signature: RecoverableSignature = secp256k1::Secp256k1::signing_only()
            .sign_ecdsa_recoverable(
                &secp256k1::Message::from_slice(&digest[..])?,
                &self.private_key,
            );
        let (recovery_id, sig) = signature.serialize_compact();

        let mut signature = Vec::with_capacity(65);
        signature.extend_from_slice(&sig);
        signature.push(recovery_id.to_i32() as u8);

        let req = disperser::AuthenticatedRequest {
            payload: Some(AuthenticationData(disperser::AuthenticationData {
                authentication_data: signature,
            })),
        };

        tx.send(req)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send AuthenticationData: {}", e))
    }

    async fn receive_blob_auth_header(
        &self,
        response_stream: &mut Streaming<AuthenticatedReply>,
    ) -> anyhow::Result<disperser::BlobAuthHeader> {
        let reply = response_stream
            .next()
            .await
            .ok_or_else(|| anyhow::anyhow!("No response from server"))?;

        let Ok(reply) = reply else {
            return Err(anyhow::anyhow!("Err from server: {:?}", reply));
        };

        let reply = reply
            .payload
            .ok_or_else(|| anyhow::anyhow!("No payload in response"))?;

        if let disperser::authenticated_reply::Payload::BlobAuthHeader(blob_auth_header) = reply {
            Ok(blob_auth_header)
        } else {
            Err(anyhow::anyhow!("Unexpected response from server"))
        }
    }

    async fn await_for_inclusion(
        &self,
        mut client: DisperserClient<Channel>,
        disperse_blob_reply: DisperseBlobReply,
    ) -> anyhow::Result<BlobInfo> {
        let polling_request = disperser::BlobStatusRequest {
            request_id: disperse_blob_reply.request_id,
        };

        let start_time = Instant::now();
        while Instant::now() - start_time < Duration::from_millis(self.config.status_query_timeout)
        {
            tokio::time::sleep(Duration::from_millis(self.config.status_query_interval)).await;
            let resp = client
                .get_blob_status(polling_request.clone())
                .await?
                .into_inner();

            match disperser::BlobStatus::try_from(resp.status)? {
                disperser::BlobStatus::Processing | disperser::BlobStatus::Dispersing => {}
                disperser::BlobStatus::Failed => {
                    return Err(anyhow::anyhow!("Blob dispatch failed"))
                }
                disperser::BlobStatus::InsufficientSignatures => {
                    return Err(anyhow::anyhow!("Insufficient signatures"))
                }
                disperser::BlobStatus::Confirmed => {
                    if !self.config.wait_for_finalization {
                        let blob_info = resp
                            .info
                            .ok_or_else(|| anyhow::anyhow!("No blob header in response"))?;
                        return Ok(blob_info);
                    }
                }
                disperser::BlobStatus::Finalized => {
                    let blob_info = resp
                        .info
                        .ok_or_else(|| anyhow::anyhow!("No blob header in response"))?;
                    return Ok(blob_info);
                }

                _ => return Err(anyhow::anyhow!("Received unknown blob status")),
            }
        }

        Err(anyhow::anyhow!("Failed to disperse blob (timeout)"))
    }

    #[cfg(test)]
    pub async fn get_blob_data(&self, blob_id: &str) -> anyhow::Result<Option<Vec<u8>>, DAError> {
        use anyhow::anyhow;
        use zksync_da_client::types::DAError;

        use crate::eigen::blob_info::BlobInfo;

        let commit = hex::decode(blob_id).map_err(|_| DAError {
            error: anyhow!("Failed to decode blob_id"),
            is_retriable: false,
        })?;
        let blob_info: BlobInfo = rlp::decode(&commit).map_err(|_| DAError {
            error: anyhow!("Failed to decode blob_info"),
            is_retriable: false,
        })?;
        let blob_index = blob_info.blob_verification_proof.blob_index;
        let batch_header_hash = blob_info
            .blob_verification_proof
            .batch_medatada
            .batch_header_hash;
        let get_response = self
            .client
            .clone()
            .retrieve_blob(disperser::RetrieveBlobRequest {
                batch_header_hash,
                blob_index,
            })
            .await
            .map_err(|e| DAError {
                error: anyhow!(e),
                is_retriable: true,
            })?
            .into_inner();

        if get_response.data.len() == 0 {
            return Err(DAError {
                error: anyhow!("Failed to get blob data"),
                is_retriable: false,
            });
        }
        //TODO: remove zkgpad_rs
        let data = kzgpad_rs::remove_empty_byte_from_padded_bytes(&get_response.data);
        return Ok(Some(data));
    }
}

fn get_account_id(secret_key: &SecretKey) -> String {
    let public_key =
        secp256k1::PublicKey::from_secret_key(&secp256k1::Secp256k1::new(), secret_key);
    let hex = hex::encode(public_key.serialize_uncompressed());

    format!("0x{}", hex)
}

fn convert_by_padding_empty_byte(data: &[u8]) -> Vec<u8> {
    let parse_size = DATA_CHUNK_SIZE - 1;

    // Calculate the number of chunks
    let data_len = (data.len() + parse_size - 1) / parse_size;

    // Pre-allocate `valid_data` with enough space for all chunks
    let mut valid_data = vec![0u8; data_len * DATA_CHUNK_SIZE];
    let mut valid_end = data_len * DATA_CHUNK_SIZE;

    for (i, chunk) in data.chunks(parse_size).enumerate() {
        let offset = i * DATA_CHUNK_SIZE;
        valid_data[offset] = 0x00; // Set first byte of each chunk to 0x00 for big-endian compliance

        let copy_end = offset + 1 + chunk.len();
        valid_data[offset + 1..copy_end].copy_from_slice(chunk);

        if i == data_len - 1 && chunk.len() < parse_size {
            valid_end = offset + 1 + chunk.len();
        }
    }

    valid_data.truncate(valid_end);
    valid_data
}
