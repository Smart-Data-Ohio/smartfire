//! Ports of the four Rails `test/models/two_factor_*_test.rb` files, plus real concurrency,
//! transactional failure injection and vectors from the pinned reference.

use rails_compat::{Secrets, ar_encryption::ArEncryption, totp};
use serde_json::Value;
use std::sync::{Barrier, OnceLock};

use super::*;
use crate::models::two_factor::{REMEMBER_FOR, SETUP_TTL};
use crate::{
    ChallengeFailure, Error, Timestamp, TwoFactorBackupCode as Backup,
    TwoFactorCredential as Credential, TwoFactorRememberedDevice as Remembered,
    TwoFactorSetupSecret as Setup, User,
};

const SECRET: &str = "JBSWY3DPEHPK3PXPJBSWY3DPEHPK3PXP";

fn vectors() -> Value {
    serde_json::from_str(include_str!("../../../../vectors/two_factor.json")).unwrap()
}

fn encryption() -> &'static ArEncryption {
    static ENCRYPTION: OnceLock<ArEncryption> = OnceLock::new();
    ENCRYPTION.get_or_init(|| {
        ArEncryption::new(&Secrets::new(
            vectors()["secret_key_base"].as_str().unwrap(),
        ))
    })
}

fn db() -> TestDb {
    TestDb::with_clock(
        TestClock::frozen_at(Timestamp::from_second(vectors()["now"].as_i64().unwrap())),
        4,
    )
}

fn credential(t: &TestDb, user: &str) -> Credential {
    let user_id = id(user);
    t.write(move |tx| Credential::create(tx, encryption(), user_id, SECRET))
}

fn setup(t: &TestDb, session: &str) -> Setup {
    let session_id = id(session);
    // A six-digit value can collide across steps or across secrets. Keep randomized fixtures
    // away from those collisions; the committed ROTP collision vector tests their semantics.
    loop {
        let setup = t.write(move |tx| Setup::issue_for(tx, encryption(), session_id));
        let secret = setup.secret(encryption()).unwrap();
        let code = code_for(&secret, t);
        let now = t.now().as_second();
        if code != code_for(SECRET, t)
            && code != totp::at(&secret, now + 30).unwrap()
            && code != totp::at(&secret, now - 30).unwrap()
        {
            return setup;
        }
    }
}

fn code_for(secret: &str, t: &TestDb) -> String {
    totp::at(secret, t.now().as_second()).unwrap()
}

fn consume(t: &TestDb, credential_id: i64, code: &str) -> bool {
    let code = code.to_string();
    t.write(move |tx| Backup::consume(tx, credential_id, &code))
}

#[test]
fn confirmation_adopts_the_session_secret_spends_it_and_stamps_the_step() {
    let t = db();
    let mut credential = credential(&t, "david");
    assert!(!credential.enabled());
    let pending = setup(&t, "david_safari");
    let secret = pending.secret(encryption()).unwrap();
    let code = code_for(&secret, &t);
    let credential = t.write(move |tx| {
        assert!(credential.confirm_with_setup_secret(tx, encryption(), &pending, &code)?);
        assert!(!credential.verify_code(tx, encryption(), &code)?);
        Ok(credential)
    });
    assert!(credential.enabled());
    assert_eq!(credential.secret(encryption()).unwrap(), secret);
    assert_eq!(credential.confirmed_at, Some(t.now()));
    assert_eq!(credential.last_totp_at, Some(t.now().as_second()));
    assert!(
        t.read(|c| Setup::valid_for(c, id("david_safari"), t.now()))
            .is_none()
    );
}

#[test]
fn confirmation_rejects_wrong_and_stored_secret_codes_without_spending_setup() {
    let t = db();
    let credential = credential(&t, "david");
    let pending = setup(&t, "david_safari");
    let stored_code = code_for(SECRET, &t);
    let credential = t.write(move |tx| {
        let mut credential = credential;
        assert!(!credential.confirm_with_setup_secret(tx, encryption(), &pending, "wrong")?);
        assert!(!credential.confirm_with_setup_secret(tx, encryption(), &pending, &stored_code)?);
        Ok(credential)
    });
    assert!(!credential.enabled());
    assert_eq!(credential.secret(encryption()).unwrap(), SECRET);
    assert!(
        t.read(|c| Setup::valid_for(c, id("david_safari"), t.now()))
            .is_some()
    );
}

