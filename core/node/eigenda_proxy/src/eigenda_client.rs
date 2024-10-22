use std::{
    str::FromStr,
    sync::Arc,
    time::{Duration, Instant},
};

use rlp::decode;
use secp256k1::{PublicKey, Secp256k1, SecretKey};
use tokio::{sync::Mutex, time::interval};
use tonic::transport::{Channel, ClientTlsConfig};
use zksync_config::configs::da_client::eigen_da::DisperserConfig;

use crate::{
    blob_info::BlobInfo,
    disperser::{
        self, authenticated_reply::Payload, disperser_client::DisperserClient, AuthenticatedReply,
        AuthenticatedRequest, AuthenticationData, BlobAuthHeader, BlobStatusRequest,
        DisperseBlobRequest,
    },
    errors::EigenDAError,
    signer::sign,
};

pub struct EigenDAClient {
    disperser: Arc<Mutex<DisperserClient<Channel>>>,
    config: DisperserConfig,
}

impl EigenDAClient {
    pub async fn new(config: DisperserConfig) -> Result<Self, EigenDAError> {
        match rustls::crypto::ring::default_provider().install_default() {
            Ok(_) => {}
            Err(_) => {} // This is not an actual error, we expect this function to return an Err(Arc<CryptoProvider>)
        };
        let inner = Channel::builder(
            config
                .disperser_rpc
                .parse()
                .map_err(|_| EigenDAError::UriError)?,
        )
        .tls_config(ClientTlsConfig::new().with_native_roots())
        .map_err(|_| EigenDAError::TlsError)?;

        let disperser = Arc::new(Mutex::new(
            DisperserClient::connect(inner)
                .await
                .map_err(|_| EigenDAError::ConnectionError)?,
        ));

        Ok(Self { disperser, config })
    }

    fn result_to_status(&self, result: i32) -> disperser::BlobStatus {
        match result {
            0 => disperser::BlobStatus::Unknown,
            1 => disperser::BlobStatus::Processing,
            2 => disperser::BlobStatus::Confirmed,
            3 => disperser::BlobStatus::Failed,
            4 => disperser::BlobStatus::Finalized,
            5 => disperser::BlobStatus::InsufficientSignatures,
            6 => disperser::BlobStatus::Dispersing,
            _ => disperser::BlobStatus::Unknown,
        }
    }

    pub async fn put_blob(&self, blob_data: Vec<u8>) -> Result<Vec<u8>, EigenDAError> {
        tracing::info!("Putting blob");
        if blob_data.len() > self.config.blob_size_limit as usize {
            return Err(EigenDAError::PutError("Blob too large".to_string()));
        }
        let reply = self
            .disperser
            .lock()
            .await
            .disperse_blob(DisperseBlobRequest {
                data: blob_data,
                custom_quorum_numbers: self
                    .config
                    .custom_quorum_numbers
                    .clone()
                    .unwrap_or_default(),
                account_id: self.config.account_id.clone().unwrap_or_default(),
            })
            .await
            .map_err(|e| EigenDAError::PutError(e.to_string()))?
            .into_inner();

        if self.result_to_status(reply.result) == disperser::BlobStatus::Failed {
            return Err(EigenDAError::PutError(
                "Failed to disperse blob".to_string(),
            ));
        }

        let request_id_str = String::from_utf8(reply.request_id.clone())
            .map_err(|e| EigenDAError::PutError(e.to_string()))?;

        let mut interval = interval(Duration::from_secs(self.config.status_query_interval));
        let start_time = Instant::now();
        while Instant::now() - start_time < Duration::from_secs(self.config.status_query_timeout) {
            let blob_status_reply = self
                .disperser
                .lock()
                .await
                .get_blob_status(BlobStatusRequest {
                    request_id: reply.request_id.clone(),
                })
                .await
                .map_err(|e| EigenDAError::PutError(e.to_string()))?
                .into_inner();

            let blob_status = blob_status_reply.status();

            tracing::info!(
                "Dispersing blob {:?}, status: {:?}",
                request_id_str,
                blob_status
            );

            match blob_status {
                disperser::BlobStatus::Unknown => {
                    interval.tick().await;
                }
                disperser::BlobStatus::Processing => {
                    interval.tick().await;
                }
                disperser::BlobStatus::Confirmed => {
                    if self.config.wait_for_finalization {
                        interval.tick().await;
                    } else {
                        match blob_status_reply.info {
                            Some(info) => {
                                let blob_info = BlobInfo::try_from(info)
                                    .map_err(|e| EigenDAError::PutError(e.to_string()))?;
                                return Ok(rlp::encode(&blob_info).to_vec());
                            }
                            None => {
                                return Err(EigenDAError::PutError(
                                    "Failed to get blob info".to_string(),
                                ));
                            }
                        }
                    }
                }
                disperser::BlobStatus::Failed => {
                    return Err(EigenDAError::PutError(
                        "Failed to disperse blob".to_string(),
                    ));
                }
                disperser::BlobStatus::InsufficientSignatures => {
                    return Err(EigenDAError::PutError(
                        "Insufficient signatures".to_string(),
                    ));
                }
                disperser::BlobStatus::Dispersing => {
                    interval.tick().await;
                }
                disperser::BlobStatus::Finalized => match blob_status_reply.info {
                    Some(info) => {
                        let blob_info = BlobInfo::try_from(info)
                            .map_err(|e| EigenDAError::PutError(e.to_string()))?;
                        return Ok(rlp::encode(&blob_info).to_vec());
                    }
                    None => {
                        return Err(EigenDAError::PutError(
                            "Failed to get blob info".to_string(),
                        ));
                    }
                },
            }
        }

        return Err(EigenDAError::PutError(
            "Failed to disperse blob".to_string(),
        ));
    }

