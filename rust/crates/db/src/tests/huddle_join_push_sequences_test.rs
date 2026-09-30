//! JoinPusher stateful cases at WS17's unchanged policy/delivery boundary.
use crate::models::huddle_notices::{self, PushKind, PushPayload, PushRequest};
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
                // These scopes must still suppress delivery if WS17 allows it.
                let policy = if [10, 11].contains(&number) {
                    true
                } else {
                    step["policy_allowed"].as_bool().unwrap()
                };
                db.write(move |tx|huddle_notices::prepare_push(tx,&request,policy)).map(|delivery|vec![json!({"payload":delivery.payload,"subscription_ids":delivery.subscriptions.iter().map(|s|s.id).collect::<Vec<_>>()})]).unwrap_or_default()
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
            // WS17 owns these records/decisions; the pinned policy result is an
            // explicit boundary input, not a port of Notifications::Policy.
            "dnd" | "star" | "quiet" | "meeting" | "ooo" | "ooo_notify" => Vec::new(),
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
case!(dnd_policy_input, 4);
case!(starred_policy_input, 5);
case!(quiet_hours_policy_input, 6);
case!(meeting_policy_input, 7);
case!(ooo_policy_inputs, 8);
case!(connected_does_not_claim, 9);
case!(off_hidden_scopes_do_not_claim, 10);
case!(muted_scope_does_not_claim, 11);
case!(disabled_invitations_do_not_claim, 12);
case!(empty_subscriptions_do_not_claim, 13);
