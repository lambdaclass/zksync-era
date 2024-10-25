use std::{
    fs::File,
    io::{self, BufReader, Read},
    str::FromStr,
};

use ark_bn254::{Fq, G1Affine};
use crossbeam_channel::{bounded, Receiver, Sender};
use lambdaworks_math::{
    elliptic_curve::{
        short_weierstrass::{
            curves::bn_254::{
                curve::{BN254Curve, BN254FieldElement},
                default_types::FrField,
            },
            traits::Compress,
        },
        traits::IsEllipticCurve,
    },
    field::{element::FieldElement, traits::IsPrimeField},
    msm::naive::msm,
    traits::ByteConversion,
};
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

    fn read_file_chunks(
        file_path: &str,
        sender: Sender<(Vec<u8>, usize, bool)>,
        point_size: usize,
        num_points: u32,
        is_native: bool,
    ) -> io::Result<()> {
        let file = File::open(file_path)?;
        let mut reader = BufReader::new(file);
        let mut position = 0;
        let mut buffer = vec![0u8; point_size];

        let mut i = 0;
        while let Ok(bytes_read) = reader.read(&mut buffer) {
            if bytes_read == 0 {
                break;
            }
            sender
                .send((buffer[..bytes_read].to_vec(), position, is_native))
                .unwrap();
            position += bytes_read;
            buffer.resize(point_size, 0); // Ensure the buffer is always the correct size
            i += 1;
            if num_points == i {
                break;
            }
        }
        Ok(())
    }

    fn process_chunks(
        receiver: Receiver<(Vec<u8>, usize, bool)>,
    ) -> Vec<(
        lambdaworks_math::elliptic_curve::short_weierstrass::point::ShortWeierstrassProjectivePoint<
            BN254Curve,
        >,
        usize,
    )> {
        #[allow(clippy::unnecessary_filter_map)]
        receiver
            .iter()
            .map(|(chunk, position, is_native)| {
                let mut chunk_clone = chunk.clone();
                let new_chunk = chunk_clone.as_mut_slice();
                let point = BN254Curve::decompress_g1_point(new_chunk).unwrap();
                (point, position)
            })
            .collect()
    }

    // Based from rust_kzg_bn254
    fn read_g1_parallel(file_path: String,
        srs_points_to_load: u32,
    is_native: bool) -> Result<Vec<lambdaworks_math::elliptic_curve::short_weierstrass::point::ShortWeierstrassProjectivePoint<BN254Curve>>, ()>{
        let (sender, receiver) = bounded::<(Vec<u8>, usize, bool)>(1000);

        // Spawning the reader thread
        let reader_thread = std::thread::spawn(
            move || -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                Self::read_file_chunks(&file_path, sender, 32, srs_points_to_load, is_native)
                    .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })
            },
        );

        let num_workers = num_cpus::get();

        let workers: Vec<_> = (0..num_workers)
            .map(|_| {
                let receiver = receiver.clone();
                std::thread::spawn(move || Self::process_chunks(receiver))
            })
            .collect();

        // Wait for the reader thread to finish
        reader_thread.join().unwrap().unwrap();

        // Collect and sort results
        let mut all_points = Vec::new();
        for worker in workers {
            let points = worker.join().expect("Worker thread panicked");
            all_points.extend(points);
        }

        // Sort by original position to maintain order
        all_points.sort_by_key(|&(_, position)| position);

        Ok(all_points.iter().map(|(point, _)| point.clone()).collect())
    }

    fn commit_lambdaworks(&self,blob: Vec<u8>) -> lambdaworks_math::elliptic_curve::short_weierstrass::point::ShortWeierstrassProjectivePoint<BN254Curve>{
        let mut fr_fields = vec![];
        for i in 0..blob.len() / 32 {
            let mut chunk = vec![0u8, 32];
            if (i + 1) * 32 > blob.len() {
                chunk = blob[i * 32..(blob.len() - 1)].to_vec();
            } else {
                chunk = blob[i * 32..(i + 1) * 32].to_vec();
            }
            let fr_field = FrField::from_hex(&hex::encode(chunk)).unwrap();
            fr_fields.push(fr_field);
        }

        println!("FR FIELDS DONE");

        let g1s = Self::read_g1_parallel("./resources/g1.point".to_string(), 2097152 / 32, false)
            .unwrap();
        println!("G1S DONE");
        let commitment = msm(&fr_fields, &g1s[..fr_fields.len()]).unwrap();
        commitment
    }

    pub fn verify_commitment(
        &self,
        expected_commitment: G1Commitment,
        blob: Vec<u8>,
    ) -> Result<(), VerificationError> {
        println!("kzg ark");
        let actual_commitment = self.commit(blob.clone());
        let expected_commitment_ark = G1Affine::new_unchecked(
            Fq::from(num_bigint::BigUint::from_bytes_be(&expected_commitment.x)),
            Fq::from(num_bigint::BigUint::from_bytes_be(&expected_commitment.y)),
        );
        if actual_commitment != expected_commitment_ark {
            return Err(VerificationError::WrongProof);
        }

        println!("lambdaworks");

        let actual_commitment_lambdaworks =
            self.commit_lambdaworks(kzgpad_rs::convert_by_padding_empty_byte(&blob.to_vec()));
        println!("finished commit lambdaworks");
        let expected_commitment_lambdaworks = BN254Curve::create_point_from_affine(
            BN254FieldElement::from_bytes_be(&expected_commitment.x).unwrap(),
            BN254FieldElement::from_bytes_be(&expected_commitment.y).unwrap(),
        )
        .unwrap();
        if actual_commitment_lambdaworks != expected_commitment_lambdaworks {
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