    async fn authentication(
        &self,
        blob_data: Vec<u8>,
        custom_quorum_numbers: Vec<u32>,
        account_id: String,
        private_key: &SecretKey,
    ) -> Result<disperser::DisperseBlobReply, EigenDAError> {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel::<AuthenticatedRequest>();
        let request = AuthenticatedRequest {
            payload: Some(disperser::authenticated_request::Payload::DisperseRequest(
                DisperseBlobRequest {
                    data: blob_data,
                    custom_quorum_numbers,
                    account_id,
                },
            )),
        };
        sender
            .send(request)
            .map_err(|e| EigenDAError::PutError(e.to_string()))?;
        let receiver_stream = tokio_stream::wrappers::UnboundedReceiverStream::new(receiver);
        let mut stream = self
            .disperser
            .lock()
            .await
            .disperse_blob_authenticated(receiver_stream)
            .await
            .map_err(|e| EigenDAError::PutError(e.to_string()))?;

        let result = stream
            .get_mut()
            .message()
            .await
            .map_err(|e| EigenDAError::PutError(e.to_string()))?;

        let reply = if let Some(AuthenticatedReply {
            payload: Some(Payload::BlobAuthHeader(header)),
        }) = result
        {
            let challenge = header.challenge_parameter;
            let new_request = AuthenticatedRequest {
                payload: Some(
                    disperser::authenticated_request::Payload::AuthenticationData(
                        AuthenticationData {
                            authentication_data: sign(challenge, private_key),
                        },
                    ),
                ),
            };
            sender
                .send(new_request)
                .map_err(|e| EigenDAError::PutError(e.to_string()))?;
            let result = stream
                .get_mut()
                .message()
                .await
                .map_err(|e| EigenDAError::PutError(e.to_string()))?;

            let reply = if let Some(AuthenticatedReply {
                payload: Some(Payload::DisperseReply(reply)),
            }) = result
            {
                reply
            } else {
                return Err(EigenDAError::PutError("Failed to authenticate".to_string()));
            };
            reply
        } else {
            return Err(EigenDAError::PutError("Failed to authenticate".to_string()));
        };

        Ok(reply)
    }

