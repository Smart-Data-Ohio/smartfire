//! Queued old invitations cannot become a retry of a newer Rails generation.
use crate::models::huddle_grant::HuddleGrant;
use crate::models::huddle_invitations::{RingRequest, publish_queued_ring};
use crate::models::room_delete::HuddleConfig;
use crate::tests::TestDb;
use crate::{ActivityItem, Job, Membership, Session, Timestamp};
use serde_json::Value;

fn frames(events: &[crate::Event], recipient: i64) -> Vec<Value> {
    events.iter().filter_map(|event| match event.as_broadcast()? {
        crate::broadcasts::Broadcast::Cable {stream, payload} if *stream == format!("user_{recipient}_activity") => Some(payload.clone()),
        _ => None,
    }).collect()
}

// RecordingSink does not persist jobs. Materialize its emitted RingRequests
// between operations so the production enqueue/supersession and handler identity
// checks run against durable rows; the app matrix tests real adapter persistence.
fn queued(db: &TestDb, events: &[crate::Event]) -> Vec<(i64, RingRequest)> {
    events.iter().filter_map(|event| event.as_job::<RingRequest>()).map(|request| {
        let saved = request.clone();
        let id = db.write(move |tx| Ok(tx.conn().query_row(
            "INSERT INTO background_jobs(arguments,job_class,queue_name,run_at,created_at,updated_at) VALUES(?,?,'default',?,?,?) RETURNING id",
            rusqlite::params![serde_json::to_value(saved).unwrap(),RingRequest::CLASS,tx.now(),tx.now(),tx.now()], |row| row.get(0))?));
        (id, request)
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
            let first = queued(&db, &db.sink.take()).into_iter().find(|(_,r)| r.recipient_id==recipient).unwrap();
            let initial = first.0;
            db.write(move |tx| {publish_queued_ring(tx, initial)?; tx.conn().execute("DELETE FROM background_jobs WHERE id=?", [initial])?; Ok(())});
            assert_eq!(serde_json::json!(frames(&db.sink.take(), recipient)), case["phases"][0]["frames"]);
            let item = first.1.invitation["activityItemId"].as_i64().unwrap();
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
            let mut backlog = queued(&db, &updates);
            // Retain an initial worker payload too; neither stale unread nor
            // stale handled/missed jobs may acquire the new retry's identity.
            backlog.push(first);
            db.travel(181);
            let retry_session = if operation == "same_attempt_new_grant" {
                db.write(move |tx| Ok(Session::start(tx, caller, None, None)?.id))
            } else { session };
            let retry = db.write(move |tx| HuddleGrant::issue(tx, retry_session, member, room, &config()));
            assert_eq!(retry.id == grant.id, operation != "same_attempt_new_grant");
            let fresh = queued(&db, &db.sink.take());
            assert_eq!(fresh.len(), 1, "{}", case["operation"]);
            let expected_time: jiff::Timestamp = case["phases"][2]["invited_at"].as_str().unwrap().parse().unwrap();
            assert_eq!(fresh[0].1.invited_at, Some(expected_time.as_microsecond()));
            if newest_first { backlog.splice(0..0, fresh); } else { backlog.extend(fresh); }
            for (id, _) in backlog { db.write(move |tx| publish_queued_ring(tx, id)); }
            assert_eq!(serde_json::json!(frames(&db.sink.take(), recipient)), case["phases"][2]["frames"], "{} newest_first={newest_first}", case["operation"]);
        }
    }
}

