use crate::models::huddle_grant::HuddleGrant;
use crate::models::huddle_notices;
use crate::tests::TestDb;
use crate::{CachedStatements, Timestamp, Tx};
use rusqlite::{params, types::Value as SqlValue};
use serde_json::{Value, json};

fn insert(tx: &Tx<'_>, table: &str, row: &Value) -> crate::Result<()> {
    let row = row.as_object().unwrap();
    let columns = row
        .keys()
        .map(|key| format!("\"{key}\""))
        .collect::<Vec<_>>()
        .join(",");
    let values = row
        .values()
        .map(|value| match value {
            Value::Null => SqlValue::Null,
            Value::Bool(value) => SqlValue::Integer(i64::from(*value)),
            Value::Number(value) => SqlValue::Integer(value.as_i64().unwrap()),
            Value::String(value) => SqlValue::Text(
                Timestamp::parse_db(value)
                    .map(|at| at.to_db())
                    .unwrap_or_else(|| value.clone()),
            ),
            _ => SqlValue::Text(value.to_string()),
        })
        .collect::<Vec<_>>();
    tx.conn().execute(
        &format!(
            "INSERT INTO {table}({columns}) VALUES({})",
            vec!["?"; values.len()].join(",")
        ),
        rusqlite::params_from_iter(values),
    )?;
    Ok(())
}

fn load(tx: &Tx<'_>, input: &Value) -> crate::Result<()> {
    tx.conn().execute_batch("DELETE FROM huddle_cleanups; DELETE FROM activity_items WHERE source_type='HuddleGrant'; DELETE FROM huddle_grants;")?;
    let mut room = input["room"].clone();
    room["creator_id"] = json!(crate::fixtures::identify("david"));
    room["created_at"] = json!(tx.now().to_db());
    room["updated_at"] = json!(tx.now().to_db());
    insert(tx, "rooms", &room)?;
    for member in input["memberships"].as_array().unwrap() {
        let mut member = member.clone();
        member["created_at"] = json!(tx.now().to_db());
        member["updated_at"] = json!(tx.now().to_db());
        insert(tx, "memberships", &member)?;
    }
    for user in input["users"].as_array().unwrap() {
        let role = match user["role"].as_str().unwrap() {
            "administrator" => 1,
            "member" => 0,
            "bot" => 2,
            _ => panic!("unknown role"),
        };
        let status = match user["status"].as_str().unwrap() {
            "active" => 0,
            "deactivated" => 1,
            "banned" => 2,
            _ => panic!("unknown status"),
        };
        tx.conn().execute_cached(
            "UPDATE users SET name=?,role=?,status=?,inbox_preferences=? WHERE id=?",
            params![
                user["name"].as_str(),
                role,
                status,
                user["inbox_preferences"].to_string(),
                user["id"].as_i64()
            ],
        )?;
    }
    for grant in input["grants"].as_array().unwrap() {
        insert(tx, "huddle_grants", grant)?;
    }
    for item in input["items"].as_array().unwrap() {
        insert(tx, "activity_items", item)?;
    }
    Ok(())
}

