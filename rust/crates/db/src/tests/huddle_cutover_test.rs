//! Complete formerly deferred PushGating scenarios, including the real source lifecycle.
use super::*;
use crate::models::{
    huddle_grant::HuddleGrant,
    huddle_invitations,
    huddle_notices::{self, PushRequest},
    notification_push::{HuddleInvitationDeliveryJob, HuddlePushRequest, enqueue_huddle_request},
    room_delete::HuddleConfig,
};
use crate::{ActivityItem, DndAllowedUser, Membership, PushSubscription, Timestamp};
use serde_json::{Value, json};

fn state(t: &TestDb, items: &[ActivityItem]) -> Value {
    json!(
        items
            .iter()
            .map(|item| {
                let saved = t.read(|c| ActivityItem::find(c, item.id));
                json!([saved.user_id, saved.event_type, saved.unread()])
            })
            .collect::<Vec<_>>()
    )
}
fn push(t: &TestDb, items: &[ActivityItem]) -> Vec<Value> {
    t.sink.take();
    for item in items {
        let id = item.id;
        t.write(move |tx| huddle_notices::push_invitation(tx, id));
    }
    let requests = t
        .events()
        .iter()
        .filter_map(|e| e.as_job::<PushRequest>())
        .collect::<Vec<_>>();
    t.sink.take();
    for request in requests {
        let request: HuddlePushRequest =
            serde_json::from_value(serde_json::to_value(request).unwrap()).unwrap();
        t.write(move |tx| enqueue_huddle_request(tx, request));
    }
    t.events()
        .iter()
        .filter_map(|e| e.as_job::<HuddleInvitationDeliveryJob>())
        .map(|job| {
            let users = t
                .read(|c| PushSubscription::for_ids(c, &job.subscription_ids))
                .into_iter()
                .map(|s| s.user_id)
                .collect::<Vec<_>>();
            json!({"payload":job.payload,"users":users})
        })
        .collect()
}
fn run(name: &str) {
    let vectors: Value =
        serde_json::from_str(include_str!("../../../../vectors/ws17_huddle_cutover.json")).unwrap();
    let row = vectors["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["name"] == name)
        .unwrap();
    let t = TestDb::with_clock(
        TestClock::frozen_at(Timestamp::parse_db(vectors["now"].as_str().unwrap()).unwrap()),
        4,
    );
    let sql = row["setup_sql"].as_str().unwrap().to_owned();
    t.write(move |tx| {
        tx.conn().execute_batch(&sql)?;
        Ok(())
    });
    let (room, caller, session) = (
        row["room"].as_i64().unwrap(),
        row["caller"].as_i64().unwrap(),
        row["session"].as_i64().unwrap(),
    );
    let recipients = row["recipients"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_i64().unwrap())
        .collect::<Vec<_>>();
    if name == "group_quiet" {
        assert!(
            t.read(|c| crate::UserStatusSettings::find(c, recipients[0]))
                .dnd_enabled
        );
        assert!(
            t.read(|c| crate::UserStatusSettings::find(c, recipients[1]))
                .quiet_hours_enabled
        );
    }
    let grant = t.write(move |tx| {
        let membership = Membership::find_by_room_and_user(tx.conn(), room, caller)?.unwrap();
        HuddleGrant::issue(
            tx,
            session,
            membership.id,
            room,
            &HuddleConfig {
                api_secret: Some("fixture-cutover-huddle-secret".into()),
                admin_configured: false,
            },
        )
    });
    let items = recipients
        .iter()
        .map(|&recipient| {
            t.read(|c| ActivityItem::find_by_user_and_source(c, recipient, "HuddleGrant", grant.id))
                .expect("real issuance must create the recipient's invitation")
        })
        .collect::<Vec<_>>();
    assert_eq!(
        state(&t, &items),
        row["initial"],
        "{name}: real issuance records every recipient"
    );
    let recipient = recipients[0];
    // push_gating_test.rb:145 enables DND after issuance; :165 already has
    // both quiet policies enabled in setup_sql before the group grant is issued.
    if name == "dnd_allowed" {
        t.write(move |tx| {
            tx.conn()
                .execute("UPDATE users SET dnd_enabled=1 WHERE id=?", [recipient])?;
            Ok(())
        });
    }
    let mut deliveries = push(&t, &items);
    assert_eq!(
        json!(deliveries),
        row["first_deliveries"],
        "{name}: policy on actual invitations"
    );
    if name == "dnd_allowed" {
        t.write(move |tx| DndAllowedUser::create(tx, recipient, caller));
        deliveries.extend(push(&t, &items));
    } else {
        let ids = items.iter().map(|i| i.id).collect::<Vec<_>>();
        t.write(move |tx| {
            for id in ids {
                tx.conn().execute(
                    "UPDATE activity_items SET created_at=? WHERE id=?",
                    (tx.now().ago(jiff::SignedDuration::from_secs(46)), id),
                )?;
            }
            Ok(())
        });
        t.write(|tx| huddle_invitations::resolve_overdue(tx, None));
        assert_eq!(
            state(&t, &items),
            row["final"],
            "{name}: suppressed pushes retain every missed call"
        );
    }
    assert_eq!(
        json!(deliveries),
        row["deliveries"],
        "{name}: complete payloads and subscription recipients"
    );
}
#[test]
fn huddle_push_honors_dnd_with_a_starred_caller_exception() {
    run("dnd_allowed");
}
#[test]
fn group_huddle_push_skips_dnd_and_quiet_hours_but_records_all_missed_calls() {
    run("group_quiet");
}
