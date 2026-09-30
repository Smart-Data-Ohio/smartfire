use crate::models::huddle_grant::HuddleGrant;
use crate::models::huddle_invitations::{self, RingRequest};
use crate::models::room_delete::HuddleConfig;
use crate::tests::{TestDb, huddle_notices_test};
use crate::{Connection, Timestamp};
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

fn snapshot(conn: &Connection, table: &str) -> crate::Result<Value> {
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
                if column == "identity" {
                    continue;
                }
                let value = match row.get_ref(index)? {
                    rusqlite::types::ValueRef::Null => Value::Null,
                    rusqlite::types::ValueRef::Integer(n) if column == "server_muted" => {
                        json!(n != 0)
                    }
                    rusqlite::types::ValueRef::Integer(n) => json!(n),
                    rusqlite::types::ValueRef::Text(s) => json!(std::str::from_utf8(s).unwrap()),
                    _ => panic!("unexpected snapshot field"),
                };
                object.insert(column.clone(), value);
            }
            Ok(Value::Object(object))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(json!(rows))
}

#[test]
fn issuance_invitations_match_forty_nine_rails_scenarios() {
    let vectors: Value =
        serde_json::from_str(include_str!("../models/huddle_issuance_vectors.json")).unwrap();
    assert_eq!(vectors["cases"].as_array().unwrap().len(), 49);
    for case in vectors["cases"].as_array().unwrap() {
        let db = TestDb::new();
        db.clock
            .travel_to(Timestamp::from_second(vectors["now"].as_i64().unwrap()));
        let input = case["input"].clone();
        db.write(move |tx| {
            huddle_notices_test::load(tx, &input)?;
            tx.conn()
                .execute("DELETE FROM sessions WHERE id IN (7001,7002,7003)", [])?;
            for session in input["sessions"].as_array().unwrap() {
                huddle_notices_test::insert(tx, "sessions", session)?;
            }
            tx.conn().execute(
                "DELETE FROM sqlite_sequence WHERE name IN ('huddle_grants','activity_items')",
                [],
            )?;
            for (name, seq) in input["sequences"].as_object().unwrap() {
                tx.conn().execute(
                    "INSERT INTO sqlite_sequence(name,seq) VALUES(?,?)",
                    rusqlite::params![name, seq.as_i64()],
                )?;
            }
            Ok(())
        });
        db.sink.take();
        let session = case["session_id"].as_i64().unwrap();
        let membership = case["membership_id"].as_i64().unwrap();
        let room = case["room_id"].as_i64().unwrap();
        let result = db.try_write(move |tx| {
            HuddleGrant::issue(
                tx,
                session,
                membership,
                room,
                &HuddleConfig {
                    api_secret: Some("ws13-fixture-api-secret".into()),
                    admin_configured: false,
                },
            )
        });
        let grant = match result {
            Ok(grant) => {
                assert!(case["error"].is_null(), "{}", case["name"]);
                json!(grant.id)
            }
            Err(error) => {
                assert_eq!(
                    error.to_string(),
                    "HuddleGrant::Ineligible",
                    "{}",
                    case["name"]
                );
                assert_eq!(case["error"], "HuddleGrant::Ineligible");
                Value::Null
            }
        };
        assert_eq!(grant, case["grant_id"], "{}", case["name"]);
        let events = db.events();
        let jobs = events
            .iter()
            .filter_map(|event| event.as_job::<crate::models::huddle_notices::PushInvitationJob>())
            .map(|job| job.activity_item_id)
            .collect::<Vec<_>>();
        let rings = events
            .iter()
            .filter_map(|event| event.as_job::<RingRequest>())
            .collect::<Vec<_>>();
        let sound = case["sound_allowed"].as_bool().unwrap();
        for ring in rings {
            db.write(move |tx| huddle_invitations::publish_ring(tx, &ring, sound));
        }
        let broadcasts = db
            .events()
            .iter()
            .filter_map(|event| match event.as_broadcast()? {
                crate::broadcasts::Broadcast::Cable { stream, payload } => {
                    Some(json!({"stream":stream,"payload":payload}))
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        for (actual, expected, field) in [
            (
                db.read(|conn| snapshot(conn, "activity_items")),
                case["items"].clone(),
                "items",
            ),
            (
                db.read(|conn| snapshot(conn, "huddle_grants")),
                case["grants"].clone(),
                "grants",
            ),
            (json!(broadcasts), case["broadcasts"].clone(), "broadcasts"),
            (json!(jobs), case["push_jobs"].clone(), "push jobs"),
        ] {
            let (mut actual, mut expected) = (actual, expected);
            normalize(&mut actual);
            normalize(&mut expected);
            assert_eq!(actual, expected, "{} {field}", case["name"]);
        }
    }
}
