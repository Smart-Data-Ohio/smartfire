//! `ActiveRecord::Encryption` (non-deterministic) exactly as Smartfire configures it:
//! `config/initializers/active_record_encryption.rb` sets the primary key, deterministic key and
//! key derivation salt to `Rails.application.key_generator.generate_key("active_record_encryption/
//! {primary,deterministic,salt}", 32)` (so from the app's SHA1 key generator, see
//! [`crate::key_generator`]), and `load_defaults 8.2` leaves `hash_digest_class` at SHA256,
//! `store_key_references` off, `support_unencrypted_data` off, no previous schemes, and Zlib
//! compression. `vectors/rails_compat_smartfire.json` (`ar_encryption.config`) pins all of it.
//!
//! The cipher key is `DerivedSecretKeyProvider.new(primary_key)`'s: PBKDF2-HMAC-SHA256 of the
//! primary key over the salt at `ActiveSupport::KeyGenerator`'s default 2**16 iterations.
//!
//! A column holds `JSON.dump({p:, h:})` (`active_record/encryption/message_serializer.rb`):
//! `p` is the strict Base64 AES-256-GCM ciphertext and `h` the headers, written in this order:
//! `iv` (12 bytes) and `at` (the 16-byte auth tag) from `cipher/aes256_gcm.rb`, `e` (the
//! plaintext's encoding name, only when it isn't UTF-8) from `cipher.rb`, and `c: true` when the
//! plaintext was over 140 bytes and was Zlib-deflated first (`encryptor.rb`). String header values
//! are strict Base64. The headers are not authenticated.
//!
//! Every way Rails fails to decrypt is a [`DecryptionError`] here, never a panic.
use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use rand::RngCore;
use serde_json::{Map, Value, json};
use sha1::{Digest, Sha1};

use crate::key_generator::DEFAULT_ITERATIONS;
use crate::{Secrets, encoding};

pub const PRIMARY_KEY_SALT: &str = "active_record_encryption/primary";
pub const DETERMINISTIC_KEY_SALT: &str = "active_record_encryption/deterministic";
pub const KEY_DERIVATION_SALT: &str = "active_record_encryption/salt";
/// `Encryptor::THRESHOLD_TO_JUSTIFY_COMPRESSION`: plaintexts over this many bytes are deflated.
pub const COMPRESSION_THRESHOLD: usize = 140;

const KEY_LENGTH: usize = 32;
const IV_LENGTH: usize = 12;
const AUTH_TAG_LENGTH: usize = 16;
/// `Cipher::DEFAULT_ENCODING`: no `e` header means UTF-8.
const DEFAULT_ENCODING: &str = "UTF-8";
/// The encodings Smartfire's writers produce. Rails would decrypt any encoding Ruby knows (and
/// raises `ArgumentError` for unknown names); this port refuses the rest.
pub(crate) const KNOWN_ENCODINGS: &[&str] = &["UTF-8", "US-ASCII", "ASCII-8BIT", "BINARY"];

