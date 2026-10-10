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

fn inherit_nothing(db: &TestDb, room: i64, recipient: i64, inbox: bool) {
    db.write(move |tx| {
        Membership::find_by_room_and_user(tx.conn(), room, recipient)?.unwrap()
            .update_involvement(tx, Some(crate::Involvement::Everything))?;
        crate::models::user::profile_settings::update(tx, recipient, crate::models::user::profile_settings::Changes {
            inbox_preferences: Some(serde_json::json!({
                "huddle_invitations": inbox,
                "default_notification_level": "nothing",
                "room_notification_levels": {room.to_string(): null}
            })),
            ..Default::default()
        })
    });
}

#[test]
fn a9_inherited_nothing_suppresses_huddle_items_and_banners() {
    use crate::models::huddle_invitations::RingRequest;
    for inbox in [true, false] {
        let db = TestDb::new();
        let (room, membership, session) = review_direct_setup(&db);
        let recipient = crate::fixtures::identify("jason");
        inherit_nothing(&db, room, recipient, inbox);
        db.sink.take();
        db.write(move |tx| HuddleGrant::issue(tx, session, membership, room, &config()));
        assert_eq!(db.read(move |c| Ok(c.query_row(
            "SELECT count(*) FROM activity_items WHERE user_id=? AND event_type='huddle_started'",
            [recipient], |r| r.get::<_, i64>(0)
        )?)), 0);
        let events = db.sink.take();
        assert!(!events.iter().filter_map(|e| e.as_job::<RingRequest>()).any(|r| r.recipient_id == recipient));
        assert!(!events.iter().any(|e| matches!(e.as_broadcast(), Some(crate::broadcasts::Broadcast::Cable { payload, .. }) if payload.get("huddleInvitation").is_some())));
    }
}

#[test]
fn a9_inherited_nothing_suppresses_already_queued_huddle_rings() {
    use crate::models::huddle_invitations::{RingRequest, publish_ring, publish_ring_with_policy};
    for inbox in [true, false] {
        let db = TestDb::new();
        let (room, membership, session) = review_direct_setup(&db);
        let recipient = crate::fixtures::identify("jason");
        if !inbox {
            db.write(move |tx| Ok(tx.conn().execute(
                "UPDATE users SET inbox_preferences=? WHERE id=?",
                rusqlite::params![r#"{"huddle_invitations":false}"#, recipient]
            )?));
        }
        db.sink.take();
        db.write(move |tx| HuddleGrant::issue(tx, session, membership, room, &config()));
        let request = db.sink.take().iter().filter_map(|e| e.as_job::<RingRequest>())
            .find(|r| r.recipient_id == recipient).unwrap();
        inherit_nothing(&db, room, recipient, inbox);
        db.sink.take();
        db.write(move |tx| {
            publish_ring_with_policy(tx, &request, None)?;
            publish_ring(tx, &request, true)
        });
        assert!(!db.sink.take().iter().any(|e| e.as_broadcast().is_some()));
    }
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
