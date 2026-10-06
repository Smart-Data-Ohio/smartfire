//! Blob keys and checksums.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use md5::{Digest, Md5};
use rand::Rng;

/// `ActiveStorage::Blob::MINIMUM_TOKEN_LENGTH`.
pub const KEY_LENGTH: usize = 28;

const BASE36_ALPHABET: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";

/// `SecureRandom.base36(28)` (`has_secure_token :key, length: 28`).
pub fn generate_key() -> String {
    let mut rng = rand::rng();
    (0..KEY_LENGTH).map(|_| BASE36_ALPHABET[rng.random_range(0..36)] as char).collect()
}

/// `OpenSSL::Digest::MD5.base64digest` of the whole content (`compute_checksum_in_chunks`).
pub fn checksum(data: &[u8]) -> String {
    STANDARD.encode(Md5::digest(data))
}

/// Streaming variant of [`checksum`] for files on disk.
pub fn checksum_file(path: &std::path::Path) -> std::io::Result<String> {
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Md5::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(STANDARD.encode(hasher.finalize()))
}
