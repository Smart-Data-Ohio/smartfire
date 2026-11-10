//! Webhook signatures: the ones Smartfire sends (`Webhook#post_payload`, `app/models/webhook.rb`)
//! and the ones GitHub sends us (`Github::WebhooksController#valid_signature?`).
use hmac::{Hmac, Mac};
use jiff::Timestamp;
use sha2::Sha256;

use subtle::ConstantTimeEq;

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
    format!(
        "{PREFIX}{}",
        hmac_sha256_hex(secret.as_bytes(), &[timestamp.as_bytes(), b".", body])
    )
}

/// The headers `post_payload(payload, secret:)` sends at `now`, in order: `Content-Type`, the
/// unix-seconds timestamp, and the signature when the secret is `present?` (not blank).
pub fn smartfire_headers(
    secret: Option<&str>,
    body: &[u8],
    now: Timestamp,
) -> Vec<(&'static str, String)> {
    let timestamp = now.as_second().to_string();
    let mut headers = vec![
        ("Content-Type", "application/json".to_string()),
        (TIMESTAMP_HEADER, timestamp.clone()),
    ];
    if let Some(secret) = secret.filter(|secret| !secret.trim().is_empty()) {
        headers.push((
            SIGNATURE_HEADER,
            smartfire_signature(secret, &timestamp, body),
        ));
    }
    headers
}

/// Checks an `X-Smartfire-Signature` the way a receiver should: exact, constant-time. (Smartfire
/// itself only signs; this is for tests and for anything in the port that receives them.)
pub fn verify_smartfire_signature(
    secret: &str,
    timestamp: &str,
    body: &[u8],
    signature: &str,
) -> bool {
    bool::from(
        signature
            .as_bytes()
            .ct_eq(smartfire_signature(secret, timestamp, body).as_bytes()),
    )
}

/// Standard Webhooks uses the decoded whsec_ key, not its printable representation.
pub fn standard_key(secret: &str) -> Option<Vec<u8>> {
    use base64::Engine;
    let key = base64::engine::general_purpose::STANDARD
        .decode(secret.strip_prefix("whsec_")?)
        .ok()?;
    (24..=64).contains(&key.len()).then_some(key)
}

pub fn standard_signature(secret: &str, id: &str, timestamp: &str, body: &[u8]) -> Option<String> {
    use base64::Engine;
    let mut mac = Hmac::<Sha256>::new_from_slice(&standard_key(secret)?).ok()?;
    for part in [id.as_bytes(), b".", timestamp.as_bytes(), b".", body] {
        mac.update(part);
    }
    Some(format!(
        "v1,{}",
        base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes())
    ))
}

pub fn standard_headers(
    secret: &str,
    previous: Option<&str>,
    id: &str,
    body: &[u8],
    now: Timestamp,
) -> Option<Vec<(&'static str, String)>> {
    let timestamp = now.as_second().to_string();
    let mut signature = standard_signature(secret, id, &timestamp, body)?;
    if let Some(previous) = previous {
        signature.push(' ');
        signature.push_str(&standard_signature(previous, id, &timestamp, body)?);
    }
    Some(vec![
        ("Content-Type", "application/json".into()),
        ("webhook-id", id.into()),
        ("webhook-timestamp", timestamp),
        ("webhook-signature", signature),
    ])
}

/// Receivers must also enforce their timestamp tolerance and deduplicate webhook-id.
pub fn verify_standard_signature(
    secret: &str,
    id: &str,
    timestamp: &str,
    body: &[u8],
    signature: &str,
) -> bool {
    let Some(expected) = standard_signature(secret, id, timestamp, body) else {
        return false;
    };
    signature.split(' ').fold(false, |valid, value| {
        valid | bool::from(value.as_bytes().ct_eq(expected.as_bytes()))
    })
}

/// `valid_signature?(secret)`: the `X-Hub-Signature-256` header (missing is `""`) starts with
/// `sha256=` and equals `sha256=<hex HMAC-SHA256(secret, raw body)>` exactly (so the hex is
/// lowercase), compared in constant time.
pub fn verify_github_signature(secret: &str, raw_body: &[u8], signature: Option<&str>) -> bool {
    let signature = signature.unwrap_or("");
    let expected = format!(
        "{PREFIX}{}",
        hmac_sha256_hex(secret.as_bytes(), &[raw_body])
    );
    signature.starts_with(PREFIX) && bool::from(signature.as_bytes().ct_eq(expected.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_webhooks_reference_vector() {
        let secret = "whsec_MfKQ9r8GKYqrTwjUPD8ILPZIo2LaLaSw";
        let id = "msg_p5jXN8AQM9LWM0D4loKWxJek";
        let body = br#"{"test": 2432232314}"#;
        let signature = "v1,g0hM9SsE+OTPJTGt/tmIKtSyZlE3uFJELVlNIOLJ1OE=";
        assert_eq!(
            standard_signature(secret, id, "1614265330", body).as_deref(),
            Some(signature)
        );
        assert!(verify_standard_signature(
            secret,
            id,
            "1614265330",
            body,
            signature
        ));
        assert!(!verify_standard_signature(
            secret,
            id,
            "1614265331",
            body,
            signature
        ));
        assert!(!verify_standard_signature(
            secret,
            "other",
            "1614265330",
            body,
            signature
        ));
        assert!(!verify_standard_signature(
            secret,
            id,
            "1614265330",
            b"{}",
            signature
        ));
    }

    #[test]
    fn signatures_bind_the_secret_timestamp_and_body() {
        let signature = smartfire_signature("secret", "1767268800", b"{}");
        assert!(verify_smartfire_signature(
            "secret",
            "1767268800",
            b"{}",
            &signature
        ));
        assert!(!verify_smartfire_signature(
            "other",
            "1767268800",
            b"{}",
            &signature
        ));
        assert!(!verify_smartfire_signature(
            "secret",
            "1767268801",
            b"{}",
            &signature
        ));
        assert!(!verify_smartfire_signature(
            "secret",
            "1767268800",
            b"{ }",
            &signature
        ));
        assert!(!verify_smartfire_signature(
            "secret",
            "1767268800",
            b"{}",
            &signature.to_uppercase()
        ));
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
            assert!(
                !verify_github_signature("secret", body, Some(&invalid)),
                "{invalid:?}"
            );
        }
        assert!(!verify_github_signature("secret", body, None));
        assert!(!verify_github_signature("other", body, Some(&valid)));
        assert!(!verify_github_signature("secret", b"{}", Some(&valid)));
    }
}
