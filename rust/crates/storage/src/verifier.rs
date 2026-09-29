//! `ActiveStorage.verifier` (`Rails.application.message_verifier("ActiveStorage")`).
//!
//! Active Storage signs blob ids (purpose "blob_id" — `ActiveStorage::Blob` overrides both the
//! signed-id verifier and `combine_signed_id_purposes`, so this is *not* the Active Record
//! signed-id verifier), variation keys ("variation"), disk URLs ("blob_key") and direct-upload
//! tokens ("blob_token"). The data is passed as already-encoded JSON because key order is part of
//! the signed bytes (e.g. `{key:, disposition:, content_type:, service_name:}`).
//!
//! The app plugs `rails_compat`'s message verifier in through the [`Verifier`] trait;
//! [`AppMessageVerifier`] is a self-contained implementation the golden-vector tests use.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use hmac::{Hmac, Mac};
use sha1::Sha1;

use crate::json::Json;

pub trait Verifier: Send + Sync {
    /// `verifier.generate(data, purpose:, expires_at:)`, with `data_json` the encoded data.
    fn generate(&self, data_json: &str, purpose: &str, expires_at: Option<jiff::Timestamp>) -> String;

    /// `verifier.verified(message, purpose:)`: the data's JSON when valid, unexpired and on purpose.
    fn verified(&self, message: &str, purpose: &str, now: jiff::Timestamp) -> Option<String>;
}

/// A Rails 7.1+ `ActiveSupport::MessageVerifier` with the app defaults: HMAC-SHA1, strict (not
/// URL-safe) Base64, the JSON serializer and metadata inside the `_rails` envelope.
pub struct AppMessageVerifier {
    secret: Vec<u8>,
}

impl AppMessageVerifier {
    /// `secret` is `key_generator.generate_key("ActiveStorage")` (64 bytes).
    pub fn new(secret: Vec<u8>) -> Self {
        Self { secret }
    }

    fn digest(&self, data: &str) -> String {
        let mut mac = Hmac::<Sha1>::new_from_slice(&self.secret).expect("any key length");
        mac.update(data.as_bytes());
        mac.finalize().into_bytes().iter().map(|b| format!("{b:02x}")).collect()
    }
}

impl Verifier for AppMessageVerifier {
    fn generate(&self, data_json: &str, purpose: &str, expires_at: Option<jiff::Timestamp>) -> String {
        let mut envelope = format!("{{\"_rails\":{{\"data\":{data_json}");
        if let Some(expires_at) = expires_at {
            envelope.push_str(&format!(",\"exp\":{}", Json::String(iso8601_ms(expires_at)).encode()));
        }
        envelope.push_str(&format!(",\"pur\":{}}}}}", Json::String(purpose.to_string()).encode()));
        let data = STANDARD.encode(envelope);
        let digest = self.digest(&data);
        format!("{data}--{digest}")
    }

    fn verified(&self, message: &str, purpose: &str, now: jiff::Timestamp) -> Option<String> {
        let (data, digest) = message.rsplit_once("--")?;
        if data.is_empty() || !constant_time_eq(self.digest(data).as_bytes(), digest.as_bytes()) {
            return None;
        }
        let decoded = String::from_utf8(STANDARD.decode(data).ok()?).ok()?;
        let envelope = Json::parse(&decoded).ok()?;
        let rails = envelope.get("_rails")?;
        if rails.get("pur").and_then(Json::as_str) != Some(purpose) {
            return None;
        }
        if let Some(exp) = rails.get("exp") {
            let exp: jiff::Timestamp = exp.as_str()?.parse().ok()?;
            if now >= exp {
                return None;
            }
        }
        Some(rails.get("data")?.encode())
    }
}

/// `Time#iso8601(3)` in UTC, as MessageVerifier writes expirations.
fn iso8601_ms(t: jiff::Timestamp) -> String {
    t.strftime("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}
