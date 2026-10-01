//! Delayed Rust rings must preserve Rails' synchronous revoke/end ordering.
use crate::models::huddle_grant::HuddleGrant;
use crate::models::room_delete::HuddleConfig;
use crate::tests::TestDb;
use crate::{Membership, Session};
fn config() -> HuddleConfig {
    HuddleConfig { api_secret:Some("ws13b-review-fixture-value".into()), admin_configured:false }
}
fn review_direct_setup(db: &TestDb) -> (i64, i64, i64) {
    db.write(|tx| {
        let room = crate::fixtures::identify("david_and_jason");
        let user = crate::fixtures::identify("david");
        let membership = Membership::find_by_room_and_user(tx.conn(), room, user)?.unwrap();
        let session = Session::start(tx, user, None, None)?;
        Ok((room, membership.id, session.id))
    })
}

#[test]
fn ws13b_review_queued_ring_does_not_leak_after_recipient_removal() {
    use crate::models::huddle_invitations::{RingRequest, publish_ring_with_policy};
    let db = TestDb::new();
    let (room, membership, session) = review_direct_setup(&db);
    db.sink.take();
    db.write(move |tx| HuddleGrant::issue(tx, session, membership, room, &config()));
    let recipient = crate::fixtures::identify("jason");
    let request = db.sink.take().iter().filter_map(|e| e.as_job::<RingRequest>()).find(|r| r.recipient_id == recipient).unwrap();
    db.write(move |tx| Membership::find_by_room_and_user(tx.conn(), room, recipient)?.unwrap().destroy(tx));
    db.sink.take();
    db.write(move |tx| publish_ring_with_policy(tx, &request, None));
    assert!(!db.sink.take().iter().any(|event| event.as_broadcast().is_some()), "delayed ring disclosed the removed room/caller");
}

#[test]
fn ws13b_review_queued_ring_does_not_restart_ended_calls() {
    use crate::models::huddle_invitations::{RingRequest, publish_ring_with_policy};
    for (operation, suppressed) in [("revoke", false), ("remove_caller", false), ("sign_out", false), ("revoke", true)] {
        let db = TestDb::new();
        let (room, membership, session) = review_direct_setup(&db);
        let recipient = crate::fixtures::identify("jason");
        if suppressed { db.write(move |tx| Ok(tx.conn().execute(r#"UPDATE users SET inbox_preferences='{"huddle_invitations":false}' WHERE id=?"#, [recipient])?)); }
        db.sink.take();
        let grant = db.write(move |tx| HuddleGrant::issue(tx, session, membership, room, &config()));
        let request = db.sink.take().iter().filter_map(|e| e.as_job::<RingRequest>()).find(|r| r.recipient_id == recipient).unwrap();
        db.write(move |tx| {
            HuddleGrant::find_by_id(tx.conn(), grant.id)?.unwrap().record_seen(tx)?;
            match operation {
                "revoke" => HuddleGrant::find_by_id(tx.conn(), grant.id)?.unwrap().revoke(tx, false, &config()),
                "remove_caller" => Membership::find(tx.conn(), membership)?.destroy(tx),
                "sign_out" => Session::find(tx.conn(), session)?.destroy(tx),
                _ => unreachable!(),
            }
        });
        assert!(db.sink.take().iter().any(|event| matches!(event.as_broadcast(), Some(crate::broadcasts::Broadcast::Cable { payload, .. }) if payload["huddleInvitation"]["eventType"] == "huddle_ended")));
        db.write(move |tx| publish_ring_with_policy(tx, &request, None));
        assert!(!db.sink.take().iter().any(|event| event.as_broadcast().is_some()), "{operation} suppressed={suppressed}: ring restarted after ended frame");
    }
}
