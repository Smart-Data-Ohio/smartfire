//! Named case ports of pinned test/models/agent_credential_test.rb.
use super::*;
use crate::{Agent, AgentCredential, CredentialChanges, Error, NewCredential};
use jiff::SignedDuration;

fn existing(t: &TestDb) -> AgentCredential {
    t.read(|conn| Ok(AgentCredential::find(conn, id("bender_main"))?.unwrap()))
}

#[test]
fn ws11_credential_case_reveal_once_digest_and_display() {
    let t = super::channel_thread_test::frozen();
    t.write(|tx| {
        let (credential, secret) = AgentCredential::create_with_secret(
            tx,
            id("bender_agent"),
            "CI runner",
            id("david"),
            None,
        )?;
        assert_eq!(secret.len(), 64);
        assert!(secret.bytes().all(|b| b.is_ascii_hexdigit()));
        let digest = crate::user::digest_bot_token(&secret);
        assert_eq!(credential.token_digest, digest);
        assert_eq!(credential.token_last_four, &digest[..4]);
        assert!(credential.active(tx.now()));
        // Reloading reveals no plaintext field; only the digest remains in SQLite.
        assert_eq!(
            AgentCredential::find(tx.conn(), credential.id)?
                .unwrap()
                .token_digest,
            digest
        );
        Ok(())
    });
}
#[test]
fn ws11_credential_case_name_required() {
    let t = super::channel_thread_test::frozen();
    t.write(|tx| {
        let error = AgentCredential::create_with_secret(tx, id("bender_agent"), "", id("david"), None).unwrap_err();
        assert!(matches!(error, Error::RecordInvalid(ref errors) if errors.0.iter().any(|(field,message)| *field=="name" && message=="can't be blank")));
        Ok(())
    });
}
#[test]
fn ws11_credential_case_digest_unique() {
    let t = super::channel_thread_test::frozen();
    let original = existing(&t);
    t.read(|conn| {
        let errors = AgentCredential::validate(
            conn,
            &NewCredential {
                agent_id: id("bender_agent"),
                created_by_id: id("david"),
                name: "Duplicate".into(),
                token_digest: original.token_digest,
                token_last_four: "1234".into(),
                ..Default::default()
            },
            None,
        )?;
        assert!(errors.0.iter().any(
            |(field, message)| *field == "token_digest" && message == "has already been taken"
        ));
        Ok(())
    });
}
#[test]
fn ws11_credential_case_authentication_strips_whitespace() {
    let t = super::channel_thread_test::frozen();
    t.read(|conn| {
        assert_eq!(
            AgentCredential::authenticate(conn, "  bender-test-secret-1234  ", t.now())?
                .unwrap()
                .id,
            id("bender_main")
        );
        Ok(())
    });
}
#[test]
fn ws11_credential_case_unknown_and_blank_secrets() {
    let t = super::channel_thread_test::frozen();
    t.read(|conn| {
        for secret in ["no-such-secret", "", " \t\n"] {
            assert!(AgentCredential::authenticate(conn, secret, t.now())?.is_none());
        }
        // Rails' nil.to_s is supplied as an empty string at the caller boundary.
        Ok(())
    });
}
#[test]
fn ws11_credential_case_revoked_cannot_authenticate() {
    let t = super::channel_thread_test::frozen();
    let mut credential = existing(&t);
    t.write(move |tx| {
        credential.revoke(tx)?;
        assert!(credential.revoked_at.is_some());
        assert!(!credential.active(tx.now()));
        assert!(
            AgentCredential::authenticate(tx.conn(), "bender-test-secret-1234", tx.now())?
                .is_none()
        );
        Ok(())
    });
}
#[test]
fn ws11_credential_case_expired_cannot_authenticate() {
    let t = super::channel_thread_test::frozen();
    let mut credential = existing(&t);
    t.write(move |tx| {
        credential.update(
            tx,
            CredentialChanges {
                expires_at: Some(Some(tx.now().ago(SignedDuration::from_mins(1)))),
                ..Default::default()
            },
        )?;
        assert!(!credential.active(tx.now()));
        assert!(
            AgentCredential::authenticate(tx.conn(), "bender-test-secret-1234", tx.now())?
                .is_none()
        );
        Ok(())
    });
}
#[test]
fn ws11_credential_case_future_expiry_stays_active() {
    let t = super::channel_thread_test::frozen();
    let mut credential = existing(&t);
    t.write(move |tx| {
        credential.update(
            tx,
            CredentialChanges {
                expires_at: Some(Some(tx.now().since(SignedDuration::from_hours(24)))),
                ..Default::default()
            },
        )?;
        assert!(credential.active(tx.now()));
        assert_eq!(
            AgentCredential::authenticate(tx.conn(), "bender-test-secret-1234", tx.now())?
                .unwrap()
                .id,
            credential.id
        );
        Ok(())
    });
}
#[test]
fn ws11_credential_case_use_stamps_time_and_ip() {
    let t = super::channel_thread_test::frozen();
    let mut credential = existing(&t);
    assert!(credential.last_used_at.is_none());
    t.write(move |tx| {
        credential.record_use(tx, Some("203.0.113.7"))?;
        let persisted = AgentCredential::find(tx.conn(), credential.id)?.unwrap();
        assert_eq!(persisted.last_used_at, Some(tx.now()));
        assert_eq!(persisted.last_used_ip.as_deref(), Some("203.0.113.7"));
        Ok(())
    });
}
#[test]
fn ws11_credential_case_use_throttle_preserves_first_ip_until_next_minute() {
    let t = super::channel_thread_test::frozen();
    let mut credential = existing(&t);
    let first = t.write(move |tx| {
        credential.record_use(tx, Some("203.0.113.7"))?;
        let first = credential.last_used_at.unwrap();
        credential.record_use(tx, Some("198.51.100.9"))?;
        let persisted = AgentCredential::find(tx.conn(), credential.id)?.unwrap();
        assert_eq!(persisted.last_used_at, Some(first));
        assert_eq!(persisted.last_used_ip.as_deref(), Some("203.0.113.7"));
        tx.conn().execute(
            "UPDATE agent_credentials SET last_used_at=? WHERE id=?",
            rusqlite::params![tx.now().ago(SignedDuration::from_secs(61)), credential.id],
        )?;
        Ok(first)
    });
    t.clock.travel(SignedDuration::from_secs(1));
    let mut credential = existing(&t);
    t.write(move |tx| {
        credential.record_use(tx, Some("198.51.100.9"))?;
        let persisted = AgentCredential::find(tx.conn(), credential.id)?.unwrap();
        assert!(persisted.last_used_at.unwrap() > first);
        assert_eq!(persisted.last_used_ip.as_deref(), Some("198.51.100.9"));
        Ok(())
    });
}
#[test]
fn ws11_credential_case_destroy_agent_removes_credentials() {
    let t = super::channel_thread_test::frozen();
    t.write(|tx| {
        Agent::find(tx.conn(), id("bender_agent"))?
            .unwrap()
            .destroy(tx)?;
        assert!(AgentCredential::find(tx.conn(), id("bender_main"))?.is_none());
        Ok(())
    });
}