#[test]
fn a9_room_mutes_suppress_huddle_invitations_until_expiry() {
    for banner_only in [false, true] {
        for mute in ["membership", "forever", "minutes15"] {
            let db = TestDb::new();
            db.clock
                .travel_to(Timestamp::parse_db("2035-01-01 12:00:00").unwrap());
            let caller = crate::fixtures::identify("david");
            let recipient = crate::fixtures::identify("jason");
            let room = crate::fixtures::identify("david_and_jason");
            let (session, member) = db.write(move |tx| {
                let until = if mute == "forever" { Value::Null } else { serde_json::json!(tx.now().since(jiff::SignedDuration::from_secs(900)).jiff()) };
                let mut preferences = serde_json::json!({"huddle_invitations": !banner_only});
                if mute == "membership" {
                    tx.conn().execute("UPDATE memberships SET involvement='nothing' WHERE room_id=? AND user_id=?", rusqlite::params![room,recipient])?;
                } else {
                    preferences["room_mute_until"] = serde_json::json!({room.to_string(): until});
                }
                tx.conn().execute("UPDATE users SET inbox_preferences=? WHERE id=?", rusqlite::params![preferences.to_string(),recipient])?;
                Ok((Session::start(tx, caller, None, None)?.id, Membership::find_by_room_and_user(tx.conn(), room, caller)?.unwrap().id))
            });
            let config = || HuddleConfig {
                api_secret: Some("a9-huddle-fixture".into()),
                admin_configured: false,
            };
            db.sink.take();
            db.write(move |tx| HuddleGrant::issue(tx, session, member, room, &config()));
            assert!(
                frames(&db.sink.take(), recipient).is_empty(),
                "{mute}, banner_only={banner_only}"
            );
            assert_eq!(db.read(|conn| Ok(conn.query_row("SELECT COUNT(*) FROM activity_items WHERE user_id=? AND source_type='HuddleGrant'", [recipient], |row| row.get::<_,i64>(0))?)), 0);
            if mute == "minutes15" {
                db.travel(900);
                db.write(move |tx| HuddleGrant::issue(tx, session, member, room, &config()));
                assert_eq!(
                    frames(&db.sink.take(), recipient).len(),
                    1,
                    "invitation resumes at expiry"
                );
            }
        }
    }
}

#[test]
fn a9_a_mute_suppresses_an_already_queued_huddle_invitation() {
    let db = TestDb::new();
    let caller = crate::fixtures::identify("david");
    let recipient = crate::fixtures::identify("jason");
    let room = crate::fixtures::identify("david_and_jason");
    let (session, member) = db.write(move |tx| {
        Ok((
            Session::start(tx, caller, None, None)?.id,
            Membership::find_by_room_and_user(tx.conn(), room, caller)?
                .unwrap()
                .id,
        ))
    });
    db.sink.take();
    db.write(move |tx| {
        HuddleGrant::issue(
            tx,
            session,
            member,
            room,
            &HuddleConfig {
                api_secret: Some("a9-huddle-fixture".into()),
                admin_configured: false,
            },
        )
    });
    let request = db
        .sink
        .take()
        .iter()
        .find_map(|event| {
            event
                .as_job::<RingRequest>()
                .filter(|request| request.recipient_id == recipient)
        })
        .unwrap();
    db.write(move |tx| {
        let until = tx.now().since(jiff::SignedDuration::from_secs(900)).jiff();
        tx.conn().execute(
            "UPDATE users SET inbox_preferences=? WHERE id=?",
            rusqlite::params![
                serde_json::json!({"room_mute_until":{room.to_string():until}}).to_string(),
                recipient
            ],
        )?;
        crate::models::huddle_invitations::publish_ring_with_policy(tx, &request, None)
    });
    assert!(frames(&db.sink.take(), recipient).is_empty());
}

