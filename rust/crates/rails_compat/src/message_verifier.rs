use hmac::{Hmac, Mac};
use jiff::Timestamp;
use serde_json::Value;
use subtle::ConstantTimeEq;

use crate::{Error, encoding, metadata};

pub use crate::metadata::Serializer;

/// `ActiveSupport::MessageVerifier`: `<base64 payload>--<hex HMAC of the base64 payload>`.
/// Each use in Rails configures it differently; the constructors below name them.
pub struct MessageVerifier {
    secret: Vec<u8>,
    digest: Digest,
    encoding: Encoding,
    serializer: Serializer,
    /// `rotate`/`fall_back_to`: tried in order when this verifier can't read a message.
    rotations: Vec<MessageVerifier>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Digest {
    Sha1,
    Sha256,
}

/// How the payload is Base64-encoded when generating. Reading is lenient in every case:
/// `MessageVerifier#decode` retries with the other alphabet, and `GlobalID::Verifier` uses
/// `urlsafe_decode64`, which accepts both alphabets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    /// `Base64.strict_encode64` (the default, `url_safe: false`).
    Strict,
    /// `url_safe: true`: URL-safe alphabet without padding.
    UrlSafe,
    /// `GlobalID::Verifier`: URL-safe alphabet *with* padding.
    UrlSafePadded,
}

impl MessageVerifier {
    pub fn new(secret: Vec<u8>, digest: Digest, encoding: Encoding, serializer: Serializer) -> Self {
        Self { secret, digest, encoding, serializer, rotations: Vec::new() }
    }

    pub fn fall_back_to(mut self, rotation: MessageVerifier) -> Self {
        self.rotations.push(rotation);
        self
    }

    pub fn generate(&self, value: &Value, purpose: Option<&str>, expires_at: Option<Timestamp>) -> String {
        self.sign(&metadata::serialize_with_metadata(self.serializer, value, purpose, expires_at))
    }

    /// `generate` for data the caller already encoded as JSON (in this verifier's serializer, e.g.
    /// `ActiveSupport::JSON` escaping for app verifiers), so key order is under the caller's
    /// control: `{"_rails":{"data":<data_json>,"exp":..,"pur":..}}`.
    pub fn generate_raw(&self, data_json: &str, purpose: Option<&str>, expires_at: Option<Timestamp>) -> String {
        self.sign(&metadata::serialize_dumped_with_metadata(self.serializer, data_json.as_bytes(), purpose, expires_at))
    }

    /// `verified`, returning the data re-encoded as JSON in this verifier's serializer. Key order
    /// is kept (serde_json `preserve_order`); escapes and number formatting are normalized, which
    /// only matters if the caller re-signs the returned string.
    pub fn verify_raw(&self, message: &str, purpose: Option<&str>, now: Timestamp) -> Result<String, Error> {
        self.verify(message, purpose, now).map(|value| self.serializer.encode_json(&value))
    }

    fn sign(&self, serialized: &[u8]) -> String {
        let encoded = match self.encoding {
            Encoding::Strict => encoding::strict_encode(serialized),
            Encoding::UrlSafe => encoding::urlsafe_encode_unpadded(serialized),
            Encoding::UrlSafePadded => encoding::urlsafe_encode_padded(serialized),
        };
        let digest = self.hex_digest(&encoded);
        format!("{encoded}--{digest}")
    }

    /// `verified`/`verify`: the value, or why it couldn't be read.
    pub fn verify(&self, message: &str, purpose: Option<&str>, now: Timestamp) -> Result<Value, Error> {
        match self.read_message(message, purpose, now) {
            Err(error) if error.rotates() => {
                for rotation in &self.rotations {
                    match rotation.read_message(message, purpose, now) {
                        Err(e) if e.rotates() => continue,
                        result => return result,
                    }
                }
                Err(error)
            }
            result => result,
        }
    }

    fn read_message(&self, message: &str, purpose: Option<&str>, now: Timestamp) -> Result<Value, Error> {
        let encoded = self.extract_encoded(message).ok_or(Error::InvalidSignature)?;
        let decoded = encoding::urlsafe_decode(encoded).ok_or(Error::InvalidSignature)?;
        metadata::deserialize_with_metadata(self.serializer, &decoded, purpose, now, encoding::urlsafe_decode)
    }

    /// `extract_encoded`: the digest is the last `2 * digest_length` characters, preceded by `--`.
    fn extract_encoded<'a>(&self, signed: &'a str) -> Option<&'a str> {
        let digest_length = self.hex_length();
        let index = signed.len().checked_sub(digest_length + 2)?;
        if !signed.is_char_boundary(index) || signed.get(index..index + 2) != Some("--") {
            return None;
        }
        let (encoded, digest) = (&signed[..index], &signed[index + 2..]);
        // `data.present? && digest.present?`
        if encoded.trim().is_empty() || digest.trim().is_empty() {
            return None;
        }
        self.digest_matches(encoded, digest).then_some(encoded)
    }

    fn hex_length(&self) -> usize {
        match self.digest {
            Digest::Sha1 => 40,
            Digest::Sha256 => 64,
        }
    }

    fn hex_digest(&self, data: &str) -> String {
        hex::encode(self.mac(data))
    }

    fn digest_matches(&self, data: &str, digest: &str) -> bool {
        // `ActiveSupport::SecurityUtils.secure_compare`: the length may leak, the contents don't.
        bool::from(digest.as_bytes().ct_eq(self.hex_digest(data).as_bytes()))
    }

    fn mac(&self, data: &str) -> Vec<u8> {
        match self.digest {
            Digest::Sha1 => {
                let mut mac = Hmac::<sha1::Sha1>::new_from_slice(&self.secret).expect("HMAC takes any key length");
                mac.update(data.as_bytes());
                mac.finalize().into_bytes().to_vec()
            }
            Digest::Sha256 => {
                let mut mac = Hmac::<sha2::Sha256>::new_from_slice(&self.secret).expect("HMAC takes any key length");
                mac.update(data.as_bytes());
                mac.finalize().into_bytes().to_vec()
            }
        }
    }
}

