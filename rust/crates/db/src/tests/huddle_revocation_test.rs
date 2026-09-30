//! The nine Rails HuddleRevocationTest declarations, through real lifecycle callbacks.
use crate::models::huddle_grant::HuddleGrant;
use crate::models::room_delete::HuddleConfig;
use crate::tests::{TestDb, huddle_notices_test};
use crate::{Connection, Membership, Session, Timestamp, User};
use serde_json::{Value, json};

fn normalize(value: &mut Value) {
    match value {
        Value::String(s) => {
            if let Some(at) = Timestamp::parse_db(s) {
                *s = at.to_db();
            }
        }
        Value::Array(values) => values.iter_mut().for_each(normalize),
        Value::Object(values) => values.values_mut().for_each(normalize),
        _ => (),
    }
}

fn snapshot(conn: &Connection) -> crate::Result<Value> {
    let mut result = serde_json::Map::new();
    for (key, table) in [("grants", "huddle_grants"), ("cleanups", "huddle_cleanups")] {
        let mut statement = conn.prepare(&format!("SELECT * FROM {table} ORDER BY id"))?;
        let columns = statement
            .column_names()
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>();
        let rows = statement
            .query_map([], |row| {
                let mut object = serde_json::Map::new();
                for (index, column) in columns.iter().enumerate() {
                    if column == "identity" && key == "grants" {
                        continue;
                    }
                    let value = match row.get_ref(index)? {
                        rusqlite::types::ValueRef::Null => Value::Null,
                        rusqlite::types::ValueRef::Integer(n) if column == "server_muted" => {
                            json!(n != 0)
                        }
                        rusqlite::types::ValueRef::Integer(n) => json!(n),
                        rusqlite::types::ValueRef::Text(_) if column == "identity" => {
                            let id: i64 = row.get("huddle_grant_id")?;
                            let identity: String = row.get(index)?;
                            let grant_identity: String = conn.query_row(
                                "SELECT identity FROM huddle_grants WHERE id=?",
                                [id],
                                |r| r.get(0),
                            )?;
                            assert_eq!(
                                identity, grant_identity,
                                "cleanup targeted an unrelated participant"
                            );
                            json!(format!("grant:{id}"))
                        }
                        rusqlite::types::ValueRef::Text(s) => {
                            json!(std::str::from_utf8(s).unwrap())
                        }
                        _ => panic!("unexpected revocation field"),
                    };
                    object.insert(column.clone(), value);
                }
                Ok(Value::Object(object))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        result.insert(key.into(), json!(rows));
    }
    Ok(Value::Object(result))
}

fn load(db: &TestDb, case: &Value) {
    let input = case["input"].clone();
    db.write(move |tx| {
        tx.conn().execute_batch("DELETE FROM huddle_cleanups; DELETE FROM huddle_grants; DELETE FROM sqlite_sequence WHERE name IN ('huddle_grants','huddle_cleanups');")?;
        for (table, key) in [("rooms", "rooms"), ("memberships", "memberships"), ("sessions", "sessions")] {
            for row in input[key].as_array().unwrap() {
                huddle_notices_test::insert(tx, table, row)?;
            }
        }
        let config = HuddleConfig { api_secret: Some("ws13b-fixture-api-secret".into()), admin_configured: true };
        for grant in case_grants(&input, &config, tx)? {
            assert!(grant.identity.starts_with("campfire-participant-"));
            let random = &grant.identity["campfire-participant-".len()..];
            assert_eq!(random.len(), 64);
            assert!(random.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)));
        }
        Ok(())
    });
    db.travel(1);
    db.write(|tx| {
        tx.conn()
            .execute("UPDATE huddle_grants SET last_seen_at=?", [tx.now()])?;
        Ok(())
    });
    db.sink.take();
}

fn case_grants(
    input: &Value,
    config: &HuddleConfig,
    tx: &mut crate::Tx<'_>,
) -> crate::Result<Vec<HuddleGrant>> {
    let name = input["case"].as_str().unwrap();
    let mut grants = vec![HuddleGrant::issue(tx, 7001, 9011, 9001, config)?];
    if matches!(name, "session" | "deactivate") {
        grants.push(HuddleGrant::issue(tx, 7001, 9021, 9002, config)?);
    } else if name == "ban" {
        grants.push(HuddleGrant::issue(tx, 7002, 9011, 9001, config)?);
    }
    grants.push(HuddleGrant::issue(tx, 7003, 9012, 9001, config)?);
    if name != "ban" {
        grants.push(HuddleGrant::issue(tx, 7002, 9011, 9001, config)?);
    }
    Ok(grants)
}

