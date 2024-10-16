use std::{
    collections::HashMap,
    sync::RwLock,
    time::{Duration, Instant},
};

use rand::{rngs::OsRng, Rng, RngCore};
use rlp::decode;
use sha3::{Digest, Keccak256};

use crate::{
    blob_info::{self, BlobInfo},
    errors::MemStoreError,
};

struct MemStoreConfig {
    max_blob_size_bytes: u64,
    blob_expiration: Duration,
    put_latency: Duration,
    get_latency: Duration,
}

struct MemStore {
    mutex: RwLock<()>,
    config: MemStoreConfig,
    key_starts: HashMap<String, Instant>,
    store: HashMap<String, Vec<u8>>,
}

impl MemStore {
    fn new(config: MemStoreConfig) -> Self {
        Self {
            mutex: RwLock::new(()),
            config,
            key_starts: HashMap::new(),
            store: HashMap::new(),
        }
    }

    async fn put(&mut self, value: Vec<u8>) -> Result<Vec<u8>, MemStoreError> {
        tokio::time::sleep(self.config.put_latency).await;
        if value.len() as u64 > self.config.max_blob_size_bytes {
            return Err(MemStoreError::BlobToLarge.into());
        }
        let _guard = self.mutex.write().unwrap();

        // todo: Encode blob?

        let mut entropy = [0u8; 10];
        OsRng.fill_bytes(&mut entropy);

        let mut hasher = Keccak256::new();
        hasher.update(&entropy);
        let mock_batch_root = hasher.finalize().to_vec();

        let block_num = OsRng.gen_range(0u32..1000);

        let blob_info = blob_info::BlobInfo {
            blob_header: blob_info::BlobHeader {
                commitment: blob_info::G1Commitment {
                    // todo: generate real commitment
                    x: vec![0u8; 32],
                    y: vec![0u8; 32],
                },
                data_length: value.len() as u32,
                blob_quorum_params: vec![blob_info::BlobQuorumParam {
                    quorum_number: 1,
                    adversary_threshold_percentage: 29,
                    confirmation_threshold_percentage: 30,
                    chunk_length: 300,
                }],
            },
            blob_verification_proof: blob_info::BlobVerificationProof {
                batch_medatada: blob_info::BatchMetadata {
                    batch_header: blob_info::BatchHeader {
                        batch_root: mock_batch_root.clone(),
                        quorum_numbers: vec![0x1, 0x0],
                        quorum_signed_percentages: vec![0x60, 0x90],
                        reference_block_number: block_num,
                    },
                    signatory_record_hash: mock_batch_root,
                    fee: vec![],
                    confirmation_block_number: block_num,
                    batch_header_hash: vec![],
                },
                batch_id: 69,
                blob_index: 420,
                inclusion_proof: entropy.to_vec(),
                quorum_indexes: vec![0x1, 0x0],
            },
        };

        let cert_bytes = rlp::encode(&blob_info).to_vec();

        let key = String::from_utf8(blob_info.blob_verification_proof.inclusion_proof.clone())
            .map_err(|_| MemStoreError::IncorrectString)?;

        if self.store.contains_key(key.as_str()) {
            return Err(MemStoreError::BlobAlreadyExists);
        }

        self.key_starts.insert(key.clone(), Instant::now());
        self.store.insert(key, value);
        Ok(cert_bytes)
    }

    async fn get(&self, commit: Vec<u8>) -> Result<Vec<u8>, MemStoreError> {
        tokio::time::sleep(self.config.get_latency).await;
        let _guard = self.mutex.read().unwrap();
        let blob_info: BlobInfo =
            decode(&commit).map_err(|_| MemStoreError::IncorrectCommitment)?;
        let key = String::from_utf8(blob_info.blob_verification_proof.inclusion_proof.clone())
            .map_err(|_| MemStoreError::IncorrectString)?;
        match self.store.get(&key) {
            Some(value) => Ok(value.clone()),
            None => Err(MemStoreError::BlobNotFound),
        }
        // TODO: verify commitment?
        // TODO: decode blob?
    }
}
