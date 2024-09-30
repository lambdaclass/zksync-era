use rlp::Decodable;
use rlp::DecoderError;
use rlp::Rlp;
use zksync_types::web3::contract::Tokenizable;
use zksync_types::web3::contract::Tokenize;
use zksync_types::ethabi::Token;
use zksync_types::U256;

#[derive(Debug)]
pub struct G1Commitment {
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

impl Tokenize for G1Commitment {
    fn into_tokens(self) -> Vec<Token> {

        let x = Token::Uint(U256::from_big_endian(&self.x));
        let y = Token::Uint(U256::from_big_endian(&self.y));

        vec![x, y]
    }
}

#[derive(Debug)]
pub struct BlobQuorumParam {
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

impl Tokenize for BlobQuorumParam {
    fn into_tokens(self) -> Vec<Token> {

        let quorum_number = Token::Uint(U256::from(self.quorum_number));
        let adversary_threshold_percentage = Token::Uint(U256::from(self.adversary_threshold_percentage));
        let confirmation_threshold_percentage = Token::Uint(U256::from(self.confirmation_threshold_percentage));
        let chunk_length = Token::Uint(U256::from(self.chunk_length));

        vec![quorum_number, adversary_threshold_percentage,confirmation_threshold_percentage,chunk_length]
    }
}

#[derive(Debug)]
pub struct BlobHeader {
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

impl Tokenize for BlobHeader {
    fn into_tokens(self) -> Vec<Token> {
        let commitment = self.commitment.into_tokens();
        let data_length = Token::Uint(U256::from(self.data_length));
        let blob_quorum_params = self.blob_quorum_params.into_iter().map(|quorum| Token::Tuple(quorum.into_tokens())).collect();

        vec![Token::Tuple(commitment), data_length,Token::Array(blob_quorum_params)]
    }
}

#[derive(Debug)]
pub struct BatchHeader {
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

impl Tokenize for BatchHeader {
    fn into_tokens(self) -> Vec<Token> {
        let batch_root = Token::FixedBytes(self.batch_root);
        let quorum_numbers = self.quorum_numbers.into_token();
        let quorum_signed_percentages = self.quorum_signed_percentages.into_token();
        let reference_block_number = Token::Uint(U256::from(self.reference_block_number));

        vec![batch_root, quorum_numbers,quorum_signed_percentages,reference_block_number]
    }
}

#[derive(Debug)]
pub struct BatchMetadata {
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

impl Tokenize for BatchMetadata {
    fn into_tokens(self) -> Vec<Token> {
        let batch_header = self.batch_header.into_tokens();
        let signatory_record_hash = Token::FixedBytes(self.signatory_record_hash);
        let confirmation_block_number = Token::Uint(U256::from(self.confirmation_block_number));

        vec![Token::Tuple(batch_header), signatory_record_hash,confirmation_block_number]
    }
}

#[derive(Debug)]
pub struct BlobVerificationProof {
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

impl Tokenize for BlobVerificationProof {
    fn into_tokens(self) -> Vec<Token> {
        let batch_id = Token::Uint(U256::from(self.batch_id));
        let blob_index = Token::Uint(U256::from(self.blob_index));
        let batch_medatada = self.batch_medatada.into_tokens();
        let inclusion_proof = self.inclusion_proof.into_token();
        let quorum_indexes = self.quorum_indexes.into_token();

        vec![batch_id, blob_index,Token::Tuple(batch_medatada),inclusion_proof,quorum_indexes]
    }
}

#[derive(Debug)]
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

impl Tokenize for BlobInfo {
    fn into_tokens(self) -> Vec<Token> {
        let blob_header = self.blob_header.into_tokens();
        let blob_verification_proof = self.blob_verification_proof.into_tokens();

        vec![Token::Tuple(vec![Token::Tuple(blob_header),Token::Tuple(blob_verification_proof)])]
    }
}


