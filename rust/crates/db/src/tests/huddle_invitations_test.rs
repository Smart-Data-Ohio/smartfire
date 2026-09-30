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

pub(crate) fn snapshot(conn: &Connection, table: &str) -> crate::Result<Value> {
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

#[test]
fn overdue_invitations_match_twenty_nine_rails_scenarios_and_are_idempotent() {
    let vectors: Value =
        serde_json::from_str(include_str!("../models/huddle_resolver_vectors.json")).unwrap();
    assert_eq!(vectors["cases"].as_array().unwrap().len(), 29);
    for case in vectors["cases"].as_array().unwrap() {
        assert_eq!(case["idempotent"], true);
        let db = TestDb::new();
        db.clock
            .travel_to(Timestamp::from_second(vectors["now"].as_i64().unwrap()));
        let input = case["input"].clone();
        db.write(move |tx| huddle_notices_test::load(tx, &input));
        db.sink.take();
        let user = case["user_id"].as_i64();
        db.write(move |tx| huddle_invitations::resolve_overdue(tx, user));
        let events = db.sink.take();
        for ring in events
            .iter()
            .filter_map(|event| event.as_job::<RingRequest>())
        {
            db.write(move |tx| huddle_invitations::publish_ring(tx, &ring, true));
        }
        let actual = db
            .events()
            .iter()
            .filter_map(|event| match event.as_broadcast()? {
                crate::broadcasts::Broadcast::Cable { stream, payload } => {
                    Some(json!({"stream":stream,"payload":payload}))
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(json!(actual), case["broadcasts"], "{}", case["name"]);
        let mut actual = db.read(|conn| snapshot(conn, "activity_items"));
        let mut expected = case["items"].clone();
        normalize(&mut actual);
        normalize(&mut expected);
        assert_eq!(actual, expected, "{}", case["name"]);
        db.sink.take();
        db.write(move |tx| huddle_invitations::resolve_overdue(tx, user));
        assert!(db.events().is_empty(), "{} resolved twice", case["name"]);
    }
}

#[test]
fn stale_stream_state_matches_sixteen_rails_scenarios() {
    let vectors: Value =
        serde_json::from_str(include_str!("../models/huddle_stale_stream_vectors.json")).unwrap();
    assert_eq!(vectors["cases"].as_array().unwrap().len(), 16);
    for case in vectors["cases"].as_array().unwrap() {
        let db = TestDb::new();
        db.clock
            .travel_to(Timestamp::from_second(vectors["now"].as_i64().unwrap()));
        let input = case["input"].clone();
        db.write(move |tx| {
            huddle_notices_test::load(tx, &input)?;
            for stream in input["streams"].as_array().unwrap() {
                huddle_notices_test::insert(tx, "streams", stream)?;
            }
            Ok(())
        });
        let ids = db.read(crate::models::huddle_stream_liveness::live_ids);
        let mut errors = Value::Null;
        for id in ids {
            match db.try_write(move |tx| crate::models::huddle_stream_liveness::end_stale(tx, id)) {
                Ok(()) => (),
                Err(crate::Error::RecordInvalid(error)) => {
                    let mut fields = serde_json::Map::new();
                    for (attribute, message) in error.0 {
                        fields
                            .entry(attribute)
                            .or_insert_with(|| json!([]))
                            .as_array_mut()
                            .unwrap()
                            .push(json!(message));
                    }
                    errors = Value::Object(fields);
                    break;
                }
                Err(error) => panic!("{}: {error}", case["name"]),
            }
        }
        assert_eq!(errors, case["errors"], "{}", case["name"]);
        let mut actual = db.read(|conn| snapshot(conn, "streams"));
        let mut expected = case["streams"].clone();
        normalize(&mut actual);
        normalize(&mut expected);
        assert_eq!(actual, expected, "{}", case["name"]);
        let ids = db.read(crate::models::huddle_stream_liveness::live_ids);
        if errors.is_null() {
            for id in ids {
                db.write(move |tx| crate::models::huddle_stream_liveness::end_stale(tx, id));
            }
            let mut repeated = db.read(|conn| snapshot(conn, "streams"));
            normalize(&mut repeated);
            assert_eq!(
                repeated, actual,
                "{} repeated sweep changed timestamps",
                case["name"]
            );
        }
    }
}

#[test]
fn hand_mutations_and_role_clearing_match_seventeen_rails_scenarios() {
    let vectors: Value =
        serde_json::from_str(include_str!("../models/huddle_hand_vectors.json")).unwrap();
    assert_eq!(vectors["cases"].as_array().unwrap().len(), 17);
    for case in vectors["cases"].as_array().unwrap() {
        let db = TestDb::new();
        db.clock
            .travel_to(Timestamp::from_second(vectors["now"].as_i64().unwrap()));
        let input = case["input"].clone();
        db.write(move |tx| {
            huddle_notices_test::load(tx, &input)?;
            // Keep the original timestamps: no-op lower and repeated raise must not touch them.
            for member in input["memberships"].as_array().unwrap() {
                let created = Timestamp::parse_db(member["created_at"].as_str().unwrap()).unwrap();
                let updated = Timestamp::parse_db(member["updated_at"].as_str().unwrap()).unwrap();
                tx.conn().execute(
                    "UPDATE memberships SET created_at=?,updated_at=? WHERE id=?",
                    rusqlite::params![created, updated, member["id"].as_i64()],
                )?;
            }
            Ok(())
        });
        let id = case["membership_id"].as_i64().unwrap();
        let operation = case["operation"].as_str().unwrap().to_string();
        let role = case["next_role"].as_str().map(|r| match r {
            "listener" => crate::StageRole::Listener,
            "speaker" => crate::StageRole::Speaker,
            "host" => crate::StageRole::Host,
            _ => panic!("unknown role"),
        });
        let result = db.try_write(move |tx| {
            let mut member = crate::Membership::find(tx.conn(), id)?;
            match operation.as_str() {
                "raise" => member.raise_hand(tx),
                "lower" => member.lower_hand(tx),
                "role" => member.change_stage_role(tx, role.unwrap()).map(|()| true),
                _ => panic!("unknown operation"),
            }
        });
        let mut errors = Value::Null;
        match result {
            Ok(changed) => assert_eq!(json!(changed), case["changed"], "{}", case["name"]),
            Err(crate::Error::RecordInvalid(error)) => {
                let mut fields = serde_json::Map::new();
                for (attribute, message) in error.0 {
                    fields
                        .entry(attribute)
                        .or_insert_with(|| json!([]))
                        .as_array_mut()
                        .unwrap()
                        .push(json!(message));
                }
                errors = Value::Object(fields);
            }
            Err(error) => panic!("{}: {error}", case["name"]),
        }
        assert_eq!(errors, case["errors"], "{}", case["name"]);
        let members = db.read(|conn| snapshot(conn, "memberships"));
        let mut actual = members
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["id"] == id)
            .unwrap()
            .clone();
        let mut expected = case["membership"].clone();
        normalize(&mut actual);
        normalize(&mut expected);
        assert_eq!(actual, expected, "{}", case["name"]);
    }
}

#[test]
fn stream_lifecycle_and_host_departure_match_twenty_rails_scenarios() {
    use crate::models::huddle_effects::{StreamChanged, StreamStopped};
    use crate::models::stream::Stream;
    let vectors: Value = serde_json::from_str(include_str!(
        "../models/huddle_stream_lifecycle_vectors.json"
    ))
    .unwrap();
    assert_eq!(vectors["cases"].as_array().unwrap().len(), 20);
    for case in vectors["cases"].as_array().unwrap() {
        let db = TestDb::new();
        db.clock
            .travel_to(Timestamp::from_second(vectors["now"].as_i64().unwrap()));
        let input = case["input"].clone();
        db.write(move |tx| {
            huddle_notices_test::load(tx, &input)?;
            tx.conn()
                .execute("DELETE FROM sqlite_sequence WHERE name='streams'", [])?;
            tx.conn().execute(
                "INSERT INTO sqlite_sequence(name,seq) VALUES('streams',39)",
                [],
            )?;
            for m in input["memberships"].as_array().unwrap() {
                tx.conn().execute(
                    "UPDATE memberships SET created_at=?,updated_at=? WHERE id=?",
                    rusqlite::params![
                        Timestamp::parse_db(m["created_at"].as_str().unwrap()),
                        Timestamp::parse_db(m["updated_at"].as_str().unwrap()),
                        m["id"].as_i64()
                    ],
                )?;
            }
            Ok(())
        });
        db.sink.take();
        for result in case["results"].as_array().unwrap() {
            let op = result["operation"].clone();
            let actual = db.try_write(move |tx| match op["op"].as_str().unwrap() {
                "create" => Stream::create(
                    tx,
                    9001,
                    op["membership_id"].as_i64().unwrap(),
                    op["user_id"].as_i64().unwrap(),
                    op["quality"].as_str().unwrap(),
                    op["started_at"].as_str().and_then(Timestamp::parse_db),
                )
                .map(|_| ()),
                "end" => Stream::find_by_id(tx.conn(), 40)?
                    .unwrap()
                    .end(tx, op["actor_id"].as_i64())
                    .map(|_| ()),
                "destroy_member" => {
                    crate::Membership::find(tx.conn(), op["id"].as_i64().unwrap())?.destroy(tx)
                }
                "deactivate" => {
                    crate::User::find(tx.conn(), op["id"].as_i64().unwrap())?.deactivate(tx)
                }
                _ => panic!("unknown lifecycle operation"),
            });
            let error = match actual {
                Ok(()) => Value::Null,
                Err(e) if e.is_record_not_unique() => json!("not_unique"),
                Err(crate::Error::RecordInvalid(error)) => {
                    let mut fields = serde_json::Map::new();
                    for (attribute, message) in error.0 {
                        fields
                            .entry(attribute)
                            .or_insert_with(|| json!([]))
                            .as_array_mut()
                            .unwrap()
                            .push(json!(message));
                    }
                    Value::Object(fields)
                }
                Err(e) => panic!("{}: {e}", case["name"]),
            };
            assert_eq!(error, result["error"], "{}", case["name"]);
            let mut actual = db.read(|conn| snapshot(conn, "streams"));
            let mut expected = result["streams"].clone();
            normalize(&mut actual);
            normalize(&mut expected);
            assert_eq!(actual, expected, "{} streams", case["name"]);
            let members=db.read(|conn|Ok(crate::Membership::for_room(conn,9001)?.iter().map(|m|json!({"id":m.id,"stage_role":m.stage_role.map(|r|r.name()),"hand_raised_at":m.hand_raised_at.map(|at|at.to_db())})).collect::<Vec<_>>()));
            let mut expected = result["members"].clone();
            normalize(&mut expected);
            assert_eq!(json!(members), expected, "{} members", case["name"]);
            let notes=db.read(|conn|crate::sql::query_all(conn,"SELECT * FROM messages WHERE room_id=9001 AND system_note=1 ORDER BY id",[],crate::Message::from_row).and_then(|messages|messages.iter().map(|m|Ok(json!({"creator_id":m.creator_id,"body":m.plain_text_body(conn,db.db.env().rich_text.as_ref())?}))).collect::<crate::Result<Vec<_>>>()));
            assert_eq!(json!(notes), result["notes"], "{} notes", case["name"]);
            let events = db.sink.take();
            let changed=events.iter().filter(|e|matches!(e,crate::Event::Broadcast(b) if b.decode::<StreamChanged>().is_some())).count();
            let stopped=events.iter().filter(|e|matches!(e,crate::Event::Broadcast(b) if b.decode::<StreamStopped>().is_some())).count();
            let frames = result["frames"].as_array().unwrap();
            assert_eq!(
                stopped,
                frames.iter().filter(|f| f["stopped"] == true).count(),
                "{} stopped",
                case["name"]
            );
            assert_eq!(
                changed,
                frames
                    .iter()
                    .filter(|f| f["target"] == "stage_live_badge_rooms_stage_9001")
                    .count(),
                "{} changed",
                case["name"]
            );
            // Quiet succession notes never update unread membership state or enqueue push work.
            if !notes.is_empty() {
                assert!(!events.iter().any(|e| {
                    e.as_job::<crate::models::huddle_notices::PushRequest>()
                        .is_some()
                }));
            }
        }
    }
}

#[test]
fn call_moderation_mutations_match_twenty_nine_rails_controller_scenarios() {
    use crate::models::call_moderation::{self, Action, Denial};
    use crate::models::huddle_effects::{RoleEvent, StageRoster};
    let vectors: Value =
        serde_json::from_str(include_str!("../models/huddle_moderation_vectors.json")).unwrap();
    assert_eq!(vectors["cases"].as_array().unwrap().len(), 29);
    for case in vectors["cases"].as_array().unwrap() {
        let db = TestDb::new();
        db.clock
            .travel_to(Timestamp::from_second(vectors["now"].as_i64().unwrap()));
        let input = case["input"].clone();
        db.write(move |tx| {
            huddle_notices_test::load(tx, &input)?;
            for m in input["memberships"].as_array().unwrap() {
                tx.conn().execute(
                    "UPDATE memberships SET created_at=?,updated_at=? WHERE id=?",
                    rusqlite::params![
                        Timestamp::parse_db(m["created_at"].as_str().unwrap()),
                        Timestamp::parse_db(m["updated_at"].as_str().unwrap()),
                        m["id"].as_i64()
                    ],
                )?;
            }
            for stream in input["streams"].as_array().unwrap() {
                huddle_notices_test::insert(tx, "streams", stream)?;
            }
            Ok(())
        });
        db.sink.take();
        let actor = case["actor_id"].as_i64().unwrap();
        let target = case["target_id"].as_i64();
        let action = match case["action"].as_str().unwrap() {
            "mute" => Action::Mute,
            "unmute" => Action::Unmute,
            _ => Action::Disconnect,
        };
        let actual = db.write(move |tx| {
            call_moderation::moderate(
                tx,
                9001,
                actor,
                target,
                action,
                &HuddleConfig {
                    api_secret: Some("ws13-fixture-api-secret".into()),
                    admin_configured: true,
                },
            )
        });
        let status = match actual {
            Err(Denial::NotFound | Denial::TargetNotFound) => 404,
            Err(Denial::Forbidden | Denial::AdministratorRank) => 403,
            Err(Denial::SelfTarget) => 422,
            Ok(()) => case["status"].as_i64().unwrap(),
        };
        assert_eq!(json!(status), case["status"], "{} status", case["name"]);
        for (table, field, columns) in [
            (
                "memberships",
                "members",
                vec!["id", "server_muted_at", "updated_at"],
            ),
            ("huddle_grants", "grants", vec!["id", "revoked_at"]),
            ("streams", "streams", vec!["id", "ended_at"]),
            (
                "huddle_cleanups",
                "cleanups",
                vec!["operation", "huddle_grant_id", "room_name", "identity"],
            ),
        ] {
            let actual = db.read(|conn| {
                let sql = format!(
                    "SELECT {} FROM {table} {} ORDER BY id",
                    columns.join(","),
                    if table == "memberships" {
                        "WHERE room_id=9001"
                    } else {
                        ""
                    }
                );
                let mut statement = conn.prepare(&sql)?;
                let rows = statement
                    .query_map([], |row| {
                        let mut object = serde_json::Map::new();
                        for (i, column) in columns.iter().enumerate() {
                            let value = match row.get_ref(i)? {
                                rusqlite::types::ValueRef::Null => Value::Null,
                                rusqlite::types::ValueRef::Integer(n) if *column == "operation" => {
                                    json!(if n == 0 {
                                        "remove_participant"
                                    } else {
                                        "delete_room"
                                    })
                                }
                                rusqlite::types::ValueRef::Integer(n) => json!(n),
                                rusqlite::types::ValueRef::Text(s) => {
                                    json!(std::str::from_utf8(s).unwrap())
                                }
                                _ => panic!("unexpected moderation field"),
                            };
                            object.insert(column.to_string(), value);
                        }
                        Ok(Value::Object(object))
                    })?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                Ok(rows)
            });
            let mut actual = json!(actual);
            let mut expected = case[field].clone();
            normalize(&mut actual);
            normalize(&mut expected);
            assert_eq!(actual, expected, "{} {field}", case["name"]);
        }
        let events = db.sink.take();
        let rosters = events
            .iter()
            .filter(
                |e| matches!(e,crate::Event::Broadcast(b) if b.decode::<StageRoster>().is_some()),
            )
            .count();
        let role_events = events
            .iter()
            .filter(|e| matches!(e,crate::Event::Broadcast(b) if b.decode::<RoleEvent>().is_some()))
            .count();
        assert_eq!(
            role_events,
            case["role_events"].as_array().unwrap().len(),
            "{} role events",
            case["name"]
        );
        let members = case["members"].as_array().unwrap().len();
        assert_eq!(
            rosters * members,
            case["rosters"].as_u64().unwrap() as usize,
            "{} roster fanout",
            case["name"]
        );
    }
}

#[test]
fn moderation_enqueue_failure_rolls_back_mute_revocation_stream_and_frames() {
    use crate::models::call_moderation::{self, Action};
    let vectors: Value =
        serde_json::from_str(include_str!("../models/huddle_moderation_vectors.json")).unwrap();
    let case = vectors["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "host_mute")
        .unwrap();
    let db = TestDb::new();
    db.clock
        .travel_to(Timestamp::from_second(vectors["now"].as_i64().unwrap()));
    let input = case["input"].clone();
    db.write(move |tx| {
        huddle_notices_test::load(tx,&input)?;
        let target=crate::Membership::find(tx.conn(),9013)?;
        crate::models::stream::Stream::create(tx,9001,target.id,target.user_id,"1080p15",None)?;
        tx.conn().execute_batch("CREATE TRIGGER ws13_reject_cleanup BEFORE INSERT ON huddle_cleanups BEGIN SELECT RAISE(ABORT,'WS13 intentional enqueue failure'); END;")?;
        Ok(())
    });
    db.sink.take();
    let actor = case["actor_id"].as_i64().unwrap();
    let result = db.try_write(move |tx| {
        call_moderation::moderate(
            tx,
            9001,
            actor,
            Some(9013),
            Action::Mute,
            &HuddleConfig {
                api_secret: Some("ws13-fixture-api-secret".into()),
                admin_configured: true,
            },
        )
    });
    assert!(result.is_err(), "failed cleanup must abort moderation");
    db.read(|conn| {
        assert!(
            crate::Membership::find(conn, 9013)?
                .server_muted_at
                .is_none()
        );
        assert!(!HuddleGrant::find_by_id(conn, 17)?.unwrap().revoked());
        assert!(crate::models::stream::Stream::live_for_room(conn, 9001)?.is_some());
        Ok(())
    });
    assert!(
        db.events().is_empty(),
        "rolled-back moderation escaped to Cable/jobs"
    );
}

#[test]
fn stage_roles_and_hands_match_thirty_four_rails_controller_scenarios() {
    use crate::models::huddle_effects::{RoleEvent, StagePanel, StageRoster, StreamChanged};
    use crate::models::stage_participation::{self, Denial, HandTarget};
    let vectors: Value =
        serde_json::from_str(include_str!("../models/huddle_participation_vectors.json")).unwrap();
    assert_eq!(vectors["cases"].as_array().unwrap().len(), 34);
    for case in vectors["cases"].as_array().unwrap() {
        let db = TestDb::new();
        db.clock
            .travel_to(Timestamp::from_second(vectors["now"].as_i64().unwrap()));
        let input = case["input"].clone();
        db.write(move |tx| {
            huddle_notices_test::load(tx, &input)?;
            for m in input["memberships"].as_array().unwrap() {
                tx.conn().execute(
                    "UPDATE memberships SET created_at=?,updated_at=? WHERE id=?",
                    rusqlite::params![
                        Timestamp::parse_db(m["created_at"].as_str().unwrap()),
                        Timestamp::parse_db(m["updated_at"].as_str().unwrap()),
                        m["id"].as_i64()
                    ],
                )?;
            }
            for stream in input["streams"].as_array().unwrap() {
                huddle_notices_test::insert(tx, "streams", stream)?;
            }
            Ok(())
        });
        db.sink.take();
        let actor = case["actor_id"].as_i64().unwrap();
        let target = case["target_id"].as_i64();
        let action = case["action"].as_str().unwrap().to_string();
        let params = case["params"].clone();
        let actual = db.try_write(move |tx| match action.as_str() {
            "role" => stage_participation::change_role(
                tx,
                9001,
                actor,
                target,
                params["stage_role"].as_str().unwrap(),
                &HuddleConfig {
                    api_secret: Some("ws13-fixture-api-secret".into()),
                    admin_configured: true,
                },
            ),
            "raise" => stage_participation::raise_hand(tx, 9001, actor),
            _ => stage_participation::lower_hand(
                tx,
                9001,
                actor,
                if params.get("membership_id").is_some() {
                    HandTarget::Other(target)
                } else {
                    HandTarget::Own
                },
            ),
        });
        let status = match actual {
            Err(crate::Error::RecordInvalid(_)) => 422,
            Err(error) => panic!("{}: {error}", case["name"]),
            Ok(Err(Denial::NotFound | Denial::TargetNotFound)) => 404,
            Ok(Err(Denial::Forbidden | Denial::PlainForbidden(_))) => 403,
            Ok(Err(Denial::UnknownRole | Denial::ListenerOnly)) => 422,
            Ok(Ok(())) => case["status"].as_i64().unwrap(),
        };
        assert_eq!(json!(status), case["status"], "{} status", case["name"]);
        for (table, field, columns) in [
            (
                "memberships",
                "members",
                vec!["id", "stage_role", "hand_raised_at", "updated_at"],
            ),
            (
                "huddle_grants",
                "grants",
                vec!["id", "stage_role", "revoked_at"],
            ),
            ("streams", "streams", vec!["id", "ended_at"]),
            (
                "huddle_cleanups",
                "cleanups",
                vec!["operation", "huddle_grant_id", "room_name", "identity"],
            ),
        ] {
            let actual = db.read(|conn| {
                let sql = format!(
                    "SELECT {} FROM {table} {} ORDER BY id",
                    columns.join(","),
                    if table == "memberships" {
                        "WHERE room_id=9001"
                    } else {
                        ""
                    }
                );
                let mut statement = conn.prepare(&sql)?;
                let rows = statement
                    .query_map([], |row| {
                        let mut object = serde_json::Map::new();
                        for (i, column) in columns.iter().enumerate() {
                            let value = match row.get_ref(i)? {
                                rusqlite::types::ValueRef::Null => Value::Null,
                                rusqlite::types::ValueRef::Integer(n) if *column == "operation" => {
                                    json!(if n == 0 {
                                        "remove_participant"
                                    } else {
                                        "delete_room"
                                    })
                                }
                                rusqlite::types::ValueRef::Integer(n) => json!(n),
                                rusqlite::types::ValueRef::Text(s) => {
                                    json!(std::str::from_utf8(s).unwrap())
                                }
                                _ => panic!("unexpected moderation field"),
                            };
                            object.insert(column.to_string(), value);
                        }
                        Ok(Value::Object(object))
                    })?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                Ok(rows)
            });
            let mut actual = json!(actual);
            let mut expected = case[field].clone();
            normalize(&mut actual);
            normalize(&mut expected);
            assert_eq!(actual, expected, "{} {field}", case["name"]);
        }
        let events = db.sink.take();
        let rosters = events
            .iter()
            .filter(
                |e| matches!(e,crate::Event::Broadcast(b) if b.decode::<StageRoster>().is_some()),
            )
            .count();
        let role_events = events
            .iter()
            .filter(|e| matches!(e,crate::Event::Broadcast(b) if b.decode::<RoleEvent>().is_some()))
            .count();
        assert_eq!(
            role_events,
            case["role_events"].as_array().unwrap().len(),
            "{} role events",
            case["name"]
        );
        let panels = events
            .iter()
            .filter(
                |e| matches!(e,crate::Event::Broadcast(b) if b.decode::<StagePanel>().is_some()),
            )
            .count();
        let changes = events
            .iter()
            .filter(
                |e| matches!(e,crate::Event::Broadcast(b) if b.decode::<StreamChanged>().is_some()),
            )
            .count();
        assert_eq!(
            panels + changes * case["members"].as_array().unwrap().len(),
            case["panels"].as_u64().unwrap() as usize,
            "{} panel fanout",
            case["name"]
        );
        let members = case["members"].as_array().unwrap().len();
        assert_eq!(
            rosters * members,
            case["rosters"].as_u64().unwrap() as usize,
            "{} roster fanout",
            case["name"]
        );
    }
}