#[test]
fn confirmation_accepts_spaced_codes() {
    let t = db();
    let mut credential = credential(&t, "david");
    let pending = setup(&t, "david_safari");
    let code = code_for(&pending.secret(encryption()).unwrap(), &t);
    let spaced = format!("{} {}", &code[..3], &code[3..]);
    assert!(t.write(move |tx| credential.confirm_with_setup_secret(
        tx,
        encryption(),
        &pending,
        &spaced
    )));
}

#[test]
fn credential_verification_accepts_adjacent_steps_rejects_far_steps_and_replays() {
    let t = db();
    let credential = credential(&t, "david");
    let at = t.now().as_second();
    let credential = t.write(move |tx| {
        let mut credential = credential;
        for offset in [-90, 90] {
            assert!(!credential.verify_code(
                tx,
                encryption(),
                &totp::at(SECRET, at + offset).unwrap()
            )?);
        }
        let behind = totp::at(SECRET, at - 30).unwrap();
        assert!(credential.verify_code(tx, encryption(), &behind)?);
        assert!(!credential.verify_code(tx, encryption(), &behind)?);
        assert!(credential.verify_code(tx, encryption(), &totp::at(SECRET, at + 30).unwrap())?);
        assert!(!credential.verify_code(tx, encryption(), &totp::at(SECRET, at).unwrap())?);
        Ok(credential)
    });
    assert_eq!(credential.last_totp_at, Some(at + 30));
}

#[test]
fn a_spent_step_stays_spent_during_the_next_step() {
    let t = db();
    let mut credential = credential(&t, "david");
    let code = code_for(SECRET, &t);
    let credential = t.write(move |tx| {
        assert!(credential.verify_code(tx, encryption(), &code)?);
        Ok(credential)
    });
    t.travel(45);
    let code = totp::at(SECRET, t.now().as_second() - 45).unwrap();
    assert!(!t.write(move |tx| {
        let mut credential = credential;
        credential.verify_code(tx, encryption(), &code)
    }));
}

#[test]
fn blank_codes_leave_the_credential_unchanged() {
    let t = db();
    let mut credential = credential(&t, "david");
    let at = credential.updated_at;
    t.travel(10);
    let credential = t.write(move |tx| {
        for code in ["", "   ", "\t\n\r"] {
            assert!(!credential.verify_code(tx, encryption(), code)?);
        }
        Ok(credential)
    });
    assert_eq!(credential.updated_at, at);
    assert!(credential.last_totp_at.is_none());
}

#[test]
fn lockouts_escalate_one_five_fifteen_and_fifteen_minutes() {
    let t = db();
    let mut credential = credential(&t, "david");
    for (index, seconds) in [60, 300, 900, 900].into_iter().enumerate() {
        let now = t.now();
        credential = t.write(move |tx| {
            for attempt in 0..5 {
                assert_eq!(
                    credential.register_challenge_failure(tx)?,
                    if attempt == 4 {
                        ChallengeFailure::Locked
                    } else {
                        ChallengeFailure::Failed
                    }
                );
            }
            Ok(credential)
        });
        assert_eq!(credential.lockout_count, index as i64 + 1);
        assert_eq!(credential.consecutive_failures, 0);
        assert_eq!(
            credential.locked_until,
            Some(now.since(jiff::SignedDuration::from_secs(seconds)))
        );
        assert!(credential.locked_out(now));
        t.travel(seconds);
        assert!(!credential.locked_out(t.now()));
    }
}

#[test]
fn failures_while_locked_change_nothing() {
    let t = db();
    let mut credential = credential(&t, "david");
    credential = t.write(move |tx| {
        for _ in 0..5 {
            credential.register_challenge_failure(tx)?;
        }
        Ok(credential)
    });
    let locked_until = credential.locked_until;
    let updated_at = credential.updated_at;
    t.travel(20);
    credential = t.write(move |tx| {
        assert_eq!(
            credential.register_challenge_failure(tx)?,
            ChallengeFailure::Failed
        );
        Ok(credential)
    });
    assert_eq!(credential.locked_until, locked_until);
    assert_eq!(credential.updated_at, updated_at);
    assert_eq!(
        (credential.consecutive_failures, credential.lockout_count),
        (0, 1)
    );
}

