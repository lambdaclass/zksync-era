use byteorder::{BigEndian, ByteOrder};
use secp256k1::{Message, Secp256k1, SecretKey};
use tiny_keccak::{Hasher, Keccak};

fn keccak256(input: &[u8]) -> [u8; 32] {
    let mut hasher = Keccak::v256();
    let mut output = [0u8; 32];
    hasher.update(input);
    hasher.finalize(&mut output);
    output
}

pub fn sign(challenge: u32, private_key: &SecretKey) -> Vec<u8> {
    let mut buf = [0u8; 4];
    BigEndian::write_u32(&mut buf, challenge);
    let hash = keccak256(&buf);
    let message = Message::from_slice(&hash).unwrap();
    let secp = Secp256k1::signing_only();
    let recoverable_sig = secp.sign_ecdsa_recoverable(&message, private_key);

    // Step 5: Convert recoverable signature to a 65-byte array (64 bytes for signature + 1 byte for recovery ID)
    let (recovery_id, sig_bytes) = recoverable_sig.serialize_compact();

    // Step 6: Append the recovery ID as the last byte to form a 65-byte signature
    let mut full_signature = [0u8; 65];
    full_signature[..64].copy_from_slice(&sig_bytes);
    full_signature[64] = recovery_id.to_i32() as u8; // Append the recovery ID as the last byte

    full_signature.to_vec()
}
