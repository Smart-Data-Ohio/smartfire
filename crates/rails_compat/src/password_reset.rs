//! Self-service password reset links, after Rails 8's `generates_token_for :password_reset,
//! expires_in: 15.minutes { password_salt.last(10) }`. The token is the user's id and a fingerprint
//! of their current password digest and email address, signed by the signed-id verifier
//! ([`crate::signed_id::verifier`]) for the purpose `user/password_reset` with an expiry. Nothing
//! is stored: a new password or email address changes the fingerprint, so the link stops working
//! once it has been used or the address it was sent to is replaced.
use hmac::{Hmac, Mac};
use jiff::{SignedDuration, Timestamp};
use serde_json::{Value, json};
use sha2::Sha256;
use subtle::ConstantTimeEq;

use crate::Secrets;
use crate::signed_id::{combine_purposes, verifier};

/// How long a reset link works: Rails 8's `password_reset` token default.
pub const EXPIRES_IN: SignedDuration = SignedDuration::from_mins(15);

/// The fingerprint key's own salt, so the fingerprint shares no key with anything else.
const FINGERPRINT_SALT: &str = "smartfire/password_reset/fingerprint";

/// Bytes of HMAC-SHA256 kept in the fingerprint.
const FINGERPRINT_BYTES: usize = 16;

/// What a verified link names: the user, and the fingerprint they had when it was sent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Claim {
    pub user_id: i64,
    fingerprint: String,
    token: String,
}

impl Claim {
    /// Whether the link is still unexpired at `now`, for a check made later than [`verify`]'s.
    pub fn unexpired(&self, secrets: &Secrets, now: Timestamp) -> bool {
        verify(secrets, &self.token, now).is_some()
    }

    /// Whether the user still has the password digest and email address the link was made for.
    pub fn matches(&self, secrets: &Secrets, password_digest: &str, email_address: &str) -> bool {
        let current = fingerprint(secrets, password_digest, email_address);
        bool::from(current.as_bytes().ct_eq(self.fingerprint.as_bytes()))
    }
}

/// A reset link's token for the user with this password digest and email address.
pub fn token(
    secrets: &Secrets,
    user_id: i64,
    password_digest: &str,
    email_address: &str,
    now: Timestamp,
) -> String {
    verifier(secrets).generate(
        &json!([user_id, fingerprint(secrets, password_digest, email_address)]),
        Some(&purpose()),
        Some(now + EXPIRES_IN),
    )
}

/// The link's claim while it is unexpired and was signed for a password reset, else `None`.
pub fn verify(secrets: &Secrets, token: &str, now: Timestamp) -> Option<Claim> {
    match verifier(secrets).verify(token, Some(&purpose()), now).ok()? {
        Value::Array(parts) => match parts.as_slice() {
            [Value::Number(id), Value::String(fingerprint)] => Some(Claim {
                user_id: id.as_i64()?,
                fingerprint: fingerprint.clone(),
                token: token.to_owned(),
            }),
            _ => None,
        },
        _ => None,
    }
}

fn purpose() -> String {
    combine_purposes("User", Some("password_reset"))
}

/// HMAC-SHA256 of the digest and address under a key of its own, truncated and hex-encoded: it
/// changes with either and reveals neither.
fn fingerprint(secrets: &Secrets, password_digest: &str, email_address: &str) -> String {
    let key = secrets.key_generator.generate_key(FINGERPRINT_SALT, 32);
    let mut mac = Hmac::<Sha256>::new_from_slice(&key).expect("HMAC accepts any key length");
    mac.update(password_digest.as_bytes());
    mac.update(b"\0");
    mac.update(email_address.as_bytes());
    hex::encode(&mac.finalize().into_bytes()[..FINGERPRINT_BYTES])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signed_id::{avatar_token, transfer_id, verify_avatar_token, verify_transfer_id};

    const DIGEST: &str = "$2a$04$abcdefghijklmnopqrstuuJ8lC2Z8ZZ0uS0nE7q7aQ3x0V8m1YxW2";

    fn secrets() -> Secrets {
        Secrets::new(&"a".repeat(128))
    }

    fn now() -> Timestamp {
        "2026-01-01T12:00:00Z".parse().unwrap()
    }

    #[test]
    fn round_trips_until_it_expires() {
        let secrets = secrets();
        let token = token(&secrets, 7, DIGEST, "ada@example.com", now());
        let claim = verify(&secrets, &token, now() + EXPIRES_IN - SignedDuration::from_secs(1))
            .expect("unexpired");
        assert_eq!(claim.user_id, 7);
        assert!(claim.matches(&secrets, DIGEST, "ada@example.com"));
        assert!(claim.unexpired(&secrets, now() + EXPIRES_IN - SignedDuration::from_secs(1)));
        assert!(!claim.unexpired(&secrets, now() + EXPIRES_IN));
        assert_eq!(verify(&secrets, &token, now() + EXPIRES_IN), None);
        assert_eq!(verify(&Secrets::new(&"b".repeat(128)), &token, now()), None);
        assert_eq!(verify(&secrets, "garbage", now()), None);
    }

    #[test]
    fn a_new_password_or_email_address_retires_the_link() {
        let secrets = secrets();
        let claim = verify(
            &secrets,
            &token(&secrets, 7, DIGEST, "ada@example.com", now()),
            now(),
        )
        .unwrap();
        assert!(!claim.matches(&secrets, "$2a$04$another-digest", "ada@example.com"));
        assert!(!claim.matches(&secrets, DIGEST, "ada@example.org"));
        // The separator keeps a digest's tail from passing for the address's head.
        assert!(!claim.matches(&secrets, &format!("{DIGEST}a"), "da@example.com"));
    }

    #[test]
    fn the_token_does_not_reveal_the_digest() {
        let secrets = secrets();
        let token = token(&secrets, 7, DIGEST, "ada@example.com", now());
        let payload = token.split("--").next().unwrap();
        let decoded = String::from_utf8(
            base64::Engine::decode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, payload).unwrap(),
        )
        .unwrap();
        assert!(!decoded.contains(DIGEST), "{decoded}");
        assert!(!decoded.contains("ada@example.com"), "{decoded}");
        assert!(decoded.contains("user/password_reset"), "{decoded}");
    }

    #[test]
    fn purposes_do_not_cross() {
        let secrets = secrets();
        let reset = token(&secrets, 7, DIGEST, "ada@example.com", now());
        assert_eq!(verify_transfer_id(&secrets, &reset, now()), None);
        assert_eq!(verify_avatar_token(&secrets, &reset, now()), None);
        assert_eq!(verify(&secrets, &transfer_id(&secrets, 7, now()), now()), None);
        assert_eq!(verify(&secrets, &avatar_token(&secrets, 7), now()), None);
        let other_model = verifier(&secrets).generate(
            &json!([7, "fingerprint"]),
            Some(&combine_purposes("Room", Some("password_reset"))),
            None,
        );
        assert_eq!(verify(&secrets, &other_model, now()), None);
    }
}