#[test]
fn success_resets_the_failure_run_and_escalation() {
    let t = db();
    let mut credential = credential(&t, "david");
    credential = t.write(move |tx| {
        for _ in 0..5 {
            credential.register_challenge_failure(tx)?;
        }
        Ok(credential)
    });
    t.travel(120);
    credential = t.write(move |tx| {
        for _ in 0..5 {
            credential.register_challenge_failure(tx)?;
        }
        assert_eq!(credential.lockout_count, 2);
        credential.register_challenge_success(tx)?;
        Ok(credential)
    });
    assert_eq!(
        (
            credential.consecutive_failures,
            credential.lockout_count,
            credential.locked_until
        ),
        (0, 0, None)
    );
}

#[test]
fn credentials_and_setup_secrets_decrypt_rails_rows_and_encrypt_rust_rows() {
    let t = db();
    let credential = credential(&t, "david");
    let pending = setup(&t, "david_safari");
    t.write(move |tx| {
        let v = vectors();
        tx.conn().execute(
            "UPDATE two_factor_credentials SET secret = ? WHERE id = ?",
            rusqlite::params![v["credential_ciphertext"].as_str().unwrap(), credential.id],
        )?;
        tx.conn().execute(
            "UPDATE two_factor_setup_secrets SET secret = ? WHERE id = ?",
            rusqlite::params![v["setup_ciphertext"].as_str().unwrap(), pending.id],
        )?;
        Ok(())
    });
    assert_eq!(
        t.read(|c| Credential::for_user(c, id("david")))
            .unwrap()
            .secret(encryption())
            .unwrap(),
        SECRET
    );
    assert_eq!(
        t.read(|c| Setup::valid_for(c, id("david_safari"), t.now()))
            .unwrap()
            .secret(encryption())
            .unwrap(),
        SECRET
    );
    let other = credential_for_jason(&t);
    let stored: String = t.read(|c| {
        Ok(c.query_row(
            "SELECT secret FROM two_factor_credentials WHERE id = ?",
            [other.id],
            |r| r.get(0),
        )?)
    });
    assert_ne!(stored, SECRET);
    assert!(!stored.contains(SECRET));
    assert_eq!(
        encryption().decrypt_bytes(&stored).unwrap().encoding,
        "ASCII-8BIT"
    );
    let mut corrupted: Value = serde_json::from_str(&stored).unwrap();
    corrupted["p"] = Value::String("AAAA".into());
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE two_factor_credentials SET secret = ? WHERE id = ?",
            rusqlite::params![corrupted.to_string(), other.id],
        )?;
        Ok(())
    });
    assert!(
        t.read(|c| Credential::for_user(c, id("jason")))
            .unwrap()
            .secret(encryption())
            .is_err()
    );
}

fn credential_for_jason(t: &TestDb) -> Credential {
    credential(t, "jason")
}

#[test]
fn unicode_base32_upcase_respects_the_rails_secret_encoding() {
    let t = db();
    let credential = credential(&t, "david");
    for case in vectors()["encoded_secrets"].as_array().unwrap() {
        let expected_error = !case["error"].is_null();
        let expected_verified = case["verified"].as_bool();
        let case = case.clone();
        let id = credential.id;
        let result = t.try_write(move |tx| {
            tx.conn().execute(
                "UPDATE two_factor_credentials SET secret = ?, last_totp_at = NULL WHERE id = ?",
                rusqlite::params![case["ciphertext"].as_str().unwrap(), id],
            )?;
            Credential::find(tx.conn(), id)?.verify_code(
                tx,
                encryption(),
                case["code"].as_str().unwrap(),
            )
        });
        if expected_error {
            assert!(result.is_err());
        } else {
            assert_eq!(result.unwrap(), expected_verified.unwrap());
        }
    }
}

