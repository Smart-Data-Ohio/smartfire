//! A 24-hour signed proof that the member received mail at a new address.
use jiff::{SignedDuration, Timestamp};
use serde_json::{Value, json};
use subtle::ConstantTimeEq;

use crate::Secrets;
use crate::password_reset::keyed_fingerprint;
use crate::signed_id::verifier;

const FINGERPRINT_SALT: &str = "smartfire/email_address_confirmation/password_fingerprint";

const PURPOSE: &str = "user/email_address_confirmation";
pub const EXPIRES_IN: SignedDuration = SignedDuration::from_hours(24);

#[derive(Debug, PartialEq, Eq)]
pub struct Claim {
    pub user_id: i64,
    pub email_address: String,
    pub issued_at: Timestamp,
    fingerprint: String,
}

impl Claim {
    pub fn matches_password(&self, secrets: &Secrets, password_digest: &str) -> bool {
        let current = keyed_fingerprint(secrets, FINGERPRINT_SALT, &[password_digest]);
        bool::from(current.as_bytes().ct_eq(self.fingerprint.as_bytes()))
    }
}

pub fn token(
    secrets: &Secrets,
    user_id: i64,
    email_address: &str,
    password_digest: &str,
    issued_at: Timestamp,
) -> String {
    let fingerprint = keyed_fingerprint(secrets, FINGERPRINT_SALT, &[password_digest]);
    let expires_at = issued_at + EXPIRES_IN;
    verifier(secrets).generate(
        &json!([user_id, email_address, fingerprint, expires_at.to_string()]),
        Some(PURPOSE),
        None,
    )
}

pub fn verify(secrets: &Secrets, token: &str, now: Timestamp) -> Option<Claim> {
    let claim = verifier(secrets).verify(token, Some(PURPOSE), now).ok()?;
    let [
        Value::Number(id),
        Value::String(address),
        Value::String(fingerprint),
        Value::String(expiry),
    ] = claim.as_array()?.as_slice()
    else {
        return None;
    };
    let expires_at: Timestamp = expiry.parse().ok()?;
    if now >= expires_at {
        return None;
    }
    Some(Claim {
        user_id: id.as_i64()?,
        email_address: address.clone(),
        issued_at: expires_at.checked_sub(EXPIRES_IN).ok()?,
        fingerprint: fingerprint.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::password_reset;
    use crate::signed_id::{avatar_token, transfer_id, verify_avatar_token, verify_transfer_id};

    #[test]
    fn confirmation_proves_the_address_until_expiry_and_has_its_own_purpose() {
        let now = "2026-01-01T12:00:00Z".parse::<Timestamp>().unwrap();
        let secrets = Secrets::new(&"a".repeat(128));
        let address = "other@example.test";
        let link = token(&secrets, 42, address, "", now);
        let claim = verify(
            &secrets,
            &link,
            now + EXPIRES_IN - SignedDuration::from_secs(1),
        )
        .expect("confirmation before expiry");
        assert_eq!(claim.user_id, 42);
        assert_eq!(claim.email_address, address);
        assert_eq!(claim.issued_at, now);
        assert!(claim.matches_password(&secrets, ""));
        assert!(!claim.matches_password(&secrets, "new digest"));
        assert_eq!(verify(&secrets, &link, now + EXPIRES_IN), None);
        assert_eq!(verify(&secrets, &format!("{link}0"), now), None);
        assert_eq!(verify_transfer_id(&secrets, &link, now), None);
        assert_eq!(verify_avatar_token(&secrets, &link, now), None);
        assert_eq!(password_reset::verify(&secrets, &link, now), None);
        for other in [
            transfer_id(&secrets, 42, now),
            avatar_token(&secrets, 42),
            password_reset::token(&secrets, 42, "digest", address, now),
        ] {
            assert_eq!(verify(&secrets, &other, now), None);
        }
    }
}
