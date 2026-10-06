//! `ActiveRecord::SignedId` (`signed_id(purpose:, expires_in:)` / `find_signed`).
//!
//! The verifier is `Rails.application.message_verifiers["active_record/signed_id"]` with the
//! legacy options prepended (`use_legacy_signed_id_verifier` defaults to `:generate_and_verify`):
//! it generates with SHA256, `::JSON`, URL-safe Base64, and falls back when reading to the
//! app-wide default (SHA1, `:json_allow_marshal`, strict Base64). The purpose is
//! `"<base class name underscored>/<purpose>"`, e.g. `user/avatar`, or just `user`.
use jiff::{SignedDuration, Timestamp};
use serde_json::Value;

use crate::Secrets;
use crate::message_verifier::{Digest, Encoding, MessageVerifier, Serializer};

pub const SALT: &str = "active_record/signed_id";

/// `model_name` is the record's *base* class name, e.g. "User" or "Room" (not "Rooms::Open").
pub fn generate(secrets: &Secrets, model_name: &str, id: i64, purpose: Option<&str>, expires_at: Option<Timestamp>) -> String {
    verifier(secrets).generate(&Value::from(id), Some(&combine_purposes(model_name, purpose)), expires_at)
}

/// `find_signed`'s verification step: the id to look up, or `None`.
pub fn verify(secrets: &Secrets, model_name: &str, signed_id: &str, purpose: Option<&str>, now: Timestamp) -> Option<i64> {
    match verifier(secrets).verify(signed_id, Some(&combine_purposes(model_name, purpose)), now).ok()? {
        Value::Number(n) => n.as_i64(),
        // `find_by(id: "7")` casts the string.
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

/// `User::Transferable::TRANSFER_LINK_EXPIRY_DURATION`.
pub const TRANSFER_EXPIRES_IN: SignedDuration = SignedDuration::from_hours(4);

/// `User#transfer_id` (`app/models/user/transferable.rb`):
/// `signed_id(purpose: :transfer, expires_in: 4.hours)`.
pub fn transfer_id(secrets: &Secrets, user_id: i64, now: Timestamp) -> String {
    generate(secrets, "User", user_id, Some("transfer"), Some(now + TRANSFER_EXPIRES_IN))
}

/// `User.find_by_transfer_id(id)`'s verification step: `find_signed(id, purpose: :transfer)`.
pub fn verify_transfer_id(secrets: &Secrets, signed_id: &str, now: Timestamp) -> Option<i64> {
    verify(secrets, "User", signed_id, Some("transfer"), now)
}

/// `User#avatar_token` (`app/models/user/avatar.rb`): `signed_id(purpose: :avatar)`, no expiry.
pub fn avatar_token(secrets: &Secrets, user_id: i64) -> String {
    generate(secrets, "User", user_id, Some("avatar"), None)
}

/// `User.from_avatar_token(sid)`'s verification step (`find_signed!`, which raises where this
/// returns `None`).
pub fn verify_avatar_token(secrets: &Secrets, signed_id: &str, now: Timestamp) -> Option<i64> {
    verify(secrets, "User", signed_id, Some("avatar"), now)
}

pub fn verifier(secrets: &Secrets) -> MessageVerifier {
    let secret = secrets.key_generator.generate_key(SALT, 64);
    let fallback = MessageVerifier::new(secret.clone(), Digest::Sha1, Encoding::Strict, Serializer::JsonWithFallback { allow_marshal: true });
    MessageVerifier::new(secret, Digest::Sha256, Encoding::UrlSafe, Serializer::Json).fall_back_to(fallback)
}

/// `combine_signed_id_purposes`: `[base_class.name.underscore, purpose.to_s].compact_blank.join("/")`.
pub fn combine_purposes(model_name: &str, purpose: Option<&str>) -> String {
    [underscore(model_name), purpose.unwrap_or("").to_string()]
        .into_iter()
        .filter(|part| !part.trim().is_empty())
        .collect::<Vec<_>>()
        .join("/")
}

/// `String#underscore` for class names: `Rooms::Open` → `rooms/open`, `WebPush` → `web_push`.
fn underscore(name: &str) -> String {
    let name = name.replace("::", "/");
    let chars: Vec<char> = name.chars().collect();
    let mut out = String::new();
    for (i, &c) in chars.iter().enumerate() {
        if c.is_ascii_uppercase() {
            let previous = i.checked_sub(1).map(|j| chars[j]);
            let next = chars.get(i + 1);
            let after_lower_or_digit = previous.is_some_and(|p| p.is_ascii_lowercase() || p.is_ascii_digit());
            let acronym_end = previous.is_some_and(|p| p.is_ascii_uppercase()) && next.is_some_and(|n| n.is_ascii_lowercase());
            if after_lower_or_digit || acronym_end {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(if c == '-' { '_' } else { c });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combines_purposes() {
        assert_eq!(combine_purposes("User", Some("avatar")), "user/avatar");
        assert_eq!(combine_purposes("User", None), "user");
        assert_eq!(combine_purposes("Rooms::Open", Some("")), "rooms/open");
        assert_eq!(combine_purposes("HTTPRequest", Some("x")), "http_request/x");
    }

    #[test]
    fn transfer_ids_expire_and_purposes_do_not_cross() {
        let secrets = Secrets::new(&"a".repeat(128));
        let now: Timestamp = "2026-01-01T12:00:00Z".parse().unwrap();
        let transfer = transfer_id(&secrets, 7, now);
        let avatar = avatar_token(&secrets, 7);
        assert_eq!(verify_transfer_id(&secrets, &transfer, now + TRANSFER_EXPIRES_IN - SignedDuration::from_secs(1)), Some(7));
        assert_eq!(verify_transfer_id(&secrets, &transfer, now + TRANSFER_EXPIRES_IN), None);
        assert_eq!(verify_avatar_token(&secrets, &avatar, now + SignedDuration::from_hours(24 * 365 * 50)), Some(7));
        assert_eq!(verify_transfer_id(&secrets, &avatar, now), None);
        assert_eq!(verify_avatar_token(&secrets, &transfer, now), None);
        let room_avatar = generate(&secrets, "Room", 7, Some("avatar"), None);
        assert_eq!(verify_avatar_token(&secrets, &room_avatar, now), None);
        assert_eq!(verify_avatar_token(&Secrets::new(&"b".repeat(128)), &avatar, now), None);
    }
}
