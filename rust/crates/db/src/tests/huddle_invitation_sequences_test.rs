//! Persist every step of the original invitation declarations, including devices,
//! retries, handled history, timeouts and the real leave/revoke callbacks.
use crate::models::huddle_grant::HuddleGrant;
use crate::models::huddle_invitations::{self, RingRequest};
use crate::models::huddle_notices::PushInvitationJob;
use crate::models::room_delete::HuddleConfig;
use crate::tests::{TestDb, huddle_invitations_test, huddle_notices_test, huddle_revocation_test};
use crate::{ActivityItem, Event, Membership, Timestamp};
use rusqlite::params;
use serde_json::{Value, json};

fn run(number: i64, test: &str) {
    if std::env::var("WS13B_INVITATION_CHILD").as_deref() != Ok(test) {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                &format!("tests::huddle_invitation_sequences_test::{test}"),
                "--nocapture",
            ])
            .env("WS13B_INVITATION_CHILD", test)
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
        serde_json::from_str(include_str!("huddle_invitation_sequence_vectors.json")).unwrap();
    assert_eq!(vectors["cases"].as_array().unwrap().len(), 38);
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
        tx.conn().execute("DELETE FROM sqlite_sequence WHERE name IN ('huddle_grants','activity_items','huddle_cleanups')", [])?;
        Ok(())
    });
    db.sink.take();
    for expected in case["results"].as_array().unwrap() {
        let op = expected["operation"].clone();
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
        let jobs = events.iter().filter_map(|e| match e {
            Event::Job(j) if j.class == "Huddle::PushInvitationJob" => Some(json!({"class":j.class, "id":e.as_job::<PushInvitationJob>().unwrap().activity_item_id, "delayed":j.wait.is_some()})),
            _ => None,
        }).collect::<Vec<_>>();
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
declaration!(one_to_one_recipient, 1);
declaration!(no_delayed_jobs, 2);
declaration!(quiet_ring_keeps_item, 3);
declaration!(off_and_hidden_recipients, 4);
// The neighboring-message mention assertion belongs to WS8/WS12 and remains partial.
declaration!(disabled_items_banner_and_timeout, 5);
declaration!(disabled_items_multiple_devices_window, 6);
declaration!(disabled_items_same_device, 7);
declaration!(channel_no_invitation, 8);
declaration!(voice_no_invitation, 9);
declaration!(connected_peer_not_rung, 10);
declaration!(quiet_peer_rung, 11);
declaration!(same_grant_retry_refreshes_item, 12);
declaration!(second_device_dedup, 13);
declaration!(missed_dedup, 14);
declaration!(handled_dedup, 15);
declaration!(old_item_retry, 16);
declaration!(old_handled_item_retry, 17);
declaration!(old_missed_item_retry, 18);
declaration!(revoke_ends_ring, 19);
declaration!(leave_ends_ring, 20);
declaration!(quiet_revoke_no_ended, 21);
declaration!(joined_peer_no_ended, 22);
declaration!(disabled_items_banner_ended, 23);
declaration!(old_disabled_banner_no_ended, 24);
declaration!(group_starter_leaves_no_ended, 25);
declaration!(group_starter_revokes_no_ended, 26);
declaration!(group_last_leave_ends_ring, 27);
declaration!(group_remaining_ring_times_out, 28);
declaration!(new_device_retry_reuses_unhandled, 29);
declaration!(new_device_retry_after_handled, 30);
declaration!(original_device_owned_item_priority, 31);
declaration!(attempt_ten_minute_boundary, 32);
declaration!(join_handles_invitation, 33);
declaration!(late_join_handles_missed, 34);
declaration!(bot_only_peer_no_ring, 35);
declaration!(group_every_human_rung, 36);
declaration!(group_bot_and_off_skipped, 37);
declaration!(removed_member_revoked_and_not_rung, 38);
