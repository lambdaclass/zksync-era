#[derive(Debug)]
pub enum MemStoreError {
    BlobToLarge,
    IncorrectString,
    BlobAlreadyExists,
    IncorrectCommitment,
    BlobNotFound,
}