#[test]
fn credential_validations_match_rails() {
    let t = db();
    credential(&t, "david");
    assert!(matches!(
        t.try_write(|tx| Credential::create(tx, encryption(), id("david"), SECRET)),
        Err(Error::RecordInvalid(_))
    ));
    for secret in ["", " ", "\u{a0}"] {
        assert!(matches!(
            t.try_write(move |tx| Credential::create(tx, encryption(), id("jason"), secret)),
            Err(Error::RecordInvalid(_))
        ));
    }
    assert!(matches!(
        t.try_write(|tx| Credential::create(tx, encryption(), -1, SECRET)),
        Err(Error::RecordInvalid(_))
    ));
}

#[test]
fn provisioning_and_formatted_keys_match_rails() {
    let t = db();
    let credential = credential(&t, "david");
    let v = vectors();
    assert_eq!(
        credential
            .provisioning_uri(encryption(), "david@example.com")
            .unwrap(),
        v["provisioning_uri"]
    );
    assert_eq!(
        credential.formatted_secret(encryption()).unwrap(),
        v["formatted_secret"]
    );
}

#[test]
fn only_active_humans_require_two_factor() {
    let t = db();
    assert!(t.read(|c| User::find(c, id("david"))).requires_two_factor());
    assert!(
        !t.read(|c| User::find(c, id("bender")))
            .requires_two_factor()
    );
    let mut user = t.read(|c| User::find(c, id("jason")));
    user.status = crate::Status::Deactivated;
    assert!(!user.requires_two_factor());
    user.status = crate::Status::Banned;
    assert!(!user.requires_two_factor());
}

#[test]
fn backup_regeneration_returns_ten_codes_and_stores_only_their_digests() {
    let t = db();
    let credential = credential(&t, "david");
    let codes = t.write(move |tx| Backup::regenerate_set(tx, credential.id));
    let rows = t.read(|c| Backup::for_credential(c, credential.id));
    assert_eq!(codes.len(), 10);
    let unique: std::collections::HashSet<_> = codes.iter().collect();
    assert_eq!(unique.len(), 10);
    for code in codes {
        assert_eq!(code.len(), 10);
        assert!(
            code.bytes()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        );
        assert!(
            rows.iter()
                .any(|row| row.code_digest == Backup::digest(&code) && row.used_at.is_none())
        );
        assert!(!rows.iter().any(|row| row.code_digest == code));
    }
}

#[test]
fn regenerating_backup_codes_invalidates_every_old_code() {
    let t = db();
    let credential = credential(&t, "david");
    let codes = t.write(move |tx| Backup::regenerate_set(tx, credential.id));
    t.write(move |tx| Backup::regenerate_set(tx, credential.id));
    for code in codes {
        assert!(!consume(&t, credential.id, &code));
    }
}

#[test]
fn backup_codes_are_single_use_and_stamp_both_timestamps() {
    let t = db();
    let credential = credential(&t, "david");
    let codes = t.write(move |tx| Backup::regenerate_set(tx, credential.id));
    t.travel(1);
    assert!(consume(&t, credential.id, &codes[0]));
    t.travel(1);
    assert!(!consume(&t, credential.id, &codes[0]));
    let rows = t.read(|c| Backup::for_credential(c, credential.id));
    assert_eq!(rows.iter().filter(|row| row.used_at.is_none()).count(), 9);
    let row = rows.iter().find(|row| row.used_at.is_some()).unwrap();
    assert_eq!(
        row.used_at,
        Some(t.now().ago(jiff::SignedDuration::from_secs(1)))
    );
    assert_eq!(row.updated_at, row.used_at.unwrap());
}

#[test]
fn backup_normalization_matches_rails_and_accepts_case_spaces_and_dashes() {
    for case in vectors()["backups"].as_array().unwrap() {
        let input = case["input"].as_str().unwrap_or("");
        assert_eq!(Backup::normalize(input), case["normalized"]);
        assert_eq!(Backup::digest(input), case["digest"]);
    }
    let t = db();
    let credential = credential(&t, "david");
    let codes = t.write(move |tx| Backup::regenerate_set(tx, credential.id));
    let code = format!("{}- {}", &codes[0][..4], &codes[0][4..]).to_uppercase();
    assert!(consume(&t, credential.id, &code));
}

#[test]
fn backup_codes_reject_unknown_blank_and_foreign_codes() {
    let t = db();
    let own = credential(&t, "david");
    let other = credential(&t, "jason");
    let codes = t.write(move |tx| Backup::regenerate_set(tx, other.id));
    for code in ["", "not-a-code", " -\t ", &codes[0]] {
        assert!(!consume(&t, own.id, code));
    }
}

