//! Generates real Rust-written rows, which `reference-tools/auth/rollback.sh` reads with Rails.
use super::*;
use crate::{
    DeviceSignIn, NewSession, Session, Timestamp, TwoFactorBackupCode as Backup,
    TwoFactorCredential as Credential, TwoFactorRememberedDevice as Remembered,
    TwoFactorSetupSecret as Setup, UserDevice,
};
use rails_compat::{Secrets, ar_encryption::ArEncryption, cookies, totp};

#[test]
fn write_rails_rollback_fixture() {
    let temporary = tempfile::tempdir().unwrap();
    let directory = std::env::var_os("WS9_ROLLBACK_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| temporary.path().to_owned());
    std::fs::create_dir_all(&directory).unwrap();
    let vectors: serde_json::Value =
        serde_json::from_str(include_str!("../../../../vectors/two_factor.json")).unwrap();
    let now = Timestamp::from_second(vectors["now"].as_i64().unwrap());
    let secrets = Secrets::new(vectors["secret_key_base"].as_str().unwrap());
    let encryption = ArEncryption::new(&secrets);
    let env = Env {
        clock: Arc::new(TestClock::frozen_at(now)),
        bcrypt_cost: 4,
        ..Default::default()
    };
    let mut config = Config::new(directory.join("rollback.sqlite3"));
    config.environment = "test".into();
    let db = Database::open(config, env).unwrap();
    let manifest = db.write_blocking(move |tx| {
        fixtures::load(tx.conn(), &fixtures::reference_dir(), &fixtures::Options { now, bcrypt_cost: 4 })?;
        let mut verified = Session::start_with(tx, id("david"), NewSession { device_id: Some("ws9-device"), user_agent: Some("ws9-browser"), ip_address: Some("1.2.3.4"), ..Default::default() })?;
        let unverified = Session::start(tx, id("jason"), Some("ws9-other"), Some("1.2.3.5"))?;
        let mut credential = Credential::create(tx, &encryption, id("david"), vectors["secret"].as_str().unwrap())?;
        let enrollment = loop {
            let enrollment = Setup::issue_for(tx, &encryption, verified.id)?;
            let secret = enrollment.secret(&encryption)?;
            if totp::at(&secret, now.as_second()).unwrap() != totp::at(&secret, now.as_second() + 30).unwrap() { break enrollment; }
        };
        let enrollment_secret = enrollment.secret(&encryption)?;
        let enrollment_code = totp::at(&enrollment_secret, now.as_second()).unwrap();
        assert!(credential.confirm_with_setup_secret(tx, &encryption, &enrollment, &enrollment_code)?);
        verified.mark_two_factor_verified(tx)?;
        let pending = Setup::issue_for(tx, &encryption, unverified.id)?;
        let pending_secret = pending.secret(&encryption)?;
        let pending_credential = Credential::create(tx, &encryption, id("jason"), vectors["secret"].as_str().unwrap())?;
        let codes = Backup::regenerate_set(tx, credential.id)?;
        assert!(Backup::consume(tx, credential.id, &codes[0])?);
        let (remembered, token) = Remembered::create_for(tx, id("david"), Some("ws9-browser"), Some("1.2.3.4"))?;
        assert_eq!(UserDevice::record_sign_in(tx, id("david"), Some("ws9-device"), Some("ws9-browser"))?, DeviceSignIn::FirstSeen);
        let cookie = cookies::two_factor_remember_cookie(&secrets, &token, now.jiff());
        let device_cookie = cookies::device_id_cookie(&secrets, "ws9-device", now.jiff());
        Ok(serde_json::json!({
            "now": now.as_second(), "credential_id": credential.id, "pending_credential_id": pending_credential.id,
            "enrollment_secret": enrollment_secret, "enrollment_code": enrollment_code,
            "setup_id": pending.id, "pending_secret": pending_secret,
            "verified_session_id": verified.id, "verified_session_token": verified.token,
            "unverified_session_id": unverified.id, "unverified_session_token": unverified.token,
            "remembered_id": remembered.id, "remembered_token": token,
            "remember_cookie": cookie.value, "remember_cookie_header": cookie.set_cookie_header(),
            "device_cookie": device_cookie.value, "backup_codes": codes,
        }))
    }).unwrap();
    std::fs::write(
        directory.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
    assert!(
        db.read_blocking(|conn| Ok(Credential::find(
            conn,
            manifest["credential_id"].as_i64().unwrap()
        )?
        .enabled()))
            .unwrap()
    );
    // Close all connections before Rails opens the snapshot.
    drop(db);
}

#[test]
#[ignore = "requires Rails to mutate the rollback fixture; run reference-tools/auth/rollback.sh"]
fn read_rails_rollback_changes() {
    let directory =
        std::path::PathBuf::from(std::env::var_os("WS9_ROLLBACK_DIR").expect("WS9_ROLLBACK_DIR"));
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(directory.join("manifest.json")).unwrap()).unwrap();
    let readback: serde_json::Value =
        serde_json::from_slice(&std::fs::read(directory.join("rails-readback.json")).unwrap())
            .unwrap();
    let vectors: serde_json::Value =
        serde_json::from_str(include_str!("../../../../vectors/two_factor.json")).unwrap();
    let secrets = Secrets::new(vectors["secret_key_base"].as_str().unwrap());
    let encryption = ArEncryption::new(&secrets);
    let now = Timestamp::from_second(manifest["now"].as_i64().unwrap() + 30);
    let mut config = Config::new(directory.join("rollback.sqlite3"));
    config.environment = "test".into();
    let db = Database::open(
        config,
        Env {
            clock: Arc::new(TestClock::frozen_at(now)),
            ..Default::default()
        },
    )
    .unwrap();
    db.read_blocking(|conn| {
        let credential = Credential::find(conn, manifest["credential_id"].as_i64().unwrap())?;
        assert_eq!(
            credential.secret(&encryption)?,
            manifest["enrollment_secret"]
        );
        assert_eq!(credential.last_totp_at, Some(now.as_second()));
        let pending = Setup::valid_for(
            conn,
            manifest["unverified_session_id"].as_i64().unwrap(),
            now,
        )?
        .unwrap();
        assert_eq!(pending.secret(&encryption)?, readback["pending_secret"]);
        assert_eq!(
            Backup::for_credential(conn, credential.id)?
                .iter()
                .filter(|code| code.used_at.is_some())
                .count(),
            2
        );
        assert!(
            Session::find(conn, manifest["verified_session_id"].as_i64().unwrap())?
                .two_factor_verified()
        );
        assert!(
            !Session::find(conn, manifest["unverified_session_id"].as_i64().unwrap())?
                .two_factor_verified()
        );
        Ok(())
    })
    .unwrap();
    let token = cookies::read_two_factor_remember(
        &secrets,
        readback["remember_cookie"].as_str().unwrap(),
        now.jiff(),
    )
    .unwrap();
    assert_eq!(token, manifest["remembered_token"]);
    assert!(
        db.write_blocking(move |tx| Remembered::find_valid(tx, Some(&token), Some(id("david"))))
            .unwrap()
            .is_some()
    );
}
