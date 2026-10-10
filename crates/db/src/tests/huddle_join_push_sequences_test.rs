//! All thirteen JoinPusher declarations through the real WS17 policy/durable adapter.
use crate::models::huddle_notices::{PushKind, PushPayload, PushRequest};
use crate::models::notification_push::{self, HuddleJoinDeliveryJob, HuddlePushRequest};
use crate::models::notification_policy::dnd_exceptions_for;
use crate::{DndAllowedUser, Job, NotificationKind, NotificationPolicy, UserStatusSettings};
use crate::tests::TestDb;
use crate::{Membership, Timestamp};
use rusqlite::params;
use serde_json::{Value, json};

fn run(number: i64) {
    let vectors: Value =
        serde_json::from_str(include_str!("huddle_join_push_sequence_vectors.json")).unwrap();
    assert_eq!(vectors["cases"].as_array().unwrap().len(), 13);
    let case = vectors["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["number"] == number)
        .unwrap();
    let clock = crate::time::TestClock::new();
    clock.travel_to(Timestamp::from_second(vectors["now"].as_i64().unwrap()));
    let db = TestDb::with_clock(clock, 4);
    let input = &case["input"];
    let member = input["membership_id"].as_i64().unwrap();
    let user = input["recipient_id"].as_i64().unwrap();
    let room = input["room_id"].as_i64().unwrap();
    let request = PushRequest {
        kind: PushKind::HuddleJoin,
        recipient_id: user,
        sender_id: input["sender_id"].as_i64().unwrap(),
        room_id: room,
        room_membership_id: Some(member),
        payload: PushPayload {
            title: "David joined your huddle".into(),
            body: "Join from the conversation".into(),
            path: format!("/rooms/{room}"),
            tag: format!("huddle-{room}"),
        },
    };
    let initial = input.clone();
    db.write(move |tx| {
        tx.conn().execute("UPDATE memberships SET involvement=?,connected_at=?,last_huddle_join_push_at=NULL WHERE id=?",params![initial["involvement"].as_str(),initial["connected_at"].as_str().and_then(Timestamp::parse_db),member])?;
        tx.conn().execute("UPDATE users SET inbox_preferences=? WHERE id=?",params![initial["inbox_preferences"].to_string(),user])?;
        Ok(())
    });
    for step in case["results"].as_array().unwrap() {
        let op = step["operation"].as_str().unwrap();
        let pushes = match op {
            "push" => {
                let request = request.clone();
                let sender = request.sender_id;
                let policy = db.read(|conn| {
                    let recipient = UserStatusSettings::find(conn, user)?;
                    let membership = Membership::find(conn, member)?;
                    Ok(NotificationPolicy {
                        room_id: None,
                        recipient: Some(&recipient), kind: NotificationKind::HuddleJoin,
                        room_involvement: Some(membership.involvement), thread_involvement: None,
                        mentioned: false, reply_to_recipient: false, keyword_matched: false,
                        dnd_exception: dnd_exceptions_for(conn, &[user], Some(sender))?.contains(&user),
                        now: db.now(),
                    }.push())
                });
                assert_eq!(json!(policy), step["policy_allowed"], "{} actual WS17 policy", case["title"]);
                // Replay the source's exact wire DTO through the same adapter as the registered worker.
                let wire: HuddlePushRequest = serde_json::from_value(serde_json::to_value(request).unwrap()).unwrap();
                db.sink.take();
                db.write(move |tx| notification_push::enqueue_huddle_request(tx, wire));
                db.events().iter().filter_map(|event| match event {
                    crate::Event::Job(job) if job.class == HuddleJoinDeliveryJob::CLASS => Some(json!({
                        "payload":job.arguments["payload"], "subscription_ids":job.arguments["subscription_ids"]
                    })),
                    _ => None,
                }).collect::<Vec<_>>()
            }
            "later" => {
                db.travel(660);
                Vec::new()
            }
            "connected" => {
                db.write(move |tx| Membership::find(tx.conn(), member)?.connected(tx));
                Vec::new()
            }
            "off" | "hidden" | "muted" => {
                let involvement = match op {
                    "off" => "nothing",
                    "hidden" => "invisible",
                    _ => "muted",
                };
                db.write(move |tx| {
                    tx.conn().execute(
                        "UPDATE memberships SET involvement=?,updated_at=? WHERE id=?",
                        params![involvement, tx.now(), member],
                    )?;
                    Ok(())
                });
                Vec::new()
            }
            "inbox_off" => {
                db.write(move |tx| {
                    tx.conn().execute(
                        "UPDATE users SET inbox_preferences=? WHERE id=?",
                        params![json!({"huddle_invitations":false}).to_string(), user],
                    )?;
                    Ok(())
                });
                Vec::new()
            }
            "no_subscriptions" => {
                db.write(move |tx| {
                    tx.conn()
                        .execute("DELETE FROM push_subscriptions WHERE user_id=?", [user])?;
                    Ok(())
                });
                Vec::new()
            }
            "dnd" | "star" | "quiet" | "meeting" | "ooo" | "ooo_notify" => {
                let op = op.to_owned();
                let sender = request.sender_id;
                db.write(move |tx| {
                    match op.as_str() {
                        "dnd" => { tx.conn().execute("UPDATE users SET dnd_enabled=1 WHERE id=?",[user])?; }
                        "star" => { DndAllowedUser::create(tx,user,sender)?; }
                        "quiet" => { tx.conn().execute("UPDATE users SET quiet_hours_enabled=1,quiet_hours_start_minute=540,quiet_hours_end_minute=1020 WHERE id=?",[user])?; }
                        "meeting" => {
                            tx.conn().execute("UPDATE users SET meeting_status_enabled=1,meeting_dnd_enabled=1 WHERE id=?",[user])?;
                            let busy = json!([[tx.now().ago(jiff::SignedDuration::from_secs(300)).jiff().to_string(),tx.now().since(jiff::SignedDuration::from_secs(3300)).jiff().to_string()]]);
                            tx.conn().execute("INSERT INTO calendar_meeting_caches(user_id,fetched_at,busy_intervals,created_at,updated_at) VALUES(?,?,?,?,?)",params![user,tx.now(),busy.to_string(),tx.now(),tx.now()])?;
                        }
                        "ooo" => { tx.conn().execute("UPDATE users SET ooo_until=? WHERE id=?",params![tx.now().since(jiff::SignedDuration::from_secs(86400)),user])?; }
                        "ooo_notify" => { tx.conn().execute("UPDATE users SET ooo_notify_enabled=1 WHERE id=?",[user])?; }
                        _ => unreachable!(),
                    }
                    Ok(())
                });
                Vec::new()
            }
            _ => panic!("unknown operation {op}"),
        };
        assert_eq!(db.now().as_second(), step["now"].as_i64().unwrap());
        assert_eq!(json!(pushes), step["pushes"], "{} {op}", case["title"]);
        let membership = db.read(|conn| Membership::find(conn, member));
        for (field, actual) in [
            (
                "last_huddle_join_push_at",
                json!(membership.last_huddle_join_push_at.map(|at| at.as_second())),
            ),
            ("updated_at", json!(membership.updated_at.as_second())),
        ] {
            let expected = step[field]
                .as_str()
                .and_then(Timestamp::parse_db)
                .map(|at| at.as_second());
            assert_eq!(actual, json!(expected), "{} {op} {field}", case["title"]);
        }
    }
}
macro_rules! case {
    ($name:ident,$number:literal) => {
        #[test]
        fn $name() {
            run($number);
        }
    };
}
case!(recipient_payload_and_throttle, 1);
case!(second_push_throttled, 2);
case!(eleven_minutes_push_again, 3);
case!(dnd_blocks_without_claim, 4);
case!(allowed_caller_pushes_through_dnd, 5);
case!(quiet_hours_blocks_without_claim, 6);
case!(meeting_blocks_without_claim, 7);
case!(ooo_and_notify_policy, 8);
case!(connected_does_not_claim, 9);
case!(off_hidden_scopes_do_not_claim, 10);
case!(muted_scope_does_not_claim, 11);
case!(disabled_invitations_do_not_claim, 12);
case!(empty_subscriptions_do_not_claim, 13);
