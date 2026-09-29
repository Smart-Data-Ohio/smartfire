//! Webhook signatures: the ones Smartfire sends (`Webhook#post_payload`, `app/models/webhook.rb`)
//! and the ones GitHub sends us (`Github::WebhooksController#valid_signature?`).
use hmac::{Hmac, Mac};
use jiff::Timestamp;
use sha2::Sha256;

use crate::message_verifier::constant_time_eq;

pub const SIGNATURE_HEADER: &str = "X-Smartfire-Signature";
pub const TIMESTAMP_HEADER: &str = "X-Smartfire-Timestamp";
pub const GITHUB_SIGNATURE_HEADER: &str = "X-Hub-Signature-256";

const PREFIX: &str = "sha256=";

fn hmac_sha256_hex(secret: &[u8], parts: &[&[u8]]) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret).expect("HMAC takes a key of any length");
    for part in parts {
        mac.update(part);
    }
    hex::encode(mac.finalize().into_bytes())
}

/// `"sha256=#{OpenSSL::HMAC.hexdigest("SHA256", secret, "#{timestamp}.#{payload}")}"`.
pub fn smartfire_signature(secret: &str, timestamp: &str, body: &[u8]) -> String {
    format!("{PREFIX}{}", hmac_sha256_hex(secret.as_bytes(), &[timestamp.as_bytes(), b".", body]))
}

/// The headers `post_payload(payload, secret:)` sends at `now`, in order: `Content-Type`, the
/// unix-seconds timestamp, and the signature when the secret is `present?` (not blank).
pub fn smartfire_headers(secret: Option<&str>, body: &[u8], now: Timestamp) -> Vec<(&'static str, String)> {
    let timestamp = now.as_second().to_string();
    let mut headers = vec![("Content-Type", "application/json".to_string()), (TIMESTAMP_HEADER, timestamp.clone())];
    if let Some(secret) = secret.filter(|secret| !secret.trim().is_empty()) {
        headers.push((SIGNATURE_HEADER, smartfire_signature(secret, &timestamp, body)));
    }
    headers
}

/// Checks an `X-Smartfire-Signature` the way a receiver should: exact, constant-time. (Smartfire
/// itself only signs; this is for tests and for anything in the port that receives them.)
pub fn verify_smartfire_signature(secret: &str, timestamp: &str, body: &[u8], signature: &str) -> bool {
    constant_time_eq(signature.as_bytes(), smartfire_signature(secret, timestamp, body).as_bytes())
}

/// `valid_signature?(secret)`: the `X-Hub-Signature-256` header (missing is `""`) starts with
/// `sha256=` and equals `sha256=<hex HMAC-SHA256(secret, raw body)>` exactly (so the hex is
/// lowercase), compared in constant time.
pub fn verify_github_signature(secret: &str, raw_body: &[u8], signature: Option<&str>) -> bool {
    let signature = signature.unwrap_or("");
    let expected = format!("{PREFIX}{}", hmac_sha256_hex(secret.as_bytes(), &[raw_body]));
    signature.starts_with(PREFIX) && constant_time_eq(signature.as_bytes(), expected.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signatures_bind_the_secret_timestamp_and_body() {
        let signature = smartfire_signature("secret", "1767268800", b"{}");
        assert!(verify_smartfire_signature("secret", "1767268800", b"{}", &signature));
        assert!(!verify_smartfire_signature("other", "1767268800", b"{}", &signature));
        assert!(!verify_smartfire_signature("secret", "1767268801", b"{}", &signature));
        assert!(!verify_smartfire_signature("secret", "1767268800", b"{ }", &signature));
        assert!(!verify_smartfire_signature("secret", "1767268800", b"{}", &signature.to_uppercase()));
    }

    #[test]
    fn github_signatures_need_the_exact_prefixed_lowercase_hmac() {
        let body = br#"{"action":"opened"}"#;
        let valid = format!("sha256={}", hmac_sha256_hex(b"secret", &[body]));
        assert!(verify_github_signature("secret", body, Some(&valid)));
        for invalid in [
            String::new(),
            valid.trim_start_matches("sha256=").to_string(),
            valid.to_uppercase(),
            format!("{valid} "),
            valid[..valid.len() - 1].to_string(),
            valid.replace("sha256=", "sha1="),
        ] {
            assert!(!verify_github_signature("secret", body, Some(&invalid)), "{invalid:?}");
        }
        assert!(!verify_github_signature("secret", body, None));
        assert!(!verify_github_signature("other", body, Some(&valid)));
        assert!(!verify_github_signature("secret", b"{}", Some(&valid)));
    }
}
