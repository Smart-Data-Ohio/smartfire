//! The encrypted token snapshot `Calendar::DisconnectCleanupJob` carries
//! (`app/jobs/calendar/disconnect_cleanup_job.rb`, written by `GoogleAccount#cleanup_snapshot`).
//!
//! `ActiveSupport::MessageEncryptor.new(key_generator.generate_key("calendar/disconnect-cleanup",
//! 32))` with the app's default serializer (`:json_allow_marshal`), and
//! `encrypt_and_sign(snapshot, expires_in: 1.day, purpose: "calendar/disconnect-cleanup")`. The
//! snapshot is `{access_token:, refresh_token:, access_token_expires_at:}`, dumped by
//! `ActiveSupport::JSON` (a time is ISO 8601 with milliseconds).
use jiff::{SignedDuration, Timestamp};
use serde_json::{Map, Value};

use crate::metadata::{Serializer, iso8601_millis};
use crate::{MessageEncryptor, Secrets};

pub const PURPOSE: &str = "calendar/disconnect-cleanup";
/// `CREDENTIALS_EXPIRES_IN`.
pub const EXPIRES_IN: SignedDuration = SignedDuration::from_hours(24);

/// `GoogleAccount#cleanup_snapshot`'s hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub access_token_expires_at: Option<Timestamp>,
}

/// `DisconnectCleanupJob.credentials_encryptor`.
pub fn encryptor(secrets: &Secrets) -> MessageEncryptor {
    MessageEncryptor::new(&secrets.key_generator.generate_key(PURPOSE, 32), Serializer::JsonWithFallback { allow_marshal: true })
}

/// `DisconnectCleanupJob.encrypt_credentials(snapshot)` at `now`.
pub fn encrypt(secrets: &Secrets, snapshot: &Snapshot, now: Timestamp) -> String {
    let mut hash = Map::new();
    hash.insert("access_token".into(), snapshot.access_token.clone().into());
    hash.insert("refresh_token".into(), snapshot.refresh_token.clone().into());
    hash.insert("access_token_expires_at".into(), snapshot.access_token_expires_at.map(iso8601_millis).into());
    encryptor(secrets).encrypt_and_sign(&Value::Object(hash), Some(PURPOSE), Some(now + EXPIRES_IN))
}

/// `DisconnectCleanupJob.decrypt_credentials(blob)`: `None` when the blob expired, carries another
/// purpose, was tampered with, or was encrypted under another key. Otherwise the decrypted value,
/// which for anything the app wrote is the snapshot hash.
pub fn decrypt(secrets: &Secrets, blob: &str, now: Timestamp) -> Option<Value> {
    encryptor(secrets).decrypt_and_verify(blob, Some(PURPOSE), now).ok()
}

/// [`decrypt`], read back into a [`Snapshot`]. `None` also when the hash isn't snapshot-shaped.
pub fn decrypt_snapshot(secrets: &Secrets, blob: &str, now: Timestamp) -> Option<Snapshot> {
    let value = decrypt(secrets, blob, now)?;
    let hash = value.as_object()?;
    let string = |key: &str| match hash.get(key) {
        None | Some(Value::Null) => Some(None),
        Some(Value::String(s)) => Some(Some(s.clone())),
        Some(_) => None,
    };
    Some(Snapshot {
        access_token: string("access_token")?,
        refresh_token: string("refresh_token")?,
        access_token_expires_at: match string("access_token_expires_at")? {
            None => None,
            Some(time) => Some(time.parse().ok()?),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secrets() -> Secrets {
        Secrets::new(&"a".repeat(128))
    }

    fn snapshot() -> Snapshot {
        Snapshot { access_token: Some("FAKE-access-token".into()), refresh_token: None, access_token_expires_at: Some("2026-01-01T13:00:00.123Z".parse().unwrap()) }
    }

    #[test]
    fn round_trips_for_a_day_only() {
        let (secrets, now): (_, Timestamp) = (secrets(), "2026-01-01T12:00:00Z".parse().unwrap());
        let blob = encrypt(&secrets, &snapshot(), now);
        assert_eq!(decrypt_snapshot(&secrets, &blob, now), Some(snapshot()));
        assert_eq!(decrypt_snapshot(&secrets, &blob, now + EXPIRES_IN - SignedDuration::from_secs(1)), Some(snapshot()));
        assert_eq!(decrypt(&secrets, &blob, now + EXPIRES_IN), None);
    }

    #[test]
    fn refuses_other_keys_purposes_and_tampering() {
        let (secrets, now): (_, Timestamp) = (secrets(), "2026-01-01T12:00:00Z".parse().unwrap());
        let blob = encrypt(&secrets, &snapshot(), now);
        assert_eq!(decrypt(&Secrets::new(&"b".repeat(128)), &blob, now), None);

        let other_purpose = encryptor(&secrets).encrypt_and_sign(&Value::from("x"), Some("calendar/other"), None);
        assert_eq!(decrypt(&secrets, &other_purpose, now), None);
        let no_purpose = encryptor(&secrets).encrypt_and_sign(&Value::from("x"), None, None);
        assert_eq!(decrypt(&secrets, &no_purpose, now), None);

        for index in 0..blob.len() {
            let mut tampered = blob.clone().into_bytes();
            tampered[index] = if tampered[index] == b'A' { b'B' } else { b'A' };
            let tampered = String::from_utf8(tampered).unwrap();
            if tampered != blob {
                assert_eq!(decrypt(&secrets, &tampered, now), None, "byte {index}");
            }
        }
    }
}
