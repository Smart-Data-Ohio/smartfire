//! `test/models/user_device_test.rb`, plus the creation race and missing-user validation.
use super::*;
use crate::{DeviceSignIn, Error, User, UserDevice};

#[test]
fn first_sign_in_is_first_seen_and_persists_the_device() {
    let t = TestDb::new();
    assert_eq!(
        t.write(|tx| UserDevice::record_sign_in(tx, id("kevin"), Some("device-a"), Some("UA/1"))),
        DeviceSignIn::FirstSeen
    );
    let devices = t.read(|c| UserDevice::for_user(c, id("kevin")));
    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0].device_id, "device-a");
}

#[test]
fn a_known_device_refreshes_its_user_agent_and_timestamp() {
    let t = TestDb::new();
    t.write(|tx| UserDevice::record_sign_in(tx, id("kevin"), Some("device-a"), Some("UA/1")));
    let created_at = t.read(|c| UserDevice::for_user(c, id("kevin")))[0].created_at;
    t.travel(1);
    assert_eq!(
        t.write(|tx| UserDevice::record_sign_in(tx, id("kevin"), Some("device-a"), Some("UA/2"))),
        DeviceSignIn::Known
    );
    let devices = t.read(|c| UserDevice::for_user(c, id("kevin")));
    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0].user_agent.as_deref(), Some("UA/2"));
    assert_eq!(devices[0].created_at, created_at);
    assert!(devices[0].updated_at > created_at);
}

#[test]
fn a_different_cookie_is_a_new_device() {
    let t = TestDb::new();
    t.write(|tx| UserDevice::record_sign_in(tx, id("kevin"), Some("device-a"), None));
    assert_eq!(
        t.write(|tx| UserDevice::record_sign_in(tx, id("kevin"), Some("device-b"), None)),
        DeviceSignIn::NewDevice
    );
    assert_eq!(t.read(|c| UserDevice::for_user(c, id("kevin"))).len(), 2);
}

#[test]
fn blank_ids_are_unknown_and_never_write_a_device() {
    let t = TestDb::new();
    for device_id in [None, Some(""), Some(" \t"), Some("\u{a0}")] {
        assert_eq!(
            t.write(move |tx| UserDevice::record_sign_in(tx, id("kevin"), device_id, None)),
            DeviceSignIn::Unknown
        );
    }
    assert!(t.read(|c| UserDevice::for_user(c, id("kevin"))).is_empty());
}

#[test]
fn device_ids_are_scoped_to_the_user() {
    let t = TestDb::new();
    t.write(|tx| UserDevice::record_sign_in(tx, id("kevin"), Some("device-a"), None));
    assert_eq!(
        t.write(|tx| UserDevice::record_sign_in(tx, id("jz"), Some("device-a"), None)),
        DeviceSignIn::FirstSeen
    );
}

#[test]
fn deactivation_removes_known_devices_and_setup_secrets_with_sessions() {
    let t = TestDb::new();
    t.write(|tx| UserDevice::record_sign_in(tx, id("kevin"), Some("device-a"), None));
    t.write(|tx| {
        let encryption = rails_compat::ar_encryption::ArEncryption::from_key(&[7; 32]);
        let session = crate::Session::start(tx, id("kevin"), None, None)?;
        crate::TwoFactorSetupSecret::issue_for(tx, &encryption, session.id)?;
        User::find(tx.conn(), id("kevin"))?.deactivate(tx)
    });
    assert!(t.read(|c| UserDevice::for_user(c, id("kevin"))).is_empty());
    assert_eq!(
        t.read(|c| crate::sql::count(c, "SELECT COUNT(*) FROM two_factor_setup_secrets", [])),
        0
    );
}

#[test]
fn missing_users_fail_the_belongs_to_validation() {
    let t = TestDb::new();
    assert!(matches!(
        t.try_write(|tx| UserDevice::record_sign_in(tx, -1, Some("device-a"), None)),
        Err(Error::RecordInvalid(_))
    ));
}

#[test]
fn concurrent_sign_ins_share_one_device() {
    let t = TestDb::new();
    let db = Arc::new(t.db);
    let barrier = Arc::new(std::sync::Barrier::new(8));
    let workers: Vec<_> = (0..8)
        .map(|_| {
            let db = db.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                db.write_blocking(|tx| {
                    UserDevice::record_sign_in(tx, id("kevin"), Some("device-a"), None)
                })
                .unwrap()
            })
        })
        .collect();
    let outcomes: Vec<_> = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect();
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| **outcome == DeviceSignIn::FirstSeen)
            .count(),
        1
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| **outcome == DeviceSignIn::Known)
            .count(),
        7
    );
    assert_eq!(
        db.read_blocking(|c| UserDevice::for_user(c, id("kevin")))
            .unwrap()
            .len(),
        1
    );
}
