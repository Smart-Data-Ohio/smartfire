use super::*;
use crate::models::notification_push::*;
use crate::{Job, PushPayload, Timestamp};
use serde_json::{Value, json};
fn vectors() -> Value {
    serde_json::from_str(include_str!(
        "../../../../vectors/ws17_notification_push.json"
    ))
    .unwrap()
}
fn stamp(s: &str) -> Timestamp {
    Timestamp::from_jiff(s.parse().unwrap())
}
fn payload(v: &Value) -> PushPayload {
    serde_json::from_value(v.clone()).unwrap()
}

#[test]
fn ws17_event_board_and_huddle_pushers_match_50_actual_rails_source_cases() {
    let golden = vectors();
    assert_eq!(golden["reference"], &include_str!("../../../../parity/reference.sha").trim()[..8]);
    assert_eq!(golden["board_reference"], &include_str!("../../../../parity/reference.sha").trim()[..8]);
    let now = stamp(golden["now"].as_str().unwrap());
    for row in golden["rows"].as_array().unwrap() {
        let t = TestDb::with_clock(TestClock::frozen_at(now), 4);
        let sql = row["setup_sql"].as_str().unwrap().to_owned();
        t.write(move |tx| {
            tx.conn().execute_batch(&sql)?;
            Ok(())
        });
        let kind = row["kind"].as_str().unwrap();
        let name = row["name"].as_str().unwrap();
        let actual = match kind {
            "event" | "board" => {
                let row = row.clone();
                let push = t.read(move |conn| {
                    if row["kind"] == "event" {
                        event_reminder_push(conn, row["event_id"].as_i64().unwrap(), now)
                    } else {
                        board_nudge_push(conn, row["nudge_id"].as_i64().unwrap(), now)
                    }
                });
                push.map(|p|json!({"payload":p.payload,"subscriptions":p.subscriptions.iter().map(|s|s.id).collect::<Vec<_>>(),"users":p.subscriptions.iter().map(|s|s.user_id).collect::<Vec<_>>() })).into_iter().collect::<Vec<_>>()
            }
            "join" | "invitation" => {
                let row = row.clone();
                let p = if let Some(delivery) = row["deliveries"].as_array().unwrap().first() {
                    payload(&delivery["payload"])
                } else {
                    PushPayload::new(
                        "Source-owned title".into(),
                        "Source-owned body".into(),
                        format!("/rooms/{}", row["room_id"]),
                        Some(format!("huddle-{}", row["room_id"])),
                    )
                };
                t.write(move |tx| {
                    if row["kind"] == "join" {
                        enqueue_huddle_join(
                            tx,
                            row["room_id"].as_i64().unwrap(),
                            row["recipient_id"].as_i64().unwrap(),
                            row["sender_id"].as_i64().unwrap(),
                            p,
                        )
                    } else {
                        enqueue_huddle_invitation(
                            tx,
                            row["room_id"].as_i64().unwrap(),
                            row["recipient_id"].as_i64().unwrap(),
                            row["sender_id"].as_i64().unwrap(),
                            p,
                        )
                    }
                });
                t.events().iter().filter_map(|e|match e {
      Event::Job(j) if matches!(j.class,HuddleJoinDeliveryJob::CLASS|HuddleInvitationDeliveryJob::CLASS)=>{
       let p=j.arguments["payload"].clone();let ids:Vec<i64>=serde_json::from_value(j.arguments["subscription_ids"].clone()).unwrap();let subs=t.read(move|c|crate::PushSubscription::for_ids(c,&ids));Some(json!({"payload":p,"subscriptions":subs.iter().map(|s|s.id).collect::<Vec<_>>(),"users":subs.iter().map(|s|s.user_id).collect::<Vec<_>>() }))
      },_=>None
    }).collect::<Vec<_>>()
            }
            _ => panic!("{kind}"),
        };
        let expected=row["deliveries"].as_array().unwrap().iter().map(|d|json!({"payload":d["payload"],"subscriptions":d["subscriptions"],"users":d["users"]})).collect::<Vec<_>>();
        assert_eq!(
            actual, expected,
            "{name} complete source payload/subscription scopes"
        );
        let recipient = row["recipient_id"].as_i64().unwrap();
        let room = row["room_id"].as_i64().unwrap();
        let throttle = t.read(move |c| {
            Ok(
                crate::Membership::find_by_room_and_user(c, room, recipient)?
                    .and_then(|m| m.last_huddle_join_push_at),
            )
        });
        assert_eq!(
            throttle,
            row["throttle"].as_str().map(stamp),
            "{name} throttle"
        );
    }
}
#[test]
fn ws17_huddle_join_only_one_process_claims_and_reruns_preserve_timestamp() {
    let golden = vectors();
    let now = stamp(golden["now"].as_str().unwrap());
    let t = TestDb::with_clock(TestClock::frozen_at(now), 4);
    let row = golden["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["name"] == "join_baseline")
        .unwrap()
        .clone();
    let setup = row["setup_sql"].as_str().unwrap().to_owned();
    t.write(move |tx| {
        tx.conn().execute_batch(&setup)?;
        Ok(())
    });
    let room = row["room_id"].as_i64().unwrap();
    let recipient = row["recipient_id"].as_i64().unwrap();
    let sender = row["sender_id"].as_i64().unwrap();
    let p = payload(&row["deliveries"][0]["payload"]);
    let p2 = p.clone();
    let other = t.another_process();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (a, b) = rt.block_on(async {
        tokio::join!(
            t.db.write(move |tx| enqueue_huddle_join(tx, room, recipient, sender, p)),
            other.write(move |tx| enqueue_huddle_join(tx, room, recipient, sender, p2))
        )
    });
    assert_eq!(usize::from(a.unwrap()) + usize::from(b.unwrap()), 1);
    assert_eq!(
        t.events()
            .iter()
            .filter(|e| e.as_job::<HuddleJoinDeliveryJob>().is_some())
            .count(),
        1
    );
    let now = now.since(jiff::SignedDuration::from_secs(60));
    t.clock.travel_to(now);
    let p = payload(&row["deliveries"][0]["payload"]);
    assert!(!t.write(move |tx| enqueue_huddle_join(tx, room, recipient, sender, p)));
    t.travel(600);
    let p = payload(&row["deliveries"][0]["payload"]);
    assert!(t.write(move |tx| enqueue_huddle_join(tx, room, recipient, sender, p)));
    assert_eq!(
        t.events()
            .iter()
            .filter(|e| e.as_job::<HuddleJoinDeliveryJob>().is_some())
            .count(),
        2
    );
}