#[test]
fn backup_digest_validations_match_rails() {
    let t = db();
    let credential = credential(&t, "david");
    t.write(move |tx| Backup::create(tx, credential.id, "example-digest"));
    assert!(matches!(
        t.try_write(move |tx| Backup::create(tx, credential.id, "example-digest")),
        Err(Error::RecordInvalid(_))
    ));
    assert!(matches!(
        t.try_write(move |tx| Backup::create(tx, credential.id, "")),
        Err(Error::RecordInvalid(_))
    ));
    assert!(matches!(
        t.try_write(|tx| Backup::create(tx, -1, "example-other")),
        Err(Error::RecordInvalid(_))
    ));
}

#[test]
fn setup_issuance_rotates_one_row_and_keeps_sessions_separate() {
    let t = db();
    let first = setup(&t, "david_safari");
    let second = setup(&t, "david_safari");
    let other_session = t.write(|tx| crate::Session::start(tx, id("jason"), None, None));
    let other = t.write(move |tx| Setup::issue_for(tx, encryption(), other_session.id));
    assert_eq!(first.id, second.id);
    assert_ne!(
        first.secret(encryption()).unwrap(),
        second.secret(encryption()).unwrap()
    );
    assert_ne!(
        second.secret(encryption()).unwrap(),
        other.secret(encryption()).unwrap()
    );
    assert_eq!(
        t.read(|c| Setup::valid_for(c, id("david_safari"), t.now()))
            .unwrap()
            .secret(encryption())
            .unwrap(),
        second.secret(encryption()).unwrap()
    );
}

#[test]
fn setup_expiry_is_exact_and_extending_preserves_the_secret() {
    let t = db();
    assert!(
        t.read(|c| Setup::valid_for(c, id("david_safari"), t.now()))
            .is_none()
    );
    let mut pending = setup(&t, "david_safari");
    assert_eq!(pending.expires_at, t.now().since(SETUP_TTL));
    t.travel(900);
    pending = t.write(move |tx| {
        pending.extend_expiry(tx)?;
        Ok(pending)
    });
    let secret = pending.secret(encryption()).unwrap();
    assert_eq!(pending.expires_at, t.now().since(SETUP_TTL));
    t.travel(1799);
    assert_eq!(
        t.read(|c| Setup::valid_for(c, id("david_safari"), t.now()))
            .unwrap()
            .secret(encryption())
            .unwrap(),
        secret
    );
    t.travel(1);
    assert!(
        t.read(|c| Setup::valid_for(c, id("david_safari"), t.now()))
            .is_none()
    );
    assert!(matches!(
        t.try_write(|tx| Setup::issue_for(tx, encryption(), -1)),
        Err(Error::RecordInvalid(_))
    ));
}

#[test]
fn setup_secret_is_encrypted_and_deleted_with_its_session() {
    let t = db();
    let pending = setup(&t, "david_safari");
    let stored: String = t.read(|c| {
        Ok(c.query_row(
            "SELECT secret FROM two_factor_setup_secrets WHERE id = ?",
            [pending.id],
            |r| r.get(0),
        )?)
    });
    let secret = pending.secret(encryption()).unwrap();
    assert_ne!(secret, stored);
    assert_eq!(encryption().decrypt(&stored).unwrap(), secret);
    t.write(|tx| crate::Session::find(tx.conn(), id("david_safari"))?.destroy(tx));
    assert!(
        t.read(|c| Setup::valid_for(c, id("david_safari"), t.now()))
            .is_none()
    );
}

#[test]
fn remembered_device_stores_a_digest_expires_in_thirty_days_and_stamps_use() {
    let t = db();
    let (device, token) =
        t.write(|tx| Remembered::create_for(tx, id("david"), Some("Browser"), Some("1.2.3.4")));
    assert_eq!(token.len(), 64);
    assert_eq!(device.token_digest, Remembered::digest(&token));
    assert_eq!(device.expires_at, t.now().since(REMEMBER_FOR));
    t.travel(86400);
    let spaced = format!(" \t{token}\0 ");
    let found = t
        .write(move |tx| Remembered::find_valid(tx, Some(&spaced), Some(id("david"))))
        .unwrap();
    assert_eq!(device.id, found.id);
    assert_eq!(found.last_used_at, Some(t.now()));
    assert_eq!(found.updated_at, t.now());
}

