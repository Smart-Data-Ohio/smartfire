use crate::models::{huddle_grant::HuddleGrant, room_delete::HuddleConfig, stream::Stream};
use crate::tests::{TestDb, huddle_invitations_test, huddle_notices_test, huddle_revocation_test};
use crate::{Membership, Message, NewMessage, Room, RoomType, StageRole, Timestamp};
use rusqlite::params;
use serde_json::{Value, json};

fn run(name: &str) {
    if std::env::var("WS13B_LIFECYCLE_CHILD").as_deref() != Ok(name) {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                &format!("tests::huddle_domain_lifecycle_sequences_test::{name}"),
                "--nocapture",
            ])
            .env("WS13B_LIFECYCLE_CHILD", name)
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
    let vectors: Value = serde_json::from_str(include_str!(
        "huddle_domain_lifecycle_sequence_vectors.json"
    ))
    .unwrap();
    assert_eq!(vectors["cases"].as_array().unwrap().len(), 20);
    let case = vectors["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == name)
        .unwrap();
    let db = TestDb::new();
    db.clock
        .travel_to(Timestamp::from_second(vectors["now"].as_i64().unwrap()));
    let david = crate::fixtures::identify("david");
    let baseline = db.read(|conn| Membership::count_without_direct_rooms(conn, david));
    let input = case["input"].clone();
    db.write(move |tx| {
        let first = json!({"room":input["rooms"][0],"memberships":[],"users":input["users"],"grants":[],"items":[]});
        huddle_notices_test::load(tx, &first)?;
        huddle_notices_test::insert(tx, "rooms", &input["rooms"][1])?;
        for table in ["memberships", "sessions"] {
            for row in input[table].as_array().unwrap() {huddle_notices_test::insert(tx, table, row)?;}
        }
        tx.conn().execute_batch("DELETE FROM sqlite_sequence WHERE name IN ('messages','streams','huddle_grants','huddle_cleanups'); INSERT INTO sqlite_sequence(name,seq) VALUES('messages',1200000000),('streams',39);")?;
        Ok(())
    });
    db.sink.take();
    for expected in case["results"].as_array().unwrap() {
        let op = expected["operation"].clone();
        let actual = db.write(move |tx| {
            let id = op["id"].as_i64().unwrap_or_default();
            let config = HuddleConfig {api_secret: Some("ws13b-fixture-api-secret".into()), admin_configured: false};
            match op["op"].as_str().unwrap() {
                "issue" => HuddleGrant::issue(tx, op["session_id"].as_i64().unwrap(), op["membership_id"].as_i64().unwrap(), op["room_id"].as_i64().unwrap(), &config).map(|g| json!(g.id)),
                "stream" => Stream::create(tx, op["room_id"].as_i64().unwrap(), op["membership_id"].as_i64().unwrap(), op["user_id"].as_i64().unwrap(), "1080p15", None).map(|s| json!(s.id)),
                "role" => Membership::find(tx.conn(), id)?.change_stage_role_with_config(tx, StageRole::from_name(op["value"].as_str().unwrap()).unwrap(), &config).map(|_| Value::Null),
                "grant_role" => {tx.conn().execute("UPDATE huddle_grants SET stage_role=? WHERE id=?",params![op["value"].as_str(), id])?; Ok(Value::Null)},
                "authorize" => HuddleGrant::authorize_or_revoke(tx, id, &config).map(|grant| json!(grant.is_some())),
                "end" => Stream::find_by_id(tx.conn(), id)?.unwrap().end(tx, None).map(|_| Value::Null),
                "revoke" => HuddleGrant::find_by_id(tx.conn(), id)?.unwrap().revoke(tx, true, &config).map(|_| Value::Null),
                "destroy_member" => Membership::find(tx.conn(), id)?.destroy(tx).map(|_| Value::Null),
                "deactivate" => {let member = Membership::find(tx.conn(), id)?; crate::User::find(tx.conn(), member.user_id)?.deactivate(tx).map(|_| Value::Null)},
                "destroy_room" => {let room = Room::find(tx.conn(), 9001)?; crate::models::room_delete::destroy(tx, &room).map(|_| Value::Null)},
                "add_member" => {let user = crate::fixtures::identify(op["user"].as_str().unwrap()); Membership::create_default(tx,9001,user).map(|member|json!(member.id))},
                "chat" => {let body = if Room::find(tx.conn(),9001)?.stage() {"Hello from stage"} else {"Hello from voice"}; Message::create(tx,NewMessage {room_id:9001, creator_id:david, body:Some(body.into()), client_message_id:Some("ws13b-lifecycle-chat".into()), ..Default::default()}).map(|m| json!(m.id))},
                "reachable" => Ok(json!({"david":Message::find_reachable(tx.conn(),david,1200000001).is_ok(),"jason":Message::find_reachable(tx.conn(),crate::fixtures::identify("jason"),1200000001).is_ok()})),
                "scopes" => Ok(json!({"channel":Room::for_user_without_directs(tx.conn(),david)?.iter().any(|r|r.id==9001),"voice":Room::of_type(tx.conn(),RoomType::Voice)?.iter().any(|r|r.id==9001),"stage":Room::of_type(tx.conn(),RoomType::Stage)?.iter().any(|r|r.id==9001),"membership_delta":Membership::count_without_direct_rooms(tx.conn(),david)?-baseline})),
                "live" => {let current=Stream::live_for_room(tx.conn(),9001)?.map(|s|s.id); let mut ids=[current,Stream::live_for_room(tx.conn(),9002)?.map(|s|s.id)].into_iter().flatten().collect::<Vec<_>>(); ids.sort(); Ok(json!({"ids":ids,"current":current}))},
                _ => panic!("unknown operation {op}"),
            }
        });
        assert_eq!(
            actual, expected["value"],
            "{name} {} value",
            expected["operation"]
        );
        for (field, mut actual) in [
            ("grants",db.read(|conn|huddle_invitations_test::snapshot(conn,"huddle_grants"))),
            ("streams",db.read(|conn|huddle_invitations_test::snapshot(conn,"streams"))),
            ("memberships",db.read(|conn|huddle_invitations_test::snapshot(conn,"memberships")).as_array().unwrap().iter().filter(|r|[9001,9002].contains(&r["room_id"].as_i64().unwrap())).cloned().collect::<Vec<_>>().into()),
            ("rooms",db.read(|conn|{let mut statement=conn.prepare("SELECT id,deleted_at FROM rooms WHERE id IN (9001,9002) ORDER BY id")?; Ok(json!(statement.query_map([],|r|Ok(json!({"id":r.get::<_,i64>(0)?,"deleted_at":r.get::<_,Option<String>>(1)?})))?.collect::<std::result::Result<Vec<_>,_>>()?))})),
            ("messages",db.read(|conn|{let messages=crate::sql::query_all(conn,"SELECT * FROM messages WHERE room_id IN (9001,9002) ORDER BY id",[],Message::from_row)?; Ok(json!(messages.iter().map(|m|Ok(json!({"id":m.id,"room_id":m.room_id,"creator_id":m.creator_id,"system_note":m.system_note,"body":m.plain_text_body(conn,db.db.env().rich_text.as_ref())?}))).collect::<crate::Result<Vec<_>>>()?))})),
        ] {
            let mut wanted=expected[field].clone();
            huddle_revocation_test::normalize(&mut actual); huddle_revocation_test::normalize(&mut wanted);
            assert_eq!(actual,wanted,"{name} {} {field}",expected["operation"]);
        }
        // Succession's note stays quiet; no push request escapes the callback.
        if expected["operation"]["op"] == "deactivate"
            || expected["operation"]["op"] == "destroy_member"
        {
            assert!(
                !db.events().iter().any(|e| e
                    .as_job::<crate::models::huddle_notices::PushRequest>()
                    .is_some()),
                "{name} quiet note pushed"
            );
        }
        db.sink.take();
    }
}
macro_rules! case {
    ($name:ident) => {
        #[test]
        fn $name() {
            run(stringify!($name));
        }
    };
}
case!(stage_scopes);
case!(stage_later_member);
case!(stage_reachable);
case!(deactivate_admin_successor);
case!(deactivate_earliest_successor);
case!(deactivate_other_host);
case!(deactivate_empty);
case!(destroy_admin_successor);
case!(destroy_other_host);
case!(destroy_speaker);
case!(fresh_live_stream);
case!(stream_live_scope);
case!(revoke_other_member);
case!(authorization_ends_stream);
case!(destroy_presenter_with_grant);
case!(deactivate_presenter_with_grant);
case!(destroy_room_streams);
case!(voice_scopes);
case!(voice_reachable);
case!(voice_deactivate);
