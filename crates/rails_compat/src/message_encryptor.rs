use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use jiff::Timestamp;
use rand::RngCore;
use serde_json::Value;

use crate::{Error, encoding, metadata};

pub use crate::metadata::Serializer;

/// `ActiveSupport::MessageEncryptor` with `aes-256-gcm`, as the encrypted cookie jar builds it:
/// `<base64 ciphertext>--<base64 12-byte IV>--<base64 16-byte auth tag>`, strict Base64, empty
/// auth data, no separate signature.
pub struct MessageEncryptor {
    cipher: Aes256Gcm,
    serializer: Serializer,
}

const IV_LENGTH: usize = 12;
const AUTH_TAG_LENGTH: usize = 16;
/// Strict Base64 lengths of the IV and auth tag (with padding).
const ENCODED_IV_LENGTH: usize = 16;
const ENCODED_AUTH_TAG_LENGTH: usize = 24;

impl MessageEncryptor {
    /// `secret` must be 32 bytes (`key_generator.generate_key(salt, 32)`).
    pub fn new(secret: &[u8], serializer: Serializer) -> Self {
        let cipher = Aes256Gcm::new_from_slice(secret).expect("aes-256-gcm needs a 32-byte key");
        Self { cipher, serializer }
    }

    pub fn encrypt_and_sign(&self, value: &Value, purpose: Option<&str>, expires_at: Option<Timestamp>) -> String {
        let plaintext = metadata::serialize_with_metadata(self.serializer, value, purpose, expires_at);
        let mut iv = [0u8; IV_LENGTH];
        rand::rng().fill_bytes(&mut iv);
        self.encrypt_with_iv(&plaintext, &iv)
    }

    fn encrypt_with_iv(&self, plaintext: &[u8], iv: &[u8; IV_LENGTH]) -> String {
        let sealed = self
            .cipher
            .encrypt(Nonce::from_slice(iv), Payload { msg: plaintext, aad: b"" })
            .expect("aes-gcm encryption doesn't fail for in-range lengths");
        let (ciphertext, tag) = sealed.split_at(sealed.len() - AUTH_TAG_LENGTH);
        [ciphertext, iv.as_slice(), tag].map(encoding::strict_encode).join("--")
    }

    pub fn decrypt_and_verify(&self, message: &str, purpose: Option<&str>, now: Timestamp) -> Result<Value, Error> {
        let plaintext = self.decrypt(message).ok_or(Error::InvalidSignature)?;
        metadata::deserialize_with_metadata(self.serializer, &plaintext, purpose, now, encoding::strict_decode)
    }

    /// The decrypted bytes, before any envelope handling.
    pub fn decrypt(&self, message: &str) -> Option<Vec<u8>> {
        let (ciphertext, iv, tag) = extract_parts(message)?;
        let (ciphertext, iv, tag) = (encoding::strict_decode(ciphertext)?, encoding::strict_decode(iv)?, encoding::strict_decode(tag)?);
        if iv.len() != IV_LENGTH || tag.len() != AUTH_TAG_LENGTH {
            return None;
        }
        let sealed = [ciphertext, tag].concat();
        self.cipher.decrypt(Nonce::from_slice(&iv), Payload { msg: &sealed, aad: b"" }).ok()
    }
}

/// `extract_parts`: fixed-length IV and tag at the end, each preceded by `--`.
fn extract_parts(message: &str) -> Option<(&str, &str, &str)> {
    let tag_start = message.len().checked_sub(ENCODED_AUTH_TAG_LENGTH)?;
    let iv_start = tag_start.checked_sub(2 + ENCODED_IV_LENGTH)?;
    let ciphertext_end = iv_start.checked_sub(2)?;
    if message.get(tag_start - 2..tag_start)? != "--" || message.get(ciphertext_end..iv_start)? != "--" {
        return None;
    }
    Some((&message[..ciphertext_end], message.get(iv_start..tag_start - 2)?, message.get(tag_start..)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_rejects_tampering() {
        let encryptor = MessageEncryptor::new(&[7u8; 32], Serializer::Null);
        let now = Timestamp::UNIX_EPOCH;
        let message = encryptor.encrypt_and_sign(&Value::String("hi".into()), Some("p"), None);
        assert_eq!(encryptor.decrypt_and_verify(&message, Some("p"), now), Ok(Value::String("hi".into())));
        assert_eq!(encryptor.decrypt_and_verify(&message, Some("q"), now), Err(Error::PurposeMismatch));
        let tampered = format!("A{}", &message[1..]);
        assert!(encryptor.decrypt_and_verify(&tampered, Some("p"), now).is_err() || tampered == message);
        assert_eq!(encryptor.decrypt_and_verify("short", None, now), Err(Error::InvalidSignature));
    }
}
