//! Queued old invitations cannot become a retry of a newer Rails generation.
use crate::models::huddle_grant::HuddleGrant;
use crate::models::huddle_invitations::{RingRequest, publish_ring_with_policy};
use crate::models::room_delete::HuddleConfig;
use crate::tests::TestDb;
use crate::{ActivityItem, Membership, Session, Timestamp};
use serde_json::Value;

fn frames(events: &[crate::Event], recipient: i64) -> Vec<Value> {
    events.iter().filter_map(|event| match event.as_broadcast()? {
        crate::broadcasts::Broadcast::Cable {stream, payload} if *stream == format!("user_{recipient}_activity") => Some(payload.clone()),
        _ => None,
    }).collect()
}

#[test]
fn pending_invitation_generations_emit_one_rails_retry_in_either_drain_order() {
    let oracle: Value = serde_json::from_str(include_str!("huddle_ring_generations.json")).unwrap();
    for case in oracle["cases"].as_array().unwrap() {
        for newest_first in [false, true] {
            let db = TestDb::new();
            db.clock.travel_to(Timestamp::parse_db("2026-01-01 12:00:00").unwrap());
            let caller = crate::fixtures::identify("david");
            let recipient = crate::fixtures::identify("jason");
            let room = crate::fixtures::identify("david_and_jason");
            let operation = case["operation"].as_str().unwrap();
            let suppressed = operation == "suppressed_retry";
            let (session, member) = db.write(move |tx| {
                if suppressed { tx.conn().execute(r#"UPDATE users SET inbox_preferences='{"huddle_invitations":false}' WHERE id=?"#, [recipient])?; }
                Ok((Session::start(tx, caller, None, None)?.id, Membership::find_by_room_and_user(tx.conn(), room, caller)?.unwrap().id))
            });
            let config = || HuddleConfig {api_secret:Some("ws13b-review-fixture-value".into()),admin_configured:false};
            db.sink.take();
            let grant = db.write(move |tx| HuddleGrant::issue(tx, session, member, room, &config()));
            let first = db.sink.take().iter().filter_map(|e| e.as_job::<RingRequest>()).find(|r| r.recipient_id==recipient).unwrap();
            let initial = first.clone();
            db.write(move |tx| publish_ring_with_policy(tx, &initial, None));
            assert_eq!(serde_json::json!(frames(&db.sink.take(), recipient)), case["phases"][0]["frames"]);
            let item = first.invitation["activityItemId"].as_i64().unwrap();
            let mutation = operation.to_owned();
            db.write(move |tx| {
                match mutation.as_str() {
                    "handled" => { ActivityItem::find(tx.conn(), item)?.mark_handled(tx)?; }
                    "missed" => {
                        tx.conn().execute("UPDATE activity_items SET event_type='huddle_missed',updated_at=? WHERE id=?", rusqlite::params![tx.now(),item])?;
                        ActivityItem::broadcast_change(tx, recipient, item)?;
                    }
                    "ended_retry" | "suppressed_retry" => {
                        let mut grant = HuddleGrant::find_by_id(tx.conn(), grant.id)?.unwrap();
                        grant.record_seen(tx)?;
                        grant.mark_out_of_call(tx, None)?;
                    }
                    _ => {}
                }
                Ok(())
            });
            let updates = db.sink.take();
            let mut backlog: Vec<_> = updates.iter().filter_map(|e| e.as_job::<RingRequest>()).collect();
            // Retain an initial worker payload too; neither stale unread nor
            // stale handled/missed jobs may acquire the new retry's identity.
            backlog.push(first);
            db.travel(181);
            let retry_session = if operation == "same_attempt_new_grant" {
                db.write(move |tx| Ok(Session::start(tx, caller, None, None)?.id))
            } else { session };
            let retry = db.write(move |tx| HuddleGrant::issue(tx, retry_session, member, room, &config()));
            assert_eq!(retry.id == grant.id, operation != "same_attempt_new_grant");
            let fresh: Vec<_> = db.sink.take().iter().filter_map(|e| e.as_job::<RingRequest>()).collect();
            assert_eq!(fresh.len(), 1, "{}", case["operation"]);
            let expected_time: jiff::Timestamp = case["phases"][2]["invited_at"].as_str().unwrap().parse().unwrap();
            assert_eq!(fresh[0].invited_at, Some(expected_time.as_microsecond()));
            if newest_first { backlog.splice(0..0, fresh); } else { backlog.extend(fresh); }
            for request in backlog { db.write(move |tx| publish_ring_with_policy(tx, &request, None)); }
            assert_eq!(serde_json::json!(frames(&db.sink.take(), recipient)), case["phases"][2]["frames"], "{} newest_first={newest_first}", case["operation"]);
        }
    }
}