#[test]
fn a9_room_mutes_suppress_huddle_join_notices_until_expiry() {
    for banner_only in [false, true] {
        for mute in ["forever", "minutes15"] {
            let db = TestDb::new();
            db.clock
                .travel_to(Timestamp::parse_db("2035-01-01 12:00:00").unwrap());
            let caller = crate::fixtures::identify("david");
            let recipient = crate::fixtures::identify("jason");
            let room = crate::fixtures::identify("david_and_jason");
            let mut grant = db.write(move |tx| {
                let until = if mute == "forever" {
                    Value::Null
                } else {
                    serde_json::json!(tx.now().since(jiff::SignedDuration::from_secs(900)).jiff())
                };
                tx.conn().execute(
                    "UPDATE users SET inbox_preferences=? WHERE id=?",
                    rusqlite::params![
                        serde_json::json!({"huddle_invitations": !banner_only, "room_mute_until": {room.to_string(): until}}).to_string(),
                        recipient
                    ],
                )?;
                let session = Session::start(tx, caller, None, None)?.id;
                let member = Membership::find_by_room_and_user(tx.conn(), room, caller)?.unwrap().id;
                let mut grant = HuddleGrant::issue(tx, session, member, room, &HuddleConfig {
                    api_secret: Some("a9-huddle-fixture".into()),
                    admin_configured: false,
                })?;
                grant.record_seen(tx)?;
                crate::models::huddle_notices::notify_join(tx, grant.id)?;
                Ok(grant)
            });
            let events = db.sink.take();
            let notices = |events: &[crate::Event]| {
                events.iter().filter_map(|event| match event.as_broadcast()? {
                    crate::broadcasts::Broadcast::Cable {stream, payload} if *stream == format!("user_{recipient}_huddle_notices") => Some(payload.clone()),
                    _ => None,
                }).collect::<Vec<_>>()
            };
            assert!(notices(&events).is_empty(), "{mute}, banner_only={banner_only}");
            assert!(events.iter().all(|event| event.as_job::<crate::models::huddle_notices::PushRequest>().is_none()));
            if mute == "minutes15" {
                db.travel(900);
                db.write(move |tx| {
                    grant.record_seen(tx)?;
                    crate::models::huddle_notices::notify_join(tx, grant.id)
                });
                let events = db.sink.take();
                assert_eq!(notices(&events).len(), 1, "join notice resumes at expiry");
                assert_eq!(notices(&events)[0]["huddleJoinNotice"]["eventType"], "huddle_joined");
            }
        }
    }
}

#[test]
fn a9_room_mutes_suppress_leave_toasts_and_keep_ended_cleanup() {
    let db = TestDb::new();
    let caller = crate::fixtures::identify("david");
    let recipient = crate::fixtures::identify("jason");
    let room = crate::fixtures::identify("david_and_jason");
    let (caller_grant, viewer_grant) = db.write(move |tx| {
        let config = HuddleConfig {
            api_secret: Some("a9-huddle-fixture".into()),
            admin_configured: false,
        };
        let mut grants = Vec::new();
        for user in [caller, recipient] {
            let session = Session::start(tx, user, None, None)?.id;
            let member = Membership::find_by_room_and_user(tx.conn(), room, user)?.unwrap().id;
            let mut grant = HuddleGrant::issue(tx, session, member, room, &config)?;
            grant.record_seen(tx)?;
            grants.push(grant);
        }
        let until = tx.now().since(jiff::SignedDuration::from_secs(900)).jiff();
        tx.conn().execute(
            "UPDATE users SET inbox_preferences=? WHERE id=?",
            rusqlite::params![serde_json::json!({"room_mute_until": {room.to_string(): until}}).to_string(), recipient],
        )?;
        Ok((grants.remove(0), grants.remove(0)))
    });
    db.sink.take();
    let leaving = caller_grant.clone();
    db.write(move |tx| crate::models::huddle_notices::notify_leave(tx, &leaving));
    let notices = |events: &[crate::Event]| {
        events.iter().filter_map(|event| match event.as_broadcast()? {
            crate::broadcasts::Broadcast::Cable {stream, payload} if *stream == format!("user_{recipient}_huddle_notices") => Some(payload.clone()),
            _ => None,
        }).collect::<Vec<_>>()
    };
    assert!(notices(&db.sink.take()).is_empty(), "muted active call emits no leave toast");
    let joining = caller_grant.clone();
    db.write(move |tx| crate::models::huddle_notices::notify_join(tx, joining.id));
    assert!(notices(&db.sink.take()).is_empty(), "muted active call emits no join toast");
    db.write(move |tx| {
        tx.conn().execute("UPDATE huddle_grants SET last_seen_at=NULL WHERE id=?", [viewer_grant.id])?;
        crate::models::huddle_notices::notify_leave(tx, &caller_grant)
    });
    let events = db.sink.take();
    assert_eq!(notices(&events).len(), 1, "ended frame still dismisses an existing banner");
    assert_eq!(notices(&events)[0]["huddleJoinNotice"]["eventType"], "huddle_ended");
}
