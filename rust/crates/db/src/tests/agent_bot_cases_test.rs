//! Named User::BotTest digest/authentication comparisons. Fixed entropy,
//! reply-token authorization and executing the webhook remain distinct cases.
use super::*;
use crate::{
    User,
    user::{BOT_KEY_PLACEHOLDER, digest_bot_token},
};
fn setup() -> TestDb {
    super::channel_thread_test::frozen()
}
fn bot(t: &TestDb) -> User {
    t.write(|tx| User::create_bot(tx, "Bender", None))
}
fn auth(t: &TestDb, key: &str) -> Option<i64> {
    t.read(|c| User::authenticate_bot(c, key)).map(|u| u.id)
}
fn plaintext(c: &Connection, id: i64) -> Result<Option<String>> {
    Ok(c.query_row("SELECT bot_token FROM users WHERE id=?", [id], |r| r.get(0))?)
}
#[test]
fn ws11_bot_case_authenticate_created_bot() {
    let t = setup();
    let b = bot(&t);
    assert_eq!(auth(&t, &b.bot_key()), Some(b.id));
}
#[test]
fn ws11_bot_case_digest_alone_and_stored_key_hidden() {
    let t = setup();
    let b = bot(&t);
    let key = b.plain_bot_key().unwrap();
    let token = b.plain_bot_token.unwrap();
    let stored = t.read(|c| User::find(c, b.id));
    assert_eq!(stored.bot_token_digest, Some(digest_bot_token(&token)));
    assert!(t.read(|c| plaintext(c, b.id)).is_none());
    assert!(stored.plain_bot_key().is_none());
    assert_eq!(stored.bot_key(), BOT_KEY_PLACEHOLDER);
    assert_eq!(auth(&t, &key), Some(b.id));
}
#[test]
fn ws11_bot_case_leftover_plaintext_never_consulted() {
    let t = setup();
    let b = bot(&t);
    let key = b.plain_bot_key().unwrap();
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE users SET bot_token='DecoyToken12' WHERE id=?",
            [b.id],
        )?;
        Ok(())
    });
    assert_eq!(auth(&t, &key), Some(b.id));
    assert!(auth(&t, &format!("{}-DecoyToken12", b.id)).is_none());
    assert_eq!(
        t.read(|c| plaintext(c, b.id)).as_deref(),
        Some("DecoyToken12")
    );
}
#[test]
fn ws11_bot_case_missing_digest_requires_reset() {
    let t = setup();
    let b = bot(&t);
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE users SET bot_token='OldRelease12',bot_token_digest=NULL WHERE id=?",
            [b.id],
        )?;
        Ok(())
    });
    for token in ["OldRelease12", "WrongToken12"] {
        assert!(auth(&t, &format!("{}-{token}", b.id)).is_none());
    }
    let new = t.write(move |tx| User::find(tx.conn(), b.id)?.reset_bot_key(tx));
    assert_eq!(auth(&t, &new), Some(b.id));
}
#[test]
fn ws11_bot_case_reset_clears_plaintext_retires_old_key() {
    let t = setup();
    let b = bot(&t);
    let old = b.plain_bot_key().unwrap();
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE users SET bot_token='PreRetire123' WHERE id=?",
            [b.id],
        )?;
        Ok(())
    });
    let new = t.write(move |tx| User::find(tx.conn(), b.id)?.reset_bot_key(tx));
    let stored = t.read(|c| User::find(c, b.id));
    assert!(t.read(|c| plaintext(c, b.id)).is_none());
    assert_eq!(
        stored.bot_token_digest,
        Some(digest_bot_token(new.split_once('-').unwrap().1))
    );
    assert!(auth(&t, &old).is_none());
    assert_eq!(auth(&t, &new), Some(b.id));
}
#[test]
fn ws11_bot_case_wrong_empty_and_malformed_keys_refused() {
    let t = setup();
    let b = bot(&t);
    let token = b.plain_bot_token.unwrap();
    for key in [
        format!("{}-{token}x", b.id),
        format!("{}-", b.id),
        b.id.to_string(),
        String::new(),
        BOT_KEY_PLACEHOLDER.into(),
        format!("{}-{token}", id("bender")),
        format!("{}-{token}", id("david")),
    ] {
        assert!(
            auth(&t, &key).is_none(),
            "malformed or unrelated bot key accepted"
        );
    }
}
#[test]
fn ws11_bot_case_deactivated_key_refused() {
    let t = setup();
    let b = bot(&t);
    let key = b.bot_key();
    t.write(move |tx| User::find(tx.conn(), b.id)?.deactivate(tx));
    assert!(auth(&t, &key).is_none());
}
#[test]
fn ws11_bot_case_pre_migration_fixture_digest_keeps_working() {
    let t = setup();
    assert_eq!(
        auth(&t, &format!("{}-BenderToken1", id("bender"))),
        Some(id("bender"))
    );
    t.read(|c| {
        assert_eq!(
            User::find(c, id("bender"))?.bot_token_digest.as_deref(),
            Some(BENDER_TOKEN_DIGEST)
        );
        Ok(())
    });
}

#[test]
fn ws11_webhook_case_signing_secret_generation_adopts_winner() {
    let t = TestDb::new();
    let crypto = rails_compat::ar_encryption::ArEncryption::new(&rails_compat::Secrets::new(
        &"ws11 public test material ".repeat(8),
    ));
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE webhooks SET signing_secret=NULL WHERE user_id=?",
            [id("bender")],
        )?;
        let mut stale = crate::Webhook::find_by_user(tx.conn(), id("bender"))?.unwrap();
        assert!(stale.signing_secret(&crypto)?.is_none());
        let winner = crate::Webhook::find_by_user(tx.conn(), id("bender"))?
            .unwrap()
            .ensure_signing_secret(tx, &crypto)?;
        assert_eq!(stale.ensure_signing_secret(tx, &crypto)?, winner);
        assert_eq!(
            crate::Webhook::find_by_user(tx.conn(), id("bender"))?
                .unwrap()
                .signing_secret(&crypto)?,
            Some(winner)
        );
        Ok(())
    });
}