    pub async fn put_blob_authenticated(
        &self,
        blob_data: Vec<u8>,
    ) -> Result<Vec<u8>, EigenDAError> {
        tracing::info!("Putting blob");
        if blob_data.len() > self.config.blob_size_limit as usize {
            return Err(EigenDAError::PutError("Blob too large".to_string()));
        }

        let custom_quorum_numbers = self
            .config
            .custom_quorum_numbers
            .clone()
            .unwrap_or_default();
        let account_id = self.config.account_id.clone().unwrap_or_default();
        let secp = Secp256k1::new();
        let secret_key = SecretKey::from_str(account_id.as_str())
            .map_err(|e| EigenDAError::PutError(e.to_string()))?;
        let public_key = PublicKey::from_secret_key(&secp, &secret_key);
        let account_id = "0x".to_string() + &hex::encode(public_key.serialize_uncompressed());

        let reply = self
            .authentication(blob_data, custom_quorum_numbers, account_id, &secret_key)
            .await?;

        if self.result_to_status(reply.result) == disperser::BlobStatus::Failed {
            return Err(EigenDAError::PutError(
                "Failed to disperse blob".to_string(),
            ));
        }

        let request_id_str = String::from_utf8(reply.request_id.clone())
            .map_err(|e| EigenDAError::PutError(e.to_string()))?;

        let mut interval = interval(Duration::from_secs(self.config.status_query_interval));
        let start_time = Instant::now();
        while Instant::now() - start_time < Duration::from_secs(self.config.status_query_timeout) {
            let blob_status_reply = self
                .disperser
                .lock()
                .await
                .get_blob_status(BlobStatusRequest {
                    request_id: reply.request_id.clone(),
                })
                .await
                .map_err(|e| EigenDAError::PutError(e.to_string()))?
                .into_inner();

            let blob_status = blob_status_reply.status();

            tracing::info!(
                "Dispersing blob {:?}, status: {:?}",
                request_id_str,
                blob_status
            );

            match blob_status {
                disperser::BlobStatus::Unknown => {
                    interval.tick().await;
                }
                disperser::BlobStatus::Processing => {
                    interval.tick().await;
                }
                disperser::BlobStatus::Confirmed => {
                    if self.config.wait_for_finalization {
                        interval.tick().await;
                    } else {
                        match blob_status_reply.info {
                            Some(info) => {
                                let blob_info = BlobInfo::try_from(info)
                                    .map_err(|e| EigenDAError::PutError(e.to_string()))?;
                                return Ok(rlp::encode(&blob_info).to_vec());
                            }
                            None => {
                                return Err(EigenDAError::PutError(
                                    "Failed to get blob info".to_string(),
                                ));
                            }
                        }
                    }
                }
                disperser::BlobStatus::Failed => {
                    return Err(EigenDAError::PutError(
                        "Failed to disperse blob".to_string(),
                    ));
                }
                disperser::BlobStatus::InsufficientSignatures => {
                    return Err(EigenDAError::PutError(
                        "Insufficient signatures".to_string(),
                    ));
                }
                disperser::BlobStatus::Dispersing => {
                    interval.tick().await;
                }
                disperser::BlobStatus::Finalized => match blob_status_reply.info {
                    Some(info) => {
                        let blob_info = BlobInfo::try_from(info)
                            .map_err(|e| EigenDAError::PutError(e.to_string()))?;
                        return Ok(rlp::encode(&blob_info).to_vec());
                    }
                    None => {
                        return Err(EigenDAError::PutError(
                            "Failed to get blob info".to_string(),
                        ));
                    }
                },
            }
        }

        return Err(EigenDAError::PutError(
            "Failed to disperse blob".to_string(),
        ));
    }

