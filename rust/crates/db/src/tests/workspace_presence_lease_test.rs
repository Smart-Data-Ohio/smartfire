use super::*;
use crate::models::workspace_presence_lease::{IDLE_AFTER, Presence, TTL};
use crate::{Timestamp, WorkspacePresenceLease as Lease};
use jiff::SignedDuration;

fn establish(t: &TestDb) -> Lease {
    t.write(|tx| Lease::establish(tx, id("david"), id("david_safari")))
        .unwrap()
}
fn presence(t: &TestDb) -> Option<Presence> {
    t.read(|c| {
        Ok(Lease::presence_by_user_id(c, &[id("david")], t.now())?
            .get(&id("david"))
            .copied())
    })
}

#[test]
fn ws17_fresh_lease_is_online_and_has_a_uuid_and_90_second_ttl() {
    let t = TestDb::new();
    let lease = establish(&t);
    assert_eq!(lease.expires_at, lease.created_at.since(TTL));
    assert!(uuid::Uuid::parse_str(&lease.connection_id).is_ok());
    assert_eq!(presence(&t), Some(Presence::Online));
}
#[test]
fn ws17_expiry_is_inclusive_and_reads_never_prune() {
    let t = TestDb::new();
    let lease = establish(&t);
    t.clock.travel_to(lease.expires_at);
    assert_eq!(presence(&t), Some(Presence::Online));
    t.clock.travel(SignedDuration::from_micros(1));
    assert_eq!(presence(&t), None);
    assert!(t.read(|c| Lease::find_by_id(c, lease.id)).is_some());
    assert_eq!(t.write(|tx| Lease::prune(tx, 100)), 1);
}
#[test]
fn ws17_deleting_one_connection_keeps_the_other_live() {
    let t = TestDb::new();
    let mut first = establish(&t);
    establish(&t);
    t.write(move |tx| first.delete(tx));
    assert_eq!(presence(&t), Some(Presence::Online));
}
#[test]
fn ws17_session_mismatch_is_absent_and_pruned() {
    let t = TestDb::new();
    let lease = establish(&t);
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE workspace_presence_leases SET user_id=? WHERE id=?",
            rusqlite::params![id("jason"), lease.id],
        )?;
        Ok(())
    });
    assert!(
        t.read(|c| Lease::presence_by_user_id(c, &[id("jason")], t.now()))
            .is_empty()
    );
    assert_eq!(t.write(|tx| Lease::prune(tx, 100)), 1);
    assert!(
        t.write(|tx| Lease::establish(tx, id("jason"), id("david_safari")))
            .is_none()
    );
}
#[test]
fn ws17_revoked_session_refresh_deletes_lease() {
    let t = TestDb::new();
    let mut lease = establish(&t);
    t.write(|tx| {
        tx.conn()
            .execute("DELETE FROM sessions WHERE id=?", [id("david_safari")])?;
        Ok(())
    });
    assert_eq!(presence(&t), None);
    assert!(!t.write(move |tx| lease.refresh(tx, false)));
}
#[test]
fn ws17_inactive_user_cannot_establish_or_refresh() {
    let t = TestDb::new();
    let mut lease = establish(&t);
    t.write(|tx| {
        tx.conn()
            .execute("UPDATE users SET status=1 WHERE id=?", [id("david")])?;
        Ok(())
    });
    assert_eq!(presence(&t), None);
    assert!(!t.write(move |tx| lease.refresh(tx, false)));
    assert!(
        t.write(|tx| Lease::establish(tx, id("david"), id("david_safari")))
            .is_none()
    );
}
#[test]
fn ws17_prune_has_bounded_batches() {
    let t = TestDb::new();
    establish(&t);
    establish(&t);
    t.clock.travel(TTL + SignedDuration::from_secs(1));
    assert_eq!(t.write(|tx| Lease::prune(tx, 1)), 1);
    assert_eq!(t.write(|tx| Lease::prune(tx, 100)), 1);
}
#[test]
fn ws17_quiet_heartbeat_extends_only_expiry_and_active_heartbeat_restores_online() {
    let t = TestDb::new();
    let mut lease = establish(&t);
    let original = lease.clone();
    t.clock.travel(IDLE_AFTER + SignedDuration::from_secs(1));
    lease = t.write(move |tx| {
        assert!(lease.refresh(tx, false)?);
        Ok(lease)
    });
    assert_eq!(lease.last_active_at, original.last_active_at);
    assert_eq!(lease.updated_at, original.updated_at); // update_columns does not touch timestamps.
    assert_eq!(presence(&t), Some(Presence::Idle));
    t.write(move |tx| lease.refresh(tx, true));
    assert_eq!(presence(&t), Some(Presence::Online));
}
#[test]
fn ws17_any_recent_or_legacy_null_activity_keeps_user_online() {
    let t = TestDb::new();
    let mut first = establish(&t);
    let second = establish(&t);
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE workspace_presence_leases SET last_active_at=? WHERE id=?",
            rusqlite::params![
                tx.now().ago(IDLE_AFTER + SignedDuration::from_secs(1)),
                second.id
            ],
        )?;
        Ok(())
    });
    assert_eq!(presence(&t), Some(Presence::Online));
    t.write(move |tx| first.delete(tx));
    assert_eq!(presence(&t), Some(Presence::Idle));
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE workspace_presence_leases SET last_active_at=NULL WHERE id=?",
            [second.id],
        )?;
        Ok(())
    });
    assert_eq!(presence(&t), Some(Presence::Online));
}
#[test]
fn ws17_absent_users_and_expired_leases_are_not_returned() {
    let t = TestDb::new();
    assert!(
        t.read(|c| Lease::presence_by_user_id(c, &[], Timestamp::from_second(0)))
            .is_empty()
    );
    assert_eq!(presence(&t), None);
}