#[test]
fn huddle_join_leave_and_call_ended_match_sixty_seven_rails_scenarios() {
    let vectors: Value =
        serde_json::from_str(include_str!("../models/huddle_notice_vectors.json")).unwrap();
    let cases = vectors["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 67);
    for case in cases {
        let db = TestDb::new();
        db.clock
            .travel_to(Timestamp::from_second(vectors["now"].as_i64().unwrap()));
        let input = case["input"].clone();
        db.write(move |tx| load(tx, &input));
        db.sink.take();
        let id = case["grant_id"].as_i64().unwrap();
        let floor = case["floor"].as_i64().map(Timestamp::from_second);
        let operation = case["operation"].as_str().unwrap().to_string();
        db.write(move |tx| {
            let grant = HuddleGrant::find_by_id(tx.conn(), id)?.unwrap();
            match operation.as_str() {
                "join" => huddle_notices::notify_join(tx, id),
                "leave" => huddle_notices::notify_leave(tx, &grant),
                "ended" => huddle_notices::call_ended(tx, &grant),
                "revoke" => grant.clone().revoke(
                    tx,
                    false,
                    &crate::models::room_delete::HuddleConfig::default(),
                ),
                "disconnect" => grant.clone().mark_out_of_call(tx, floor).map(|_| ()),
                _ => panic!("unknown oracle operation"),
            }
        });
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
        assert_eq!(json!(broadcasts), case["broadcasts"], "{}", case["name"]);
        if case["name"].as_str().unwrap().starts_with("direct_pref_") {
            let requests = db
                .events()
                .iter()
                .filter_map(|event| event.as_job::<huddle_notices::PushRequest>())
                .count();
            assert_eq!(
                requests,
                case["pushes"].as_array().unwrap().len(),
                "{}",
                case["name"]
            );
        }
        for event in db.events() {
            if let Some(request) = event.as_job::<huddle_notices::PushRequest>() {
                assert_eq!(request.kind, huddle_notices::PushKind::HuddleJoin);
                assert_eq!(request.payload.body, "Join from the conversation");
                assert_eq!(request.payload.tag, "huddle-9001");
                assert_eq!(request.payload.path, "/rooms/9001");
            }
        }
    }
}

#[test]
fn huddle_push_scopes_and_throttle_match_thirty_six_rails_scenarios() {
    use huddle_notices::{PushKind, PushPayload, PushRequest};
    use jiff::SignedDuration;
    let vectors: Value =
        serde_json::from_str(include_str!("../models/huddle_push_vectors.json")).unwrap();
    assert_eq!(vectors["cases"].as_array().unwrap().len(), 36);
    for case in vectors["cases"].as_array().unwrap() {
        let db = TestDb::new();
        db.clock
            .travel_to(Timestamp::from_second(vectors["now"].as_i64().unwrap()));
        let kind = if case["kind"] == "huddle" {
            PushKind::Huddle
        } else {
            PushKind::HuddleJoin
        };
        let payload = vectors["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|base| {
                base["kind"] == case["kind"] && base["name"].as_str().unwrap().ends_with("_default")
            })
            .unwrap()["pushes"][0]["payload"]
            .clone();
        let request = PushRequest {
            kind,
            recipient_id: case["recipient_id"].as_i64().unwrap(),
            sender_id: case["sender_id"].as_i64().unwrap(),
            room_id: case["room_id"].as_i64().unwrap(),
            room_membership_id: case["membership_id"].as_i64(),
            payload: serde_json::from_value::<PushPayload>(payload).unwrap(),
        };
        let membership = request.room_membership_id.unwrap();
        let recipient = request.recipient_id;
        let options = case["options"].clone();
        db.write(move |tx| {
            let connected = options["connected_age"].as_i64().map(|age|tx.now().ago(SignedDuration::from_secs(age)));
            let pushed = options["push_age"].as_i64().map(|age|tx.now().ago(SignedDuration::from_secs(age)));
            let involvement = options.get("involvement").map_or(Some("everything"),|value|value.as_str());
            let preferences = if options["inbox"]==false {json!({"huddle_invitations":false})} else {json!({})};
            tx.conn().execute_cached("UPDATE memberships SET involvement=?,connections=?,connected_at=?,last_huddle_join_push_at=? WHERE id=?",params![involvement,options["connections"].as_i64().unwrap_or(0),connected,pushed,membership])?;
            tx.conn().execute_cached("UPDATE users SET inbox_preferences=? WHERE id=?",params![preferences.to_string(),recipient])?;
            if options["no_subscriptions"] == true { tx.conn().execute("DELETE FROM push_subscriptions WHERE user_id=?",[recipient])?; }
            Ok(())
        });
        let updated = db.read(|conn| Ok(crate::Membership::find(conn, membership)?.updated_at));
        let allowed = case["policy_allowed"].as_bool().unwrap();
        let delivery = db.write(move |tx| huddle_notices::prepare_push(tx, &request, allowed));
        let pushes = delivery.map(|delivery|vec![json!({"payload":delivery.payload,"subscription_ids":delivery.subscriptions.iter().map(|s|s.id).collect::<Vec<_>>()})]).unwrap_or_default();
        assert_eq!(json!(pushes), case["pushes"], "{}", case["name"]);
        let pushed: Option<Timestamp> = db.read(|conn| {
            Ok(conn.query_row(
                "SELECT last_huddle_join_push_at FROM memberships WHERE id=?",
                [membership],
                |r| r.get(0),
            )?)
        });
        assert_eq!(
            json!(pushed.map(|at| at.as_second())),
            case["after"],
            "{}",
            case["name"]
        );
        assert_eq!(
            updated,
            db.read(|conn| Ok(crate::Membership::find(conn, membership)?.updated_at)),
            "throttle touched updated_at"
        );
    }
}
