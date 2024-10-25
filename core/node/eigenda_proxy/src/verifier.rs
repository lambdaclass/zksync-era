use std::str::FromStr;

use ark_bn254::{Fq, G1Affine};
use rust_kzg_bn254::{blob::Blob, kzg::Kzg, polynomial::PolynomialFormat};

use crate::{
    blob_info::{BlobHeader, BlobInfo},
    common::G1Commitment,
};

pub enum VerificationError {
    WrongProof,
}

/// Processes the Merkle root proof
pub fn process_inclusion_proof(
    proof: &[u8],
    leaf: &[u8],
    index: u64,
) -> Result<Vec<u8>, VerificationError> {
    todo!()
}

pub struct VerifierConfig {
    // kzg_config: KzgConfig,
    verify_certs: bool,
    rpc_url: String,
    svc_manager_addr: String,
    eth_confirmation_deph: u64,
}

pub struct Verifier {
    g1: Vec<G1Affine>,
    kzg: Kzg,
    verify_certs: bool,  // TODO: change this.
    cert_verifier: bool, // TODO: change this.
}

impl Verifier {
    pub fn new(cfg: VerifierConfig) -> Self {
        let srs_points_to_load = 2097152 / 32; // 2 Mb / 32 (Max blob size)
        let kzg = Kzg::setup(
            "./resources/g1.point",
            "",
            "./resources/g2.point.powerOf2",
            268435456,
            srs_points_to_load,
            "".to_string(),
        );
        let kzg = kzg.unwrap();
        let g1: Vec<G1Affine> = kzg.get_g1_points();
        let cert_verifier = if cfg.verify_certs {
            false
            // TODO: create CertVerifier
        } else {
            true
        };
        // TODO: create new kzg verifier
        // let kzg_verifier = todo!();
        //

        Self {
            g1,
            kzg,
            verify_certs: false,
            cert_verifier: false,
        }
    }

    fn commit(&self, blob: Vec<u8>) -> G1Affine {
        let blob = Blob::from_bytes_and_pad(&blob.to_vec());
        self.kzg
            .blob_to_kzg_commitment(&blob, PolynomialFormat::InEvaluationForm)
            .unwrap()
    }

    pub fn verify_commitment(
        &self,
        expected_commitment: G1Commitment,
        blob: Vec<u8>,
    ) -> Result<(), VerificationError> {
        let actual_commitment = self.commit(blob);
        let expected_commitment = G1Affine::new_unchecked(
            Fq::from(num_bigint::BigUint::from_bytes_be(&expected_commitment.x)),
            Fq::from(num_bigint::BigUint::from_bytes_be(&expected_commitment.y)),
        );
        if actual_commitment != expected_commitment {
            return Err(VerificationError::WrongProof);
        }
        Ok(())
    }

    fn hash_encode_blob_header(&self, blob_header: BlobHeader) -> Vec<u8> {
        //let blob_hash = hash_blob_header(blob_header);
        vec![]
    }

    pub fn verify_merkle_proof(&self, cert: BlobInfo) -> Result<(), VerificationError> {
        let inclusion_proof = cert.blob_verification_proof.inclusion_proof;
        let root = cert
            .blob_verification_proof
            .batch_medatada
            .batch_header
            .batch_root;
        let blob_index = cert.blob_verification_proof.blob_index;
        let blob_header = cert.blob_header;

        let leafHash = self.hash_encode_blob_header(blob_header);

        Ok(())
    }

    pub fn verify_certificate(&self, cert: BlobInfo) -> Result<(), VerificationError> {
        self.verify_merkle_proof(cert)?;
        todo!()
    }
}

#[cfg(test)]
mod test {

    #[test]
    fn test_verify_commitment() {
        let verifier = super::Verifier::new(super::VerifierConfig {
            verify_certs: false,
            rpc_url: "".to_string(),
            svc_manager_addr: "".to_string(),
            eth_confirmation_deph: 0,
        });
        let commitment = super::G1Commitment {
            x: vec![
                22, 11, 176, 29, 82, 48, 62, 49, 51, 119, 94, 17, 156, 142, 248, 96, 240, 183, 134,
                85, 152, 5, 74, 27, 175, 83, 162, 148, 17, 110, 201, 74,
            ],
            y: vec![
                12, 132, 236, 56, 147, 6, 176, 135, 244, 166, 21, 18, 87, 76, 122, 3, 23, 22, 254,
                236, 148, 129, 110, 207, 131, 116, 58, 170, 4, 130, 191, 157,
            ],
        };
        let blob = vec![1u8; 100]; // Actual blob sent was this blob but kzg-padded, but Blob::from_bytes_and_pad padds it inside, so we don't need to pad it here.
        let result = verifier.verify_commitment(commitment, blob);
        assert_eq!(result.is_ok(), true);
    }
}
