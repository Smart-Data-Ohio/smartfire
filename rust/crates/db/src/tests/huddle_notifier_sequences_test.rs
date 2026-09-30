//! Persist every step of the original JoinNotifier declarations, including devices,
//! retries, handled history, timeouts and the real leave/revoke callbacks.
use crate::models::huddle_grant::{HuddleGrant, JoinNoticeJob, PresenceJob};
use crate::models::huddle_invitations::{self, RingRequest};
use crate::models::huddle_notices::{self, PushInvitationJob, PushRequest};
use crate::models::room_delete::HuddleConfig;
use crate::tests::{TestDb, huddle_invitations_test, huddle_notices_test, huddle_revocation_test};
use crate::{ActivityItem, Event, Membership, Timestamp};
use rusqlite::params;
use serde_json::{Value, json};

fn run(number: i64, test: &str) {
    if std::env::var("WS13B_NOTIFIER_CHILD").as_deref() != Ok(test) {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                &format!("tests::huddle_notifier_sequences_test::{test}"),
                "--nocapture",
            ])
            .env("WS13B_NOTIFIER_CHILD", test)
            .env("LIVEKIT_API_KEY", "ws13b-fixture-api-key")
            .env("LIVEKIT_API_SECRET", "ws13b-fixture-api-secret")
            .env_remove("LIVEKIT_URL")
            .env_remove("LIVEKIT_INTERNAL_URL")
            .env_remove("LIVEKIT_GATEWAY_SECRET")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    let vectors: Value =
        serde_json::from_str(include_str!("huddle_notifier_sequence_vectors.json")).unwrap();
    assert_eq!(vectors["cases"].as_array().unwrap().len(), 33);
    let case = vectors["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["number"] == number)
        .unwrap();
    let db = TestDb::new();
    let base = vectors["now"].as_i64().unwrap();
    db.clock.travel_to(Timestamp::from_second(base));
    let input = case["input"].clone();
    db.write(move |tx| {
        huddle_notices_test::load(tx, &input)?;
        for row in input["sessions"].as_array().unwrap() { huddle_notices_test::insert(tx, "sessions", row)?; }
        tx.conn().execute("DELETE FROM push_subscriptions", [])?;
        for row in input["subscriptions"].as_array().unwrap() { huddle_notices_test::insert(tx, "push_subscriptions", row)?; }
        tx.conn().execute("DELETE FROM sqlite_sequence WHERE name IN ('huddle_grants','activity_items','huddle_cleanups')", [])?;
        Ok(())
    });
    db.sink.take();
    let mut pending = Vec::new();
    for expected in case["results"].as_array().unwrap() {
        let op = expected["operation"].clone();
        let pending_jobs = if op["op"] == "perform_joins" {
            std::mem::take(&mut pending)
        } else {
            Vec::new()
        };
        let actual = if op["op"] == "at" {
            db.clock.travel_to(Timestamp::from_second(
                base + op["seconds"].as_i64().unwrap(),
            ));
            Value::Null
        } else {
            db.write(move |tx| {
                let id = op["id"].as_i64().unwrap_or_default();
                let user = op["user"]
                    .as_str()
                    .map(crate::fixtures::identify)
                    .unwrap_or_default();
                let config = HuddleConfig {
                    api_secret: Some("ws13b-fixture-api-secret".into()),
                    admin_configured: false,
                };
                match op["op"].as_str().unwrap() {
                    "issue" => HuddleGrant::issue(
                        tx,
                        op["session_id"].as_i64().unwrap(),
                        op["membership_id"].as_i64().unwrap(),
                        op["room_id"].as_i64().unwrap(),
                        &config,
                    )
                    .map(|g| json!(g.id)),
                    "join" => huddle_notices::notify_join(tx, id).map(|_| Value::Null),
                    "resolve_ring" => {tx.conn().execute("UPDATE activity_items SET event_type='huddle_missed' WHERE user_id=? AND event_type='huddle_started'", [user])?; Ok(Value::Null)},
                    "status" => {tx.conn().execute("UPDATE users SET status=? WHERE id=?", params![if op["value"] == "deactivated" {1} else {0}, user])?; Ok(Value::Null)},
                    "record_seen" => HuddleGrant::find_by_id(tx.conn(), id)?.unwrap().record_seen(tx).map(|_| Value::Null),
                    "perform_joins" => { for id in pending_jobs { huddle_notices::notify_join(tx, id)?; } Ok(Value::Null) },
                    "seen" => {
                        tx.conn().execute(
                            "UPDATE huddle_grants SET last_seen_at=? WHERE id=?",
                            params![
                                tx.now().ago(jiff::SignedDuration::from_secs(
                                    op["age"].as_i64().unwrap()
                                )),
                                id
                            ],
                        )?;
                        Ok(Value::Null)
                    }
                    "inbox" => {
                        tx.conn().execute(
                            "UPDATE users SET inbox_preferences=? WHERE id=?",
                            params![json!({"huddle_invitations":op["value"]}).to_string(), user],
                        )?;
                        Ok(Value::Null)
                    }
                    "involvement" => {
                        tx.conn().execute(
                            "UPDATE memberships SET involvement=? WHERE room_id=9001 AND user_id=?",
                            params![op["value"].as_str(), user],
                        )?;
                        Ok(Value::Null)
                    }
                    "missed" | "handled" => {
                        let item_id: i64 = tx.conn().query_row(
                            "SELECT id FROM activity_items WHERE user_id=? ORDER BY id LIMIT 1",
                            [user],
                            |r| r.get(0),
                        )?;
                        let item = ActivityItem::find(tx.conn(), item_id)?;
                        if op["op"] == "handled" {
                            item.mark_handled(tx)?;
                        } else {
                            ActivityItem::refresh_unread(
                                tx,
                                user,
                                &item.source_type,
                                item.source_id,
                                "huddle_missed",
                            )?;
                        }
                        Ok(Value::Null)
                    }
                    "resolve" => huddle_invitations::resolve_overdue(tx, None).map(|_| Value::Null),
                    "revoke" => HuddleGrant::find_by_id(tx.conn(), id)?
                        .unwrap()
                        .revoke(tx, false, &config)
                        .map(|_| Value::Null),
                    "leave" => HuddleGrant::find_by_id(tx.conn(), id)?
                        .unwrap()
                        .mark_out_of_call(tx, None)
                        .map(|changed| json!(changed)),
                    "destroy_member" => Membership::find(tx.conn(), id)?
                        .destroy(tx)
                        .map(|_| Value::Null),
                    _ => panic!("unknown operation {op}"),
                }
            })
        };
        assert_eq!(
            actual, expected["value"],
            "{} {}",
            case["title"], expected["operation"]
        );
        let events = db.events();
        let jobs = events
            .iter()
            .filter_map(|e| {
                let Event::Job(job) = e else {
                    return None;
                };
                let id = if let Some(job) = e.as_job::<PushInvitationJob>() {
                    job.activity_item_id
                } else if let Some(job) = e.as_job::<JoinNoticeJob>() {
                    pending.push(job.grant_id);
                    job.grant_id
                } else if let Some(job) = e.as_job::<PresenceJob>() {
                    job.grant_id
                } else {
                    return None;
                };
                Some(json!({"class":job.class,"id":id,"delayed":job.wait.is_some()}))
            })
            .collect::<Vec<_>>();
        let mut pushes = Vec::new();
        for request in events.iter().filter_map(|e| e.as_job::<PushRequest>()) {
            if let Some(delivery) =
                db.write(move |tx| huddle_notices::prepare_push(tx, &request, true))
            {
                pushes.push(json!({"payload":delivery.payload,"subscription_ids":delivery.subscriptions.iter().map(|s| s.id).collect::<Vec<_>>()}));
            }
        }
        // Every issue side effect must be immediate, including other job classes.
        assert!(
            events
                .iter()
                .all(|e| !matches!(e, Event::Job(j) if j.wait.is_some()))
        );
        for request in events.iter().filter_map(|e| e.as_job::<RingRequest>()) {
            let sound = case["sound_allowed"].as_bool().unwrap();
            db.write(move |tx| huddle_invitations::publish_ring(tx, &request, sound));
        }
        let broadcasts = db
            .events()
            .iter()
            .filter_map(|e| match e.as_broadcast()? {
                crate::broadcasts::Broadcast::Cable { stream, payload } => {
                    Some(json!({"stream":stream,"payload":payload}))
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        for (field, mut actual) in [
            (
                "items",
                db.read(|conn| huddle_invitations_test::snapshot(conn, "activity_items")),
            ),
            (
                "grants",
                db.read(|conn| huddle_invitations_test::snapshot(conn, "huddle_grants")),
            ),
            ("pushes", json!(pushes)),
            ("throttle", db.read(|conn| {
                let mut statement = conn.prepare("SELECT id,last_huddle_join_push_at FROM memberships WHERE room_id=9001 ORDER BY id")?;
                Ok(json!(statement.query_map([], |r| Ok(json!({"id":r.get::<_,i64>(0)?,"last_huddle_join_push_at":r.get::<_,Option<String>>(1)?})))?.collect::<std::result::Result<Vec<_>,_>>()?))
            })),
            ("jobs", json!(jobs)),
            ("broadcasts", json!(broadcasts)),
        ] {
            let mut expected_value = expected[field].clone();
            huddle_revocation_test::normalize(&mut actual);
            huddle_revocation_test::normalize(&mut expected_value);
            assert_eq!(
                actual, expected_value,
                "{} {} {field}",
                case["title"], expected["operation"]
            );
        }
        db.sink.take();
    }
}

macro_rules! declaration {
    ($test:ident, $number:literal) => {
        #[test]
        fn $test() {
            run($number, stringify!($test));
        }
    };
}
declaration!(insider_and_joiner, 1);
declaration!(non_member_receives_nothing, 2);
declaration!(outsider_banner_and_push, 3);
declaration!(group_insider_and_outsider, 4);
// Exact query/preload counts remain open; this verifies fan-out results only.
declaration!(group_fanout_results, 5);
declaration!(channel_insider_only, 6);
declaration!(channel_outsider_silent, 7);
declaration!(voice_insider_only, 8);
declaration!(inactive_and_bot_viewers_silent, 9);
declaration!(bot_join_silent, 10);
declaration!(second_device_no_join_job, 11);
declaration!(two_devices_before_job_notify_once, 12);
declaration!(join_job_after_leave_silent, 13);
declaration!(live_ring_suppresses_join, 14);
declaration!(stale_ring_allows_join, 15);
declaration!(off_and_hidden_outsiders_silent, 16);
declaration!(disabled_invitations_banner_without_push, 17);
declaration!(muted_room_banner_without_push, 18);
declaration!(recent_revoke_rejoin, 19);
declaration!(disconnect_clears_rejoin_mark, 20);
declaration!(quiet_revoke_no_rejoin_mark, 21);
declaration!(six_second_revoke_no_rejoin_mark, 22);
declaration!(ten_second_revoke_no_rejoin_mark, 23);
declaration!(leave_toasts_insider, 24);
declaration!(revoke_toasts_insider, 25);
declaration!(group_leave_updates_outsider, 26);
declaration!(channel_leave_outsider_silent, 27);
declaration!(last_dm_leave_dismisses_banner, 28);
declaration!(last_channel_leave_silent, 29);
declaration!(never_seen_leave_silent, 30);
declaration!(quiet_revoke_silent, 31);
declaration!(another_device_stays_no_leave, 32);
declaration!(leave_after_revoke_no_duplicate, 33);
