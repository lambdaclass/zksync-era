use rlp::Decodable;
use rlp::DecoderError;
use rlp::Rlp;
use serde::{Serialize, Deserialize};

#[derive(Debug,Serialize, Deserialize)]
struct G1Commitment {
    pub x: Vec<u8>,
    pub y: Vec<u8>,
}

impl Decodable for G1Commitment {
    fn decode(rlp: &Rlp) -> Result<Self, DecoderError> {
        let x: Vec<u8> = rlp.val_at(0)?;  // Decode first element as Vec<u8>
        let y: Vec<u8> = rlp.val_at(1)?;  // Decode second element as Vec<u8>

        Ok(G1Commitment { x, y })
    }
}

#[derive(Debug,Serialize, Deserialize)]
struct BlobQuorumParam {
    pub quorum_number: u32,
    pub adversary_threshold_percentage: u32,
    pub confirmation_threshold_percentage: u32,
    pub chunk_length: u32
}

impl Decodable for BlobQuorumParam {
    fn decode(rlp: &Rlp) -> Result<Self, DecoderError> {
        Ok(BlobQuorumParam {
            quorum_number: rlp.val_at(0)?,
            adversary_threshold_percentage: rlp.val_at(1)?,
            confirmation_threshold_percentage: rlp.val_at(2)?,
            chunk_length: rlp.val_at(3)?,
        })
    }
}

#[derive(Debug,Serialize, Deserialize)]
struct BlobHeader {
    pub commitment: G1Commitment,
    pub data_length: u32,
    pub blob_quorum_params: Vec<BlobQuorumParam>
}

impl Decodable for BlobHeader {
    fn decode(rlp: &Rlp) -> Result<Self, DecoderError> {
        let commitment: G1Commitment = rlp.val_at(0)?;
        let data_length: u32 = rlp.val_at(1)?;
        let blob_quorum_params: Vec<BlobQuorumParam> = rlp.list_at(2)?;

        Ok(BlobHeader {
            commitment,
            data_length,
            blob_quorum_params,
        })
    }
}

#[derive(Debug,Serialize, Deserialize)]
struct BatchHeader {
    pub batch_root: Vec<u8>,
    pub quorum_numbers: Vec<u8>,
    pub quorum_signed_percentages: Vec<u8>,
    pub reference_block_number: u32
}

impl Decodable for BatchHeader {
    fn decode(rlp: &Rlp) -> Result<Self, DecoderError> {
        Ok(BatchHeader {
            batch_root: rlp.val_at(0)?,
            quorum_numbers: rlp.val_at(1)?,
            quorum_signed_percentages: rlp.val_at(2)?,
            reference_block_number: rlp.val_at(3)?,
        })
    }
}

#[derive(Debug,Serialize, Deserialize)]
struct BatchMetadata {
    pub batch_header: BatchHeader,
    pub signatory_record_hash: Vec<u8>,
    pub fee: Vec<u8>,
    pub confirmation_block_number: u32,
    pub batch_header_hash: Vec<u8>
}

impl Decodable for BatchMetadata {
    fn decode(rlp: &Rlp) -> Result<Self, DecoderError> {
        let batch_header: BatchHeader = rlp.val_at(0)?;

        Ok(BatchMetadata {
            batch_header,
            signatory_record_hash: rlp.val_at(1)?,
            fee: rlp.val_at(2)?,
            confirmation_block_number: rlp.val_at(3)?,
            batch_header_hash: rlp.val_at(4)?,
        })
    }
}

#[derive(Debug,Serialize, Deserialize)]
struct BlobVerificationProof {
    pub batch_id: u32,
    pub blob_index: u32,
    pub batch_medatada: BatchMetadata,
    pub inclusion_proof: Vec<u8>,
    pub quorum_indexes: Vec<u8>
}

impl Decodable for BlobVerificationProof {
    fn decode(rlp: &Rlp) -> Result<Self, DecoderError> {
        Ok(BlobVerificationProof {
            batch_id: rlp.val_at(0)?,
            blob_index: rlp.val_at(1)?,
            batch_medatada: rlp.val_at(2)?,
            inclusion_proof: rlp.val_at(3)?,
            quorum_indexes: rlp.val_at(4)?,
        })
    }
}

#[derive(Debug,Serialize, Deserialize)]
pub struct BlobInfo {
    pub blob_header: BlobHeader,
    pub blob_verification_proof: BlobVerificationProof
}

impl Decodable for BlobInfo {
    fn decode(rlp: &Rlp) -> Result<Self, DecoderError> {
        let blob_header: BlobHeader = rlp.val_at(0)?;
        let blob_verification_proof: BlobVerificationProof = rlp.val_at(1)?;

        Ok(BlobInfo {
            blob_header,
            blob_verification_proof,
        })
    }
}