#[test]
fn remembered_devices_reject_blank_unknown_foreign_and_expired_tokens() {
    let t = db();
    let (device, token) = t.write(|tx| Remembered::create_for(tx, id("david"), None, None));
    let clone = token.clone();
    t.write(move |tx| {
        for token in [None, Some(""), Some("   "), Some("unknown")] {
            assert!(Remembered::find_valid(tx, token, Some(id("david")))?.is_none());
        }
        assert!(Remembered::find_valid(tx, Some(&clone), Some(id("jason")))?.is_none());
        assert!(Remembered::find_valid(tx, Some(&clone), None)?.is_none());
        Ok(())
    });
    t.travel(30 * 24 * 60 * 60);
    assert!(
        t.write(move |tx| Remembered::find_valid(tx, Some(&token), Some(id("david"))))
            .is_none()
    );
    let unchanged = t
        .read(|c| Remembered::for_user(c, id("david")))
        .pop()
        .unwrap();
    assert_eq!(unchanged, device);
}

#[test]
fn remembered_devices_validate_and_truncate_user_agents_like_rails() {
    let t = db();
    let ua = "é".repeat(600);
    let (device, _) = t.write(move |tx| Remembered::create_for(tx, id("david"), Some(&ua), None));
    assert_eq!(
        device.user_agent.unwrap(),
        format!("{}...", "é".repeat(509))
    );
    let digest = device.token_digest;
    assert!(matches!(
        t.try_write(move |tx| Remembered::create(tx, id("jason"), &digest, None, None, tx.now())),
        Err(Error::RecordInvalid(_))
    ));
    assert!(matches!(
        t.try_write(|tx| Remembered::create(tx, id("jason"), "", None, None, tx.now())),
        Err(Error::RecordInvalid(_))
    ));
    assert!(matches!(
        t.try_write(|tx| Remembered::create_for(tx, -1, None, None)),
        Err(Error::RecordInvalid(_))
    ));
}

#[test]
fn remembered_device_revocation_is_scoped_and_recent_first_is_stable() {
    let t = db();
    let (first, _) = t.write(|tx| Remembered::create_for(tx, id("david"), None, None));
    let (second, _) = t.write(|tx| Remembered::create_for(tx, id("david"), None, None));
    assert_eq!(
        t.read(|c| Remembered::for_user(c, id("david")))
            .iter()
            .map(|d| d.id)
            .collect::<Vec<_>>(),
        vec![second.id, first.id]
    );
    t.write(move |tx| Remembered::revoke(tx, id("jason"), first.id));
    assert_eq!(t.read(|c| Remembered::for_user(c, id("david"))).len(), 2);
    t.write(move |tx| Remembered::revoke(tx, id("david"), first.id));
    assert_eq!(t.read(|c| Remembered::for_user(c, id("david"))).len(), 1);
    t.write(|tx| Remembered::revoke_all(tx, id("david")));
    assert!(t.read(|c| Remembered::for_user(c, id("david"))).is_empty());
}

#[test]
fn reset_clears_the_credential_codes_and_remembered_devices_only_for_that_user() {
    let t = db();
    let credential = credential(&t, "david");
    let other = credential_for_jason(&t);
    t.write(move |tx| {
        Backup::regenerate_set(tx, credential.id)?;
        Backup::regenerate_set(tx, other.id)?;
        Remembered::create_for(tx, id("david"), None, None)?;
        Remembered::create_for(tx, id("jason"), None, None)?;
        User::find(tx.conn(), id("david"))?.reset_two_factor(tx)
    });
    assert!(!t.read(|c| User::find(c, id("david"))?.two_factor_enabled(c)));
    assert!(t.read(|c| Credential::for_user(c, id("david"))).is_none());
    assert!(
        t.read(|c| Backup::for_credential(c, credential.id))
            .is_empty()
    );
    assert!(t.read(|c| Remembered::for_user(c, id("david"))).is_empty());
    assert_eq!(t.read(|c| Backup::for_credential(c, other.id)).len(), 10);
    assert_eq!(t.read(|c| Remembered::for_user(c, id("jason"))).len(), 1);
}