// Lifecycle methods read LiveKit configuration from the environment. Isolate that
// environment in a subprocess, without changing variables under concurrent tests.
fn run_case(name: &str, test: &str, rollback: bool) {
    if std::env::var("WS13B_REVOCATION_CHILD").as_deref() != Ok(test) {
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                &format!("tests::huddle_revocation_test::{test}"),
                "--nocapture",
            ])
            .env("WS13B_REVOCATION_CHILD", test)
            .env("LIVEKIT_API_KEY", "ws13b-fixture-api-key")
            .env("LIVEKIT_API_SECRET", "ws13b-fixture-api-secret")
            .env("LIVEKIT_URL", "wss://huddle.example.test")
            .env("LIVEKIT_GATEWAY_SECRET", "ws13b-fixture-gateway-secret");
        if name == "unavailable" {
            command.env_remove("LIVEKIT_INTERNAL_URL");
        } else {
            command.env("LIVEKIT_INTERNAL_URL", "ws://livekit.example.test:7880");
        }
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    let vectors: Value =
        serde_json::from_str(include_str!("huddle_revocation_vectors.json")).unwrap();
    assert_eq!(vectors["cases"].as_array().unwrap().len(), 9);
    let mut case = vectors["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == name)
        .unwrap()
        .clone();
    case["input"]["case"] = json!(name);
    let db = TestDb::new();
    db.clock
        .travel_to(Timestamp::from_second(vectors["now"].as_i64().unwrap()));
    load(&db, &case);
    let mut before = db.read(snapshot);
    let mut expected = case["before"].clone();
    normalize(&mut before);
    normalize(&mut expected);
    assert_eq!(before, expected, "{name} setup");
    if rollback {
        db.write(|tx| Ok(tx.conn().execute_batch("CREATE TRIGGER ws13b_reject_cleanup BEFORE INSERT ON huddle_cleanups BEGIN SELECT RAISE(ABORT,'WS13b intentional cleanup failure'); END;")?));
    }
    let operation = case["operation"].as_str().unwrap().to_string();
    let result = db.try_write(move |tx| match operation.as_str() {
        "membership" => Membership::find(tx.conn(), 9011)?.destroy(tx),
        "session" => Session::find(tx.conn(), 7001)?.destroy(tx),
        "ban" => User::find(tx.conn(), crate::fixtures::identify("david"))?.ban(tx),
        "deactivate" => User::find(tx.conn(), crate::fixtures::identify("david"))?.deactivate(tx),
        "room" => crate::Room::find(tx.conn(), 9001)?.begin_destroy(tx),
        _ => panic!("unknown lifecycle operation"),
    });
    let mut actual = db.read(snapshot);
    normalize(&mut actual);
    if rollback {
        assert!(result.is_err(), "cleanup failure must abort revocation");
        assert_eq!(actual, before, "partial revocation escaped rollback");
        assert!(
            db.events().is_empty(),
            "rolled-back jobs/broadcasts escaped"
        );
        db.read(|conn| {
            assert!(
                Membership::find_by_room_and_user(conn, 9001, crate::fixtures::identify("david"))?
                    .is_some()
            );
            assert!(Session::find(conn, 7001).is_ok());
            assert!(User::find(conn, crate::fixtures::identify("david"))?.is_active());
            assert!(crate::Room::find(conn, 9001)?.deleted_at.is_none());
            Ok(())
        });
    } else {
        result.unwrap();
        let mut expected = case["after"].clone();
        normalize(&mut expected);
        assert_eq!(actual, expected, "{name} persisted lifecycle");
        db.read(|conn| {
            for row in actual["grants"].as_array().unwrap() {
                let grant = HuddleGrant::find_by_id(conn, row["id"].as_i64().unwrap())?.unwrap();
                assert_eq!(
                    grant.authorized(conn)?,
                    !grant.revoked(),
                    "revoked/restored authorization"
                );
            }
            Ok(())
        });
    }
}

macro_rules! revocation_case {
    ($test:ident, $case:literal) => {
        #[test]
        fn $test() {
            run_case($case, stringify!($test), false);
        }
    };
}
revocation_case!(membership_revokes_only_target_grants, "membership");
revocation_case!(sign_out_revokes_every_room_only_for_that_session, "session");
revocation_case!(ban_revokes_after_bulk_session_deletion, "ban");
revocation_case!(
    deactivate_revokes_after_bulk_membership_and_session_deletion,
    "deactivate"
);
revocation_case!(room_deletion_uses_one_room_cleanup, "room");
revocation_case!(
    voice_member_removal_preserves_other_participants,
    "voice_membership"
);
revocation_case!(voice_room_deletion_uses_one_room_cleanup, "voice_room");
revocation_case!(deactivate_ends_voice_sessions, "voice_deactivate");
revocation_case!(
    livekit_outage_keeps_unenqueued_durable_cleanup,
    "unavailable"
);

#[test]
fn cleanup_failure_rolls_back_membership_removal() {
    run_case(
        "membership",
        "cleanup_failure_rolls_back_membership_removal",
        true,
    );
}
#[test]
fn cleanup_failure_rolls_back_sign_out() {
    run_case("session", "cleanup_failure_rolls_back_sign_out", true);
}
#[test]
fn cleanup_failure_rolls_back_ban() {
    run_case("ban", "cleanup_failure_rolls_back_ban", true);
}
#[test]
fn cleanup_failure_rolls_back_deactivation() {
    run_case(
        "deactivate",
        "cleanup_failure_rolls_back_deactivation",
        true,
    );
}
#[test]
fn cleanup_failure_rolls_back_room_deletion() {
    run_case("room", "cleanup_failure_rolls_back_room_deletion", true);
}
