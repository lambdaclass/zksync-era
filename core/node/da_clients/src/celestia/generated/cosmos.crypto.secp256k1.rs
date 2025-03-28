// This file is @generated by prost-build.
/// PubKey defines a secp256k1 public key.
///
/// Key is the compressed form of the pubkey. The first byte depends is a 0x02 byte
/// if the y-coordinate is the lexicographically largest of the two associated with
/// the x-coordinate. Otherwise the first byte is a 0x03.
/// This prefix is followed with the x-coordinate.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PubKey {
    #[prost(bytes = "bytes", tag = "1")]
    pub key: ::prost::bytes::Bytes,
}
impl ::prost::Name for PubKey {
    const NAME: &'static str = "PubKey";
    const PACKAGE: &'static str = "cosmos.crypto.secp256k1";
    fn full_name() -> ::prost::alloc::string::String {
        "cosmos.crypto.secp256k1.PubKey".into()
    }
    fn type_url() -> ::prost::alloc::string::String {
        "/cosmos.crypto.secp256k1.PubKey".into()
    }
}