#[test]
fn backup_regeneration_rolls_back_if_any_insert_fails() {
    let t = db();
    let credential = credential(&t, "david");
    let old = t.write(move |tx| Backup::regenerate_set(tx, credential.id));
    t.write(|tx| { tx.conn().execute_batch("CREATE TRIGGER refuse_backup BEFORE INSERT ON two_factor_backup_codes BEGIN SELECT RAISE(ABORT, 'injected'); END;")?; Ok(()) });
    assert!(
        t.try_write(move |tx| Backup::regenerate_set(tx, credential.id))
            .is_err()
    );
    for code in old {
        assert!(consume(&t, credential.id, &code));
    }
}

#[test]
fn confirmation_rolls_back_if_spending_the_setup_secret_fails() {
    let t = db();
    let mut credential = credential(&t, "david");
    let pending = setup(&t, "david_safari");
    let code = code_for(&pending.secret(encryption()).unwrap(), &t);
    t.write(|tx| { tx.conn().execute_batch("CREATE TRIGGER refuse_setup_delete BEFORE DELETE ON two_factor_setup_secrets BEGIN SELECT RAISE(ABORT, 'injected'); END;")?; Ok(()) });
    assert!(
        t.try_write(move |tx| credential.confirm_with_setup_secret(
            tx,
            encryption(),
            &pending,
            &code
        ))
        .is_err()
    );
    assert!(
        !t.read(|c| Credential::for_user(c, id("david")))
            .unwrap()
            .enabled()
    );
    assert!(
        t.read(|c| Setup::valid_for(c, id("david_safari"), t.now()))
            .is_some()
    );
}

#[test]
fn concurrent_totp_verifications_from_stale_copies_spend_the_step_once() {
    let t = db();
    let credential = credential(&t, "david");
    let code = code_for(SECRET, &t);
    let db = Arc::new(t.db);
    let barrier = Arc::new(Barrier::new(8));
    let workers: Vec<_> = (0..8)
        .map(|_| {
            let db = db.clone();
            let barrier = barrier.clone();
            let mut credential = credential.clone();
            let code = code.clone();
            std::thread::spawn(move || {
                barrier.wait();
                db.write_blocking(move |tx| credential.verify_code(tx, encryption(), &code))
                    .unwrap()
            })
        })
        .collect();
    assert_eq!(
        workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .filter(|accepted| *accepted)
            .count(),
        1
    );
}

#[test]
fn concurrent_backup_code_consumers_spend_one_code_once() {
    let t = db();
    let credential = credential(&t, "david");
    let codes = t.write(move |tx| Backup::regenerate_set(tx, credential.id));
    let db = Arc::new(t.db);
    let barrier = Arc::new(Barrier::new(8));
    let workers: Vec<_> = (0..8)
        .map(|_| {
            let db = db.clone();
            let barrier = barrier.clone();
            let code = codes[0].clone();
            std::thread::spawn(move || {
                barrier.wait();
                db.write_blocking(move |tx| Backup::consume(tx, credential.id, &code))
                    .unwrap()
            })
        })
        .collect();
    assert_eq!(
        workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .filter(|accepted| *accepted)
            .count(),
        1
    );
}

#[test]
fn concurrent_failures_start_one_lockout_and_do_not_increment_during_it() {
    let t = db();
    let credential = credential(&t, "david");
    let db = Arc::new(t.db);
    let barrier = Arc::new(Barrier::new(10));
    let workers: Vec<_> = (0..10)
        .map(|_| {
            let db = db.clone();
            let barrier = barrier.clone();
            let mut credential = credential.clone();
            std::thread::spawn(move || {
                barrier.wait();
                db.write_blocking(move |tx| credential.register_challenge_failure(tx))
                    .unwrap()
            })
        })
        .collect();
    assert_eq!(
        workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .filter(|outcome| *outcome == ChallengeFailure::Locked)
            .count(),
        1
    );
    let stored = db
        .read_blocking(|c| Credential::find(c, credential.id))
        .unwrap();
    assert_eq!((stored.consecutive_failures, stored.lockout_count), (0, 1));
}