/// Every `encrypts` declaration in the app (`app/models/**`), with the encoding its writer's
/// plaintext has, which is what Rails records in the `e` header. Pass it to
/// [`ArEncryption::encrypt_with_encoding`] to write what Rails writes.
pub const ENCRYPTED_COLUMNS: &[EncryptedColumn] = &[
    EncryptedColumn { model: "Agent", table: "agents", column: "webhook_signing_secret", encoding: "US-ASCII" },
    EncryptedColumn { model: "FizzyConnectedAccount", table: "fizzy_connected_accounts", column: "access_token", encoding: "UTF-8" },
    EncryptedColumn { model: "GithubConnectedAccount", table: "github_connected_accounts", column: "access_token", encoding: "UTF-8" },
    EncryptedColumn { model: "GithubConnectedAccount", table: "github_connected_accounts", column: "refresh_token", encoding: "UTF-8" },
    EncryptedColumn { model: "GoogleAccount", table: "google_accounts", column: "access_token", encoding: "UTF-8" },
    EncryptedColumn { model: "GoogleAccount", table: "google_accounts", column: "refresh_token", encoding: "UTF-8" },
    EncryptedColumn { model: "SlackConnection", table: "slack_connections", column: "access_token", encoding: "UTF-8" },
    EncryptedColumn { model: "SlackWorkspace", table: "slack_workspaces", column: "client_secret", encoding: "UTF-8" },
    EncryptedColumn { model: "TwoFactorCredential", table: "two_factor_credentials", column: "secret", encoding: "ASCII-8BIT" },
    EncryptedColumn { model: "TwoFactorSetupSecret", table: "two_factor_setup_secrets", column: "secret", encoding: "ASCII-8BIT" },
    EncryptedColumn { model: "Webhook", table: "webhooks", column: "signing_secret", encoding: "US-ASCII" },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EncryptedColumn {
    pub model: &'static str,
    pub table: &'static str,
    pub column: &'static str,
    /// `SecureRandom.hex` is US-ASCII and `ROTP::Base32.random_base32` ASCII-8BIT; provider
    /// tokens and form input are UTF-8.
    pub encoding: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DecryptionError {
    /// `ActiveRecord::Encryption::Errors::Decryption`: not a message, wrong key, tampered,
    /// malformed headers.
    #[error("the value can't be decrypted")]
    Decryption,
    /// The auth tag checked out but the `c` flag's Zlib inflate failed (Rails raises
    /// `Zlib::DataError`, which it doesn't rescue).
    #[error("the decrypted value can't be decompressed")]
    Decompression,
    /// An `e` header naming an encoding Smartfire never writes (see `KNOWN_ENCODINGS`).
    #[error("the value's encoding header is not one Smartfire writes")]
    UnknownEncoding,
    /// [`ArEncryption::decrypt`] only: the plaintext isn't UTF-8.
    #[error("the decrypted value is not UTF-8")]
    NotUtf8,
}

/// A decrypted value: the bytes and the Ruby encoding Rails would tag them with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decrypted {
    pub bytes: Vec<u8>,
    pub encoding: String,
}

pub struct ArEncryption {
    cipher: Aes256Gcm,
    key_id: String,
}

impl ArEncryption {
    /// `ActiveRecord::Encryption.key_provider` as the app configures it.
    pub fn new(secrets: &Secrets) -> Self {
        // Rails keeps its DerivedSecretKeyProvider (and its derived keys) for the
        // application lifetime. Cache with these Secrets, including concurrent callers.
        let key = secrets.ar_encryption_key.get_or_init(|| {
            let primary_key = secrets.key_generator.generate_key(PRIMARY_KEY_SALT, KEY_LENGTH);
            let salt = secrets.key_generator.generate_key(KEY_DERIVATION_SALT, KEY_LENGTH);
            derive_key(&primary_key, &salt)
        });
        Self::from_key(key)
    }

    /// From the derived 32-byte cipher key itself. The type carries the length, so no key can
    /// make this fail.
    pub fn from_key(key: &[u8; KEY_LENGTH]) -> Self {
        let cipher = Aes256Gcm::new(key.into());
        // ActiveRecord::Encryption::Key#id
        let key_id = hex::encode(Sha1::digest(key))[..4].to_string();
        Self { cipher, key_id }
    }

    /// `Key#id`: the first four hex digits of the key's SHA1, matched against an `i` header.
    pub fn key_id(&self) -> &str {
        &self.key_id
    }

    /// Encrypts a UTF-8 value (no `e` header), as Rails does for a UTF-8 string.
    pub fn encrypt(&self, plaintext: &str) -> String {
        self.encrypt_with_encoding(plaintext.as_bytes(), DEFAULT_ENCODING)
    }

    /// Encrypts bytes that Ruby would hold in `encoding` (see [`EncryptedColumn::encoding`]).
    pub fn encrypt_with_encoding(&self, plaintext: &[u8], encoding: &str) -> String {
        let mut iv = [0u8; IV_LENGTH];
        rand::rng().fill_bytes(&mut iv);
        self.encrypt_with_iv(plaintext, encoding, &iv)
    }

    pub(crate) fn encrypt_with_iv(&self, plaintext: &[u8], encoding: &str, iv: &[u8; IV_LENGTH]) -> String {
        let compressed = plaintext.len() > COMPRESSION_THRESHOLD;
        let data = if compressed { deflate(plaintext) } else { plaintext.to_vec() };
        let sealed = self
            .cipher
            .encrypt(Nonce::from_slice(iv), Payload { msg: &data, aad: b"" })
            .expect("aes-gcm encryption doesn't fail for in-range lengths");
        let (ciphertext, tag) = sealed.split_at(sealed.len() - AUTH_TAG_LENGTH);

        let mut headers = Map::new();
        headers.insert("iv".into(), encoding::strict_encode(iv).into());
        headers.insert("at".into(), encoding::strict_encode(tag).into());
        if encoding != DEFAULT_ENCODING {
            headers.insert("e".into(), encoding::strict_encode(encoding.as_bytes()).into());
        }
        if compressed {
            headers.insert("c".into(), true.into());
        }
        json!({ "p": encoding::strict_encode(ciphertext), "h": headers }).to_string()
    }

    /// Decrypts a column value to a string. Values Smartfire writes are all UTF-8 compatible.
    pub fn decrypt(&self, ciphertext: &str) -> Result<String, DecryptionError> {
        String::from_utf8(self.decrypt_bytes(ciphertext)?.bytes).map_err(|_| DecryptionError::NotUtf8)
    }

    /// `Encryptor#decrypt`, following `MessageSerializer#load`, `KeyProvider#decryption_keys`,
    /// `Cipher::Aes256Gcm#decrypt` and `Encryptor#uncompress_if_needed`.
    pub fn decrypt_bytes(&self, ciphertext: &str) -> Result<Decrypted, DecryptionError> {
        let data: Value = serde_json::from_str(ciphertext).map_err(|_| DecryptionError::Decryption)?;
        let message = parse_message(&data, 1)?;

        // An `i` header names the key (store_key_references); a truthy one must be ours.
        if let Some(id) = message.headers.get("i").filter(|header| header.truthy())
            && !matches!(id, Header::Bytes(bytes) if bytes == self.key_id.as_bytes())
        {
            return Err(DecryptionError::Decryption);
        }

        let payload = message.payload.ok_or(DecryptionError::Decryption)?;
        let iv = message.headers.bytes("iv").filter(|iv| iv.len() == IV_LENGTH).ok_or(DecryptionError::Decryption)?;
        let tag = message.headers.bytes("at").filter(|tag| tag.len() == AUTH_TAG_LENGTH).ok_or(DecryptionError::Decryption)?;
        let sealed = [payload.as_slice(), tag].concat();
        let decrypted = self
            .cipher
            .decrypt(Nonce::from_slice(iv), Payload { msg: &sealed, aad: b"" })
            .map_err(|_| DecryptionError::Decryption)?;

        let encoding = match message.headers.get("e") {
            None | Some(Header::Scalar(Value::Null)) => DEFAULT_ENCODING.to_string(),
            Some(Header::Bytes(name)) => {
                let name = String::from_utf8(name.clone()).map_err(|_| DecryptionError::UnknownEncoding)?;
                if !KNOWN_ENCODINGS.contains(&name.as_str()) {
                    return Err(DecryptionError::UnknownEncoding);
                }
                name
            }
            Some(_) => return Err(DecryptionError::UnknownEncoding),
        };

        let compressed = message.headers.get("c").is_some_and(Header::truthy);
        let bytes = if compressed { inflate(&decrypted).ok_or(DecryptionError::Decompression)? } else { decrypted };
        Ok(Decrypted { bytes, encoding })
    }
}

#[cfg(test)]
thread_local! { static DERIVATIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) }; }
/// `ActiveRecord::Encryption::KeyGenerator#derive_key_from`: `ActiveSupport::KeyGenerator.new(
/// primary_key, hash_digest_class: SHA256).generate_key(salt, 32)`, which is
/// `OpenSSL::PKCS5.pbkdf2_hmac` at the default 2**16 iterations.
pub(crate) fn derive_key(primary_key: &[u8], salt: &[u8]) -> [u8; KEY_LENGTH] {
    #[cfg(test)]
    DERIVATIONS.with(|n| n.set(n.get() + 1));
    let mut key = [0u8; KEY_LENGTH];
    pbkdf2::pbkdf2_hmac::<sha2::Sha256>(primary_key, salt, DEFAULT_ITERATIONS, &mut key);
    key
}