    pub async fn get_blob(&self, commit: Vec<u8>) -> Result<Vec<u8>, EigenDAError> {
        tracing::info!("Getting blob");
        let blob_info: BlobInfo = decode(&commit).map_err(|_| EigenDAError::GetError)?;
        let blob_index = blob_info.blob_verification_proof.blob_index;
        let batch_header_hash = blob_info
            .blob_verification_proof
            .batch_medatada
            .batch_header_hash;
        let get_response = self
            .disperser
            .lock()
            .await
            .retrieve_blob(disperser::RetrieveBlobRequest {
                batch_header_hash,
                blob_index,
            })
            .await
            .map_err(|_| EigenDAError::GetError)?
            .into_inner();

        if get_response.data.len() == 0 {
            return Err(EigenDAError::GetError);
        }

        return Ok(get_response.data);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_eigenda_client() {
        let config = DisperserConfig {
            api_node_url: "".to_string(),
            custom_quorum_numbers: Some(vec![]),
            account_id: Some("".to_string()),
            disperser_rpc: "https://disperser-holesky.eigenda.xyz:443".to_string(),
            eth_confirmation_depth: -1,
            eigenda_eth_rpc: "".to_string(),
            eigenda_svc_manager_addr: "".to_string(),
            blob_size_limit: 2 * 1024 * 1024, // 2MB
            status_query_timeout: 1800,       // 30 minutes
            status_query_interval: 5,         // 5 seconds
            wait_for_finalization: false,
        };
        let store = match EigenDAClient::new(config).await {
            Ok(store) => store,
            Err(e) => panic!("Failed to create EigenDAProxyClient {:?}", e),
        };

        let blob = vec![0u8; 100];
        let cert = store.put_blob(blob.clone()).await.unwrap();
        let blob2 = store.get_blob(cert).await.unwrap();
        assert_eq!(blob, blob2);
    }

    #[tokio::test]
    async fn test_eigenda_multiple() {
        let config = DisperserConfig {
            api_node_url: "".to_string(),
            custom_quorum_numbers: Some(vec![]),
            account_id: Some("".to_string()),
            disperser_rpc: "https://disperser-holesky.eigenda.xyz:443".to_string(),
            eth_confirmation_depth: -1,
            eigenda_eth_rpc: "".to_string(),
            eigenda_svc_manager_addr: "".to_string(),
            blob_size_limit: 2 * 1024 * 1024, // 2MB
            status_query_timeout: 1800,       // 30 minutes
            status_query_interval: 5,         // 5 seconds
            wait_for_finalization: false,
        };
        let store = match EigenDAClient::new(config).await {
            Ok(store) => store,
            Err(e) => panic!("Failed to create EigenDAProxyClient {:?}", e),
        };

        let blob = vec![0u8; 100];
        let blob2 = vec![1u8; 100];
        let cert = store.put_blob(blob.clone());
        let cert2 = store.put_blob(blob2.clone());
        let (val1, val2) = tokio::join!(cert, cert2);
        let blob_result = store.get_blob(val1.unwrap()).await.unwrap();
        let blob_result2 = store.get_blob(val2.unwrap()).await.unwrap();
        assert_eq!(blob, blob_result);
        assert_eq!(blob2, blob_result2);
    }

    #[tokio::test]
    async fn test_eigenda_blob_size_limit() {
        let config = DisperserConfig {
            api_node_url: "".to_string(),
            custom_quorum_numbers: Some(vec![]),
            account_id: Some("".to_string()),
            disperser_rpc: "https://disperser-holesky.eigenda.xyz:443".to_string(),
            eth_confirmation_depth: -1,
            eigenda_eth_rpc: "".to_string(),
            eigenda_svc_manager_addr: "".to_string(),
            blob_size_limit: 2,         // 2MB
            status_query_timeout: 1800, // 30 minutes
            status_query_interval: 5,   // 5 seconds
            wait_for_finalization: false,
        };
        let store = match EigenDAClient::new(config).await {
            Ok(store) => store,
            Err(e) => panic!("Failed to create EigenDAProxyClient {:?}", e),
        };

        let blob = vec![0u8; 3];
        let cert = store.put_blob(blob.clone()).await;
        assert!(cert.is_err());
    }

    #[tokio::test]
    async fn test_eigenda_client_authenticated() {
        let config = DisperserConfig {
            api_node_url: "".to_string(),
            custom_quorum_numbers: Some(vec![]),
            account_id: Some(
                "850683b40d4a740aa6e745f889a6fdc8327be76e122f5aba645a5b02d0248db8".to_string(),
            ),
            disperser_rpc: "https://disperser-holesky.eigenda.xyz:443".to_string(),
            eth_confirmation_depth: -1,
            eigenda_eth_rpc: "".to_string(),
            eigenda_svc_manager_addr: "".to_string(),
            blob_size_limit: 2 * 1024 * 1024, // 2MB
            status_query_timeout: 1800,       // 30 minutes
            status_query_interval: 5,         // 5 seconds
            wait_for_finalization: false,
        };
        let store = match EigenDAClient::new(config).await {
            Ok(store) => store,
            Err(e) => panic!("Failed to create EigenDAProxyClient {:?}", e),
        };

        let blob = vec![0u8; 100];
        let cert = store.put_blob_authenticated(blob.clone()).await.unwrap();
        let blob2 = store.get_blob(cert).await.unwrap();
        assert_eq!(blob, blob2);
    }
}
