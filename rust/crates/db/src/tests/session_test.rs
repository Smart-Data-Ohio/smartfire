//! `test/models/session_test.rb`: the columns our sessions added (`device_id`,
//! `two_factor_verified_at`).

use super::*;
use crate::{NewSession, Session};

#[test]
fn start_records_the_device_and_verification() {
    let t = TestDb::new();
    let user_id = id("david");
    let session = t.write(move |tx| {
        Session::start_with(
            tx,
            user_id,
            NewSession {
                user_agent: Some("ua"),
                ip_address: Some("1.2.3.4"),
                device_id: Some("device-1"),
                two_factor_verified: true,
            },
        )
    });
    assert_eq!(session.device_id.as_deref(), Some("device-1"));
    assert_eq!(session.two_factor_verified_at, Some(session.created_at));
    assert!(session.two_factor_verified());
    assert_eq!(t.read(|c| Session::find(c, session.id)), session);
}

#[test]
fn start_defaults_to_no_device_and_unverified() {
    let t = TestDb::new();
    let session = t.write(|tx| Session::start(tx, id("david"), Some("ua"), None));
    let stored = t.read(|c| Session::find(c, session.id));
    assert_eq!((stored.device_id, stored.two_factor_verified_at), (None, None));
    assert!(!session.two_factor_verified());
}

#[test]
fn marking_and_clearing_two_factor_verification() {
    let t = TestDb::new();
    let session = t.write(|tx| Session::start(tx, id("david"), Some("ua"), None));
    t.travel(10);
    let marked = t.write(move |tx| {
        let mut s = session;
        s.mark_two_factor_verified(tx)?;
        Ok(s)
    });
    let at = marked.two_factor_verified_at.expect("verified");
    assert!(marked.updated_at >= at);
    assert_eq!(t.read(|c| Session::find(c, marked.id)), marked);

    // Already verified: the first stamp stays.
    t.travel(10);
    let again = t.write(move |tx| {
        let mut s = marked;
        s.mark_two_factor_verified(tx)?;
        Ok(s)
    });
    assert_eq!(t.read(|c| Session::find(c, again.id)).two_factor_verified_at, Some(at));

    let cleared = t.write(move |tx| {
        let mut s = again;
        s.clear_two_factor_verified(tx)?;
        Ok(s)
    });
    assert_eq!(t.read(|c| Session::find(c, cleared.id)).two_factor_verified_at, None);
}

/// Destroying a session removes its presence leases (`dependent: :delete_all`).
#[test]
fn destroying_a_session_removes_its_presence_leases() {
    let t = TestDb::new();
    let session_id = id("david_safari");
    t.write(move |tx| {
        let now = tx.now();
        tx.conn().execute(
            "INSERT INTO workspace_presence_leases (connection_id, created_at, expires_at, session_id, updated_at, user_id) VALUES ('c1', ?, ?, ?, ?, ?)",
            rusqlite::params![now, now, session_id, now, id("david")],
        )?;
        Ok(())
    });
    let session = t.read(|c| Session::find(c, session_id));
    t.write(move |tx| session.destroy(tx));
    let left = t.read(|c| crate::sql::count(c, "SELECT COUNT(*) FROM workspace_presence_leases WHERE session_id = ?", [session_id]));
    assert_eq!(left, 0);
}
