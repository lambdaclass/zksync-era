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
    kzg_verifier: bool, // TODO: change this.
    verify_certs: bool,
    cert_verifier: bool, // TODO: change this.
}

impl Verifier {
    pub fn new(cfg: VerifierConfig) -> Self {
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
            kzg_verifier: todo!(),
            verify_certs: cfg.verify_certs,
            cert_verifier,
        }
    }
}
