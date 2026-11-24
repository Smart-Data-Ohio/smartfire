//! A 24-hour signed proof that the member received mail at a new address.
use jiff::{SignedDuration, Timestamp};
use serde_json::{Value, json};

use crate::Secrets;
use crate::signed_id::verifier;

const PURPOSE: &str = "user/email_address_confirmation";
pub const EXPIRES_IN: SignedDuration = SignedDuration::from_hours(24);

#[derive(Debug, PartialEq, Eq)]
pub struct Claim {
    pub user_id: i64,
    pub email_address: String,
}

pub fn token(secrets: &Secrets, user_id: i64, email_address: &str, now: Timestamp) -> String {
    verifier(secrets).generate(
        &json!([user_id, email_address]),
        Some(PURPOSE),
        Some(now + EXPIRES_IN),
    )
}

pub fn verify(secrets: &Secrets, token: &str, now: Timestamp) -> Option<Claim> {
    let claim = verifier(secrets).verify(token, Some(PURPOSE), now).ok()?;
    let [Value::Number(id), Value::String(address)] = claim.as_array()?.as_slice() else {
        return None;
    };
    Some(Claim {
        user_id: id.as_i64()?,
        email_address: address.clone(),
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
        let link = token(&secrets, 42, address, now);
        assert_eq!(
            verify(
                &secrets,
                &link,
                now + EXPIRES_IN - SignedDuration::from_secs(1)
            ),
            Some(Claim {
                user_id: 42,
                email_address: address.into()
            }),
        );
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
