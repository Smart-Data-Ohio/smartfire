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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfirmationCursor {
    pub audit_id: i64,
    pub deactivation_id: i64,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Claim {
    pub user_id: i64,
    pub email_address: String,
    pub issued_at: Timestamp,
    pub cursor: ConfirmationCursor,
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
    cursor: ConfirmationCursor,
) -> String {
    let fingerprint = keyed_fingerprint(secrets, FINGERPRINT_SALT, &[password_digest]);
    let expires_at = issued_at + EXPIRES_IN;
    verifier(secrets).generate(
        &json!([
            user_id,
            email_address,
            fingerprint,
            expires_at.to_string(),
            cursor.audit_id,
            cursor.deactivation_id
        ]),
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
        Value::Number(audit_id),
        trailing @ ..,
    ] = claim.as_array()?.as_slice()
    else {
        return None;
    };
    let deactivation_id = match trailing {
        [] => 0,
        [Value::Number(id)] => id.as_i64()?,
        _ => return None,
    };
    let expires_at: Timestamp = expiry.parse().ok()?;
    if now >= expires_at {
        return None;
    }
    Some(Claim {
        user_id: id.as_i64()?,
        email_address: address.clone(),
        issued_at: expires_at.checked_sub(EXPIRES_IN).ok()?,
        cursor: ConfirmationCursor {
            audit_id: audit_id.as_i64()?,
            deactivation_id,
        },
        fingerprint: fingerprint.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::password_reset;
    use crate::signed_id::{avatar_token, transfer_id, verify_avatar_token, verify_transfer_id};

    #[test]
    fn confirmation_without_an_audit_id_is_refused() {
        let now = "2026-01-01T12:00:00Z".parse::<Timestamp>().unwrap();
        let secrets = Secrets::new(&"a".repeat(128));
        let fingerprint = keyed_fingerprint(&secrets, FINGERPRINT_SALT, &[""]);
        let old_link = verifier(&secrets).generate(
            &json!([
                42,
                "other@example.test",
                fingerprint,
                (now + EXPIRES_IN).to_string()
            ]),
            Some(PURPOSE),
            None,
        );
        assert_eq!(verify(&secrets, &old_link, now), None);
    }

    #[test]
    fn confirmation_with_only_an_audit_id_predates_deactivation_records() {
        let now = "2026-01-01T12:00:00Z".parse::<Timestamp>().unwrap();
        let secrets = Secrets::new(&"a".repeat(128));
        let fingerprint = keyed_fingerprint(&secrets, FINGERPRINT_SALT, &[""]);
        let link = verifier(&secrets).generate(
            &json!([
                42,
                "other@example.test",
                fingerprint,
                (now + EXPIRES_IN).to_string(),
                123
            ]),
            Some(PURPOSE),
            None,
        );
        let claim = verify(&secrets, &link, now).unwrap();
        assert_eq!(
            claim.cursor,
            ConfirmationCursor {
                audit_id: 123,
                deactivation_id: 0
            }
        );
    }

    #[test]
    fn confirmation_proves_the_address_until_expiry_and_has_its_own_purpose() {
        let now = "2026-01-01T12:00:00Z".parse::<Timestamp>().unwrap();
        let secrets = Secrets::new(&"a".repeat(128));
        let address = "other@example.test";
        let cursor = ConfirmationCursor {
            audit_id: 123,
            deactivation_id: 456,
        };
        let link = token(&secrets, 42, address, "", now, cursor);
        let claim = verify(
            &secrets,
            &link,
            now + EXPIRES_IN - SignedDuration::from_secs(1),
        )
        .expect("confirmation before expiry");
        assert_eq!(claim.user_id, 42);
        assert_eq!(claim.email_address, address);
        assert_eq!(claim.issued_at, now);
        assert_eq!(claim.cursor, cursor);
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