/// `Zlib::Deflate.deflate(data)`: zlib format at the default level.
fn deflate(data: &[u8]) -> Vec<u8> {
    use std::io::Write;
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(data).expect("writing to a Vec");
    encoder.finish().expect("writing to a Vec")
}

fn inflate(data: &[u8]) -> Option<Vec<u8>> {
    use std::io::Read;
    let mut out = Vec::new();
    flate2::read::ZlibDecoder::new(data).read_to_end(&mut out).ok()?;
    Some(out)
}

struct Message {
    payload: Option<Vec<u8>>,
    headers: Headers,
}

/// A header value, as `Properties` allows them: a (Base64-decoded) string, a JSON scalar, or a
/// nested message.
enum Header {
    Bytes(Vec<u8>),
    Scalar(Value),
    #[allow(dead_code)] // Parsed for its validation; Smartfire doesn't use envelope encryption.
    Message(Box<Message>),
}

impl Header {
    fn truthy(&self) -> bool {
        !matches!(self, Header::Scalar(Value::Null | Value::Bool(false)))
    }
}

struct Headers(Vec<(String, Header)>);

impl Headers {
    fn get(&self, key: &str) -> Option<&Header> {
        self.0.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    fn bytes(&self, key: &str) -> Option<&[u8]> {
        match self.get(key)? {
            Header::Bytes(bytes) => Some(bytes),
            _ => None,
        }
    }
}

/// `MessageSerializer#parse_message`: a hash with a `p` key, and headers nested at most one level.
fn parse_message(data: &Value, level: usize) -> Result<Message, DecryptionError> {
    if level > 2 {
        return Err(DecryptionError::Decryption);
    }
    let data = data.as_object().filter(|data| data.contains_key("p")).ok_or(DecryptionError::Decryption)?;
    let payload = match &data["p"] {
        Value::Null => None,
        Value::String(encoded) => Some(encoding::strict_decode(encoded).ok_or(DecryptionError::Decryption)?),
        // Message#validate_payload_type: only strings (or nil).
        _ => return Err(DecryptionError::Decryption),
    };
    let headers = match data.get("h") {
        None | Some(Value::Null) => Headers(Vec::new()),
        Some(Value::Object(headers)) => Headers(
            headers
                .iter()
                .map(|(key, value)| Ok((key.clone(), parse_header(value, level)?)))
                .collect::<Result<_, DecryptionError>>()?,
        ),
        // An array of pairs iterates like a hash in Ruby, and anything else raises; neither is a
        // shape Rails writes, and none of them can carry a valid IV and tag.
        Some(_) => return Err(DecryptionError::Decryption),
    };
    Ok(Message { payload, headers })
}

fn parse_header(value: &Value, level: usize) -> Result<Header, DecryptionError> {
    match value {
        Value::Object(_) => Ok(Header::Message(Box::new(parse_message(value, level + 1)?))),
        Value::String(encoded) => Ok(Header::Bytes(encoding::strict_decode(encoded).ok_or(DecryptionError::Decryption)?)),
        // Properties::ALLOWED_VALUE_CLASSES has no Array.
        Value::Array(_) => Err(DecryptionError::Decryption),
        scalar => Ok(Header::Scalar(scalar.clone())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encryption() -> ArEncryption {
        ArEncryption::from_key(&[7u8; 32])
    }

    #[test]
    fn round_trips_short_long_and_empty_values() {
        let encryption = encryption();
        for value in ["", "a", &"x".repeat(140), &"x".repeat(141), &"é".repeat(500)] {
            let ciphertext = encryption.encrypt(value);
            assert_eq!(encryption.decrypt(&ciphertext).unwrap(), value);
            let headers = serde_json::from_str::<Value>(&ciphertext).unwrap()["h"].clone();
            assert_eq!(headers.get("c").is_some(), value.len() > COMPRESSION_THRESHOLD, "{value:?}");
        }
    }

    #[test]
    fn records_non_utf8_encodings() {
        let encryption = encryption();
        let ciphertext = encryption.encrypt_with_encoding(b"JBSWY3DPEHPK3PXP", "ASCII-8BIT");
        let decrypted = encryption.decrypt_bytes(&ciphertext).unwrap();
        assert_eq!((decrypted.bytes.as_slice(), decrypted.encoding.as_str()), (b"JBSWY3DPEHPK3PXP".as_slice(), "ASCII-8BIT"));
    }

    #[test]
    fn the_wrong_key_is_a_decryption_error() {
        let ciphertext = encryption().encrypt("secret");
        assert_eq!(ArEncryption::from_key(&[8u8; 32]).decrypt(&ciphertext), Err(DecryptionError::Decryption));
    }

    #[test]
    fn every_tampered_byte_is_rejected() {
        let encryption = encryption();
        for value in ["short token", &"long refresh token ".repeat(20)] {
            let ciphertext = encryption.encrypt(value);
            let message: Value = serde_json::from_str(&ciphertext).unwrap();
            for field in ["p", "iv", "at"] {
                let encoded = if field == "p" { &message["p"] } else { &message["h"][field] };
                let bytes = encoding::strict_decode(encoded.as_str().unwrap()).unwrap();
                for index in 0..bytes.len() {
                    let mut tampered_bytes = bytes.clone();
                    tampered_bytes[index] ^= 0x01;
                    let mut tampered = message.clone();
                    let target = if field == "p" { &mut tampered["p"] } else { &mut tampered["h"][field] };
                    *target = encoding::strict_encode(&tampered_bytes).into();
                    assert_eq!(encryption.decrypt(&tampered.to_string()), Err(DecryptionError::Decryption), "{field}[{index}]");
                }
            }
        }
    }

    #[test]
    fn garbage_never_panics() {
        let encryption = encryption();
        for input in ["", "null", "[]", "{}", "{\"p\":null}", "{\"p\":1}", "{\"p\":\"\",\"h\":{\"iv\":5,\"at\":[]}}", "\u{0}", "{\"p\":\"AA==\",\"h\":\"x\"}"] {
            assert!(encryption.decrypt(input).is_err(), "{input:?}");
        }
    }
}

#[cfg(test)]
#[test]
fn provider_derives_once_across_concurrent_instances_like_rails() {
    use std::sync::{Arc, Barrier};
    let secrets = Arc::new(Secrets::new("FAKE-cached-provider"));
    let barrier = Arc::new(Barrier::new(4));
    let workers: Vec<_> = (0..4).map(|_| {
        let secrets = secrets.clone(); let barrier = barrier.clone();
        std::thread::spawn(move || {
            barrier.wait();
            let first = ArEncryption::new(&secrets);
            let encrypted = first.encrypt("provider fixture");
            let second = ArEncryption::new(&secrets);
            assert_eq!(second.decrypt(&encrypted).unwrap(), "provider fixture");
            DERIVATIONS.with(std::cell::Cell::get)
        })
    }).collect();
    assert_eq!(workers.into_iter().map(|w| w.join().unwrap()).sum::<usize>(),1);
    let other = ArEncryption::new(&Secrets::new("FAKE-another-provider"));
    assert!(other.decrypt(&ArEncryption::new(&secrets).encrypt("separate keys")).is_err());
}
