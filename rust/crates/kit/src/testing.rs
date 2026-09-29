//! Test support: an insecure, transparent [`Crypto`] and a frozen clock.
//!
//! `TestCrypto` has the same *shape* as Rails' (signed values are `data--digest`, encrypted values
//! are opaque) but none of the byte compatibility, which `rails_compat` owns. Only compiled for
//! tests: this crate's own, and others' through the `test-support` feature.

use std::sync::Arc;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use jiff::Timestamp;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::clock::{FrozenClock, SharedClock};
use crate::crypto::{Crypto, SharedCrypto};
use rails_compat::message_verifier::{Digest as VerifierDigest, Encoding, Serializer};
use rails_compat::{MessageEncryptor, MessageVerifier};

pub const TEST_TIME: &str = "2024-06-01T12:00:00Z";

pub fn crypto() -> SharedCrypto {
    Arc::new(TestCrypto::default())
}

pub fn frozen_clock() -> SharedClock {
    Arc::new(FrozenClock::new(TEST_TIME.parse().unwrap()))
}

#[derive(Debug, Clone)]
pub struct TestCrypto {
    secret: String,
}

impl Default for TestCrypto {
    fn default() -> Self {
        Self { secret: "test-secret".into() }
    }
}

impl TestCrypto {
    fn digest(&self, parts: &[&str]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.secret.as_bytes());
        for part in parts {
            hasher.update(b"\0");
            hasher.update(part.as_bytes());
        }
        hex::encode(&hasher.finalize()[..16])
    }

    fn envelope(&self, kind: &str, name: &str, value: Value, expires_at: Option<Timestamp>) -> String {
        let payload = json!({ "v": value, "exp": expires_at.map(|t| t.to_string()), "pur": format!("cookie.{name}") });
        let data = STANDARD.encode(payload.to_string());
        let digest = self.digest(&[kind, &data]);
        format!("{data}--{digest}")
    }

    fn open(&self, kind: &str, name: &str, raw: &str, now: Timestamp) -> Option<Value> {
        let (data, digest) = raw.rsplit_once("--")?;
        if self.digest(&[kind, data]) != digest {
            return None;
        }
        let payload: Value = serde_json::from_slice(&STANDARD.decode(data).ok()?).ok()?;
        if payload["pur"] != format!("cookie.{name}") {
            return None;
        }
        if let Some(exp) = payload["exp"].as_str()
            && exp.parse::<Timestamp>().ok()? <= now {
                return None;
            }
        Some(payload["v"].clone())
    }
}

impl Crypto for TestCrypto {
    fn sign_cookie(&self, name: &str, value: &str, expires_at: Option<Timestamp>) -> String {
        self.envelope("signed", name, Value::String(value.into()), expires_at)
    }

    fn verify_signed_cookie(&self, name: &str, raw: &str, now: Timestamp) -> Option<String> {
        self.open("signed", name, raw, now)?.as_str().map(str::to_string)
    }

    fn encrypt_cookie(&self, name: &str, value: &Value, expires_at: Option<Timestamp>) -> String {
        // A random prefix so repeated encryptions differ, like AES-GCM with a fresh IV.
        let nonce: u32 = rand::random();
        format!("{nonce:08x}{}", self.envelope("encrypted", name, value.clone(), expires_at))
    }

    fn decrypt_cookie(&self, name: &str, raw: &str, now: Timestamp) -> Option<Value> {
        self.open("encrypted", name, raw.get(8..)?, now)
    }
}

/// Our Rails app's cookie jars: `cookies.signed` (HMAC-SHA1) and `cookies.encrypted`
/// (aes-256-gcm), both with the legacy `_rails` envelope and JSON values, keyed by
/// PBKDF2-HMAC-**SHA1** over `secret_key_base` (1000 iterations).
///
/// Not the SHA256 `load_defaults` asks for: `config/initializers/active_record_encryption.rb`
/// calls `Rails.application.key_generator` while the app initializes, which memoizes a generator
/// built before `key_generator_hash_digest_class` takes effect (an `after_initialize`), and the
/// cookie jars use that generator. `rails_compat::KeyGenerator` only derives with SHA256, which
/// reads stock Campfire's cookies (`vectors/rails_compat.json`) but not ours. Until it can do both,
/// tests that replay our Rails app's cookies use this. (Values are JSON-encoded with `serde_json`,
/// which matches `ActiveSupport::JSON` for the plain strings, numbers and hashes tests use.)
pub struct OurRailsCrypto {
    verifier: MessageVerifier,
    encryptor: MessageEncryptor,
}

impl OurRailsCrypto {
    pub fn new(secret_key_base: &str) -> Self {
        let key = |salt: &str, length: usize| {
            let mut key = vec![0u8; length];
            pbkdf2::pbkdf2_hmac::<sha1::Sha1>(secret_key_base.as_bytes(), salt.as_bytes(), 1000, &mut key);
            key
        };
        Self {
            verifier: MessageVerifier::new(key("signed cookie", 64), VerifierDigest::Sha1, Encoding::Strict, Serializer::Null),
            encryptor: MessageEncryptor::new(&key("authenticated encrypted cookie", 32), Serializer::Null),
        }
    }

    fn load(dumped: Value) -> Option<Value> {
        serde_json::from_str(dumped.as_str()?).ok()
    }

    /// `cookies.encrypted[name]`
    pub fn decrypt(&self, name: &str, raw: &str, now: Timestamp) -> Option<Value> {
        let purpose = format!("cookie.{name}");
        let dumped = self.encryptor.decrypt_and_verify(raw, Some(&purpose), now).or_else(|_| self.encryptor.decrypt_and_verify(raw, None, now));
        Self::load(dumped.ok()?)
    }
}

impl Crypto for OurRailsCrypto {
    fn sign_cookie(&self, name: &str, value: &str, expires_at: Option<Timestamp>) -> String {
        let dumped = Value::String(serde_json::to_string(value).unwrap());
        self.verifier.generate(&dumped, Some(&format!("cookie.{name}")), expires_at)
    }

    fn verify_signed_cookie(&self, name: &str, raw: &str, now: Timestamp) -> Option<String> {
        let purpose = format!("cookie.{name}");
        let dumped = self.verifier.verify(raw, Some(&purpose), now).or_else(|_| self.verifier.verify(raw, None, now)).ok()?;
        Self::load(dumped)?.as_str().map(str::to_string)
    }

    fn encrypt_cookie(&self, name: &str, value: &Value, expires_at: Option<Timestamp>) -> String {
        let dumped = Value::String(serde_json::to_string(value).unwrap());
        self.encryptor.encrypt_and_sign(&dumped, Some(&format!("cookie.{name}")), expires_at)
    }

    fn decrypt_cookie(&self, name: &str, raw: &str, now: Timestamp) -> Option<Value> {
        self.decrypt(name, raw, now)
    }
}
