use std::str::FromStr;

use ark_bn254::{Fq, G1Affine};

use crate::{common::G1Commitment, rust_kzg_bn254::Kzg};

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
    g1: bool, // TODO: change this.
    kzg: Kzg,
    verify_certs: bool,
    cert_verifier: bool, // TODO: change this.
}

impl Verifier {
    pub fn new(cfg: VerifierConfig) -> Self {
        let srs_points_to_load = 2097152 / 32; // 2 Mb / 32 (Max blob size)
        let kzg = Kzg::setup("../resources/g1.point", 268435456, srs_points_to_load);
        let kzg = kzg.unwrap();
        //let g1: Vec<G1Affine> = kzg.get_g1_points();
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
            g1: true,
            kzg: kzg,
            verify_certs: false,
            cert_verifier: false,
        }
    }

    fn commit(&self, blob: Vec<u8>) -> G1Affine {
        //let blob = Blob::from_bytes_and_pad(&blob.to_vec());
        //self.kzg.blob_to_kzg_commitment(&blob).unwrap()
        todo!()
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
                46, 103, 30, 170, 173, 163, 217, 225, 108, 178, 168, 65, 182, 189, 146, 95, 228,
                31, 156, 176, 160, 98, 173, 127, 56, 190, 187, 0, 221, 129, 137, 113,
            ],
            y: vec![
                24, 27, 147, 230, 31, 241, 198, 35, 65, 194, 169, 170, 87, 203, 87, 163, 121, 50,
                234, 169, 140, 182, 78, 155, 248, 178, 37, 3, 93, 128, 201, 148,
            ],
        };
        let blob = vec![1u8; 32];
        let result = verifier.verify_commitment(commitment, blob);
        assert_eq!(result.is_ok(), true);
    }
}
