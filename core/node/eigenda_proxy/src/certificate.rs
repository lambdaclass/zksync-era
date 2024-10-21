pub struct CertVerifier {
    eth_confirmation_depth: u64,
    //manager: todo!(), // TODO: add bindings
    //eth_client: todo!(), // TODO: inject client
}

impl CertVerifier {
    pub fn new() -> Self {
        todo!()
    }

    // TODO: this requires the bindings
    pub fn verify_batch(&self) {
        todo!()
    }

    pub fn verify_merkle_proof(
        &self,
        inclusion_proof: &[u8],
        root: &[u8],
        blob_index: u32,
        blob_header: crate::blob_info::BlobHeader,
    ) {
        todo!()
    }
}
