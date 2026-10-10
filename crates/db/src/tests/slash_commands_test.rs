use super::*;
use crate::slash_commands::{self as slash, Context};
use serde_json::{Value, json};
fn vectors() -> Value {
    serde_json::from_str(include_str!("ws8_slash_vectors.json")).unwrap()
}
fn review_vectors() -> Value {
    serde_json::from_str(include_str!("ws8_slash_review_vectors.json")).unwrap()
}
fn stamp(t: Option<crate::Timestamp>) -> Value {
    t.map(|t| json!(t.to_db())).unwrap_or(Value::Null)
}
#[test]
fn slash_parser_matches_rails_timezone_and_dst_vectors() {
    for case in vectors()["parsing"].as_array().unwrap() {
        let now = crate::Timestamp::parse_db(case["now"].as_str().unwrap()).unwrap();
        let text = case["text"].as_str().unwrap();
        let zone = case["zone"].as_str().unwrap();
        assert_eq!(
            stamp(slash::time_parser::parse(text, zone, now)),
            case["parse"],
            "{case}"
        );
        let leading = slash::time_parser::split_leading_time(text, zone, now)
            .map(|(t, s)| json!([stamp(Some(t)), s]))
            .unwrap_or(Value::Null);
        assert_eq!(leading, case["leading"], "{case}");
        let (title, t) = slash::time_parser::split_trailing_time(text, zone, now);
        assert_eq!(json!([title, stamp(t)]), case["trailing"], "{case}");
    }
}
#[test]
fn slash_review_parser_differential_matches_rails() {
    let vectors = review_vectors();
    parser_differential(&vectors["parsing"], "Review");
    parser_differential(&vectors["supplemental_parsing"], "Supplemental");
}
fn parser_differential(cases: &Value, label: &str) {
    let mut passed = 0;
    let mut failed = 0;
    for case in cases.as_array().unwrap() {
        let now = crate::Timestamp::parse_db(case["now"].as_str().unwrap()).unwrap();
        let text = case["text"].as_str().unwrap();
        let zone = case["zone"].as_str().unwrap();
        let leading = slash::time_parser::split_leading_time(text, zone, now)
            .map(|(t, s)| json!([stamp(Some(t)), s]))
            .unwrap_or(Value::Null);
        let (title, t) = slash::time_parser::split_trailing_time(text, zone, now);
        let actual = json!({
            "parse": stamp(slash::time_parser::parse(text, zone, now)),
            "leading": leading, "trailing": [title, stamp(t)]
        });
        if ["parse", "leading", "trailing"]
            .iter()
            .any(|key| actual[*key] != case[*key])
        {
            failed += 1;
            eprintln!("{}", json!({"case":case,"rust":actual}));
        } else {
            passed += 1;
        }
    }
    println!("{label} parser differential: {passed} passed; {failed} failed");
    assert_eq!(failed, 0, "Rails review parser differential");
}
#[test]
fn slash_registry_and_recognition_match_rails() {
    assert_eq!(
        serde_json::to_value(slash::registry()).unwrap(),
        vectors()["registry"]
    );
    for case in vectors()["commands"].as_array().unwrap() {
        assert_eq!(
            slash::command_text(case["text"].as_str().unwrap()),
            case["recognized"].as_bool().unwrap(),
            "{case}"
        );
    }
}
fn run_case(case: &Value) -> (TestDb, Value) {
    let t = TestDb::with_clock(
        TestClock::frozen_at(crate::Timestamp::parse_db(case["now"].as_str().unwrap()).unwrap()),
        4,
    );
    let case = case.clone();
    let result=t.write(move |tx| {
        let user=identify("david");let room=identify("watercooler");
        tx.conn().execute("UPDATE users SET time_zone=?,updated_at=? WHERE id=?",rusqlite::params![case["zone"].as_str().unwrap(),tx.now(),user])?;
        for (key,value) in case["initial"].as_object().unwrap() {
            let value=match value {Value::Null=>rusqlite::types::Value::Null,Value::Bool(v)=>rusqlite::types::Value::Integer(i64::from(*v)),Value::Number(v)=>rusqlite::types::Value::Integer(v.as_i64().unwrap()),Value::String(v)=>rusqlite::types::Value::Text(v.clone()),Value::Object(_)|Value::Array(_)=>rusqlite::types::Value::Text(value.to_string())};
            tx.conn().execute(&format!("UPDATE users SET {key}=? WHERE id=?"),rusqlite::params![value,user])?;
        }
        if !case["intervals"].is_null() {tx.conn().execute("INSERT INTO calendar_meeting_caches (user_id,ooo_intervals,busy_intervals,fetched_at,created_at,updated_at) VALUES (?,?,'[]',?,?,?)",rusqlite::params![user,case["intervals"].to_string(),tx.now(),tx.now(),tx.now()])?;}
        let place=case["place"].as_str().unwrap();
        if place=="board" {tx.conn().execute("UPDATE rooms SET type='Rooms::Board' WHERE id=?",[room])?;}
        let thread=if place=="thread" || place=="locked" {
            let th=crate::ChannelThread::create(tx,crate::NewChannelThread{room_id:room,creator_id:user,name:Some("Slash test".into()),..Default::default()})?;
            if place=="locked" {tx.conn().execute("UPDATE channel_threads SET locked_at=? WHERE id=?",rusqlite::params![tx.now(),th.id])?;}
            Some(th.id)
        }else{None};
        let result=slash::dispatch(tx,&Context{user_id:user,room_id:room,thread_id:thread,huddles_configured:case["huddles"].as_bool().unwrap()},case["text"].as_str().unwrap());
        let (result,id)=match result {
            Ok(r)=>{let id=r.message_id;(serde_json::to_value(r).unwrap(),id)},
            Err(crate::Error::RecordInvalid(errors))=>(json!({"exception":"ActiveRecord::RecordInvalid","message":slash::sentence(errors.full_messages())}),None),
            Err(crate::Error::Other(message)) if message.starts_with("Invalid Timezone:")=>(json!({"exception":"ArgumentError","message":message}),None),
            Err(error)=>return Err(error),
        };
        let mut user_state=serde_json::Map::new();
        for key in ["custom_status_emoji","custom_status_text","custom_status_expires_at","dnd_enabled","dnd_until","ooo_until","ooo_note","ooo_broadcast","updated_at"] {
            let value:rusqlite::types::Value=tx.conn().query_row(&format!("SELECT {key} FROM users WHERE id=?"),[user],|r|r.get(0))?;
            user_state.insert(key.into(),match value {rusqlite::types::Value::Null=>Value::Null,rusqlite::types::Value::Integer(n)=>json!(n!=0),rusqlite::types::Value::Text(s)=>json!(s),_=>unreachable!()});
        }
        let message=if let Some(id)=id {let m=crate::Message::find(tx.conn(),id)?;json!({"action":m.action,"system_note":m.system_note,"streaming":m.streaming,"markdown_source":m.markdown_source,"created_at":m.created_at.to_db(),"updated_at":m.updated_at.to_db()})}else{Value::Null};
        let saved=if let Some(id)=id {crate::SavedItem::find_by_user_and_message(tx.conn(),user,id)?.map(|s|json!({"remind_at":stamp(s.remind_at),"status":s.status,"reminded_at":stamp(s.reminded_at),"created_at":s.created_at.to_db(),"updated_at":s.updated_at.to_db()})).unwrap_or(Value::Null)}else{Value::Null};
        let thread=if let Some(id)=thread {let th=crate::ChannelThread::find(tx.conn(),id)?;json!({"messages_count":th.messages_count,"closed_at":stamp(th.closed_at),"last_activity_at":th.last_activity_at.to_db(),"updated_at":th.updated_at.to_db()})}else{Value::Null};
        Ok(json!({"result":result,"user":user_state,"message":message,"saved":saved,"thread":thread}))
    });
    (t, result)
}
#[test]
fn slash_dispatch_and_rows_match_rails() {
    for case in vectors()["rows"].as_array().unwrap() {
        let (t, actual) = run_case(case);
        export_user_case(&t, case, &actual);
        for key in ["result", "user", "message", "saved", "thread"] {
            assert_eq!(actual[key], case[key], "{key}: {case}");
        }
    }
}
fn export_user_case(t: &TestDb, case: &Value, actual: &Value) {
    if case["place"] == "root"
        && actual["result"]["kind"] == "ephemeral"
        && let Ok(output) = std::env::var("WS8_SLASH_EXPORT_DIR")
    {
        std::fs::create_dir_all(&output).unwrap();
        let hash = crc32fast::hash(case.to_string().as_bytes());
        let path = std::path::Path::new(&output).join(format!("case-user-{hash}.sqlite3"));
        if path.exists() {
            std::fs::remove_file(&path).unwrap();
        }
        Connection::open(t.db.path())
            .unwrap()
            .execute("VACUUM INTO ?", [path.to_str().unwrap()])
            .unwrap();
        std::fs::write(
            path.with_extension("json"),
            json!({"now":case["now"],"user_id":identify("david"),"user":case["user"]}).to_string(),
        )
        .unwrap();
    }
}
fn callback_rows(events: &[Event]) -> (Value, Value) {
    use crate::broadcasts::Broadcast;
    let mut jobs = Vec::<String>::new();
    let mut broadcasts = Vec::new();
    for event in events {
        match event {
            Event::PushMessage { .. } => jobs.push("Room::PushMessageJob".into()),
            Event::DeliverWebhook { .. } => jobs.push("Bot::WebhookJob".into()),
            Event::Job(job) => jobs.push(job.class.into()),
            Event::Broadcast(request) if request.decode::<crate::models::user_status_settings::updates::StatusBadgeBroadcast>().is_some() => broadcasts.push(json!("status")),
            Event::Broadcast(_) => match event.as_broadcast() {
                Some(Broadcast::MessageCreated { .. }) => broadcasts.push(json!("message")),
                Some(Broadcast::UserStatus { .. }) => broadcasts.push(json!("status")),
                Some(broadcast) => if let Some((stream, payload)) = broadcast.channel_frame() { broadcasts.push(json!({"method":"cable","stream":stream,"payload":payload})); },
                None => {},
            },
            _ => {},
        }
    }
    jobs.sort();
    (json!(jobs), json!(broadcasts))
}
fn expected_callbacks(value: &Value) -> Value {
    json!(value.as_array().unwrap().iter().filter_map(|row| match row["partial"].as_str() {
        Some("messages/message") => Some(json!("message")),
        Some("users/statuses/badge") => Some(json!("status")),
        Some("rooms/show/ooo_notice_line") => None,
        _ => Some(row.clone()),
    }).collect::<Vec<_>>())
}
fn sorted_callbacks(value: &Value) -> Value {
    let mut rows = value.as_array().unwrap().clone();
    rows.sort_by_key(Value::to_string);
    json!(rows)
}
#[test]
fn slash_review_dispatch_and_rows_match_rails() {
    let mut passed = 0;
    let mut failed = 0;
    for case in review_vectors()["rows"].as_array().unwrap() {
        let (t, actual) = run_case(case);
        export_user_case(&t, case, &actual);
        let (jobs, broadcasts) = callback_rows(&t.events());
        let rows_match = ["result", "user", "message", "saved", "thread"]
            .iter()
            .all(|key| actual[*key] == case[*key]);
        if rows_match
            && jobs == case["jobs"]
            && sorted_callbacks(&broadcasts) == sorted_callbacks(&expected_callbacks(&case["broadcasts"]))
        {
            passed += 1;
        } else {
            failed += 1;
            eprintln!(
                "{}",
                json!({"case":case,"rust":actual,"jobs":jobs,"broadcasts":broadcasts})
            );
        }
    }
    println!("Review command differential: {passed} passed; {failed} failed");
    assert_eq!(failed, 0, "Rails review command differential");
}
#[test]
fn slash_callbacks_match_rails() {
    for case in vectors()["rows"].as_array().unwrap() {
        let (t, _) = run_case(case);
        let (jobs, broadcasts) = callback_rows(&t.events());
        assert_eq!(jobs, case["jobs"], "jobs: {case}");
        assert_eq!(
            sorted_callbacks(&broadcasts),
            sorted_callbacks(&expected_callbacks(&case["broadcasts"])),
            "broadcasts: {case}"
        );
    }
}

#[test]
fn slash_preexisting_user_validations_and_calendar_match_rails() {
    for case in vectors()["rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["stateful"] == true)
    {
        let (t, actual) = run_case(case);
        for key in ["result", "user", "message", "saved", "thread"] {
            assert_eq!(actual[key], case[key], "{key}: {case}");
        }
        let (jobs, broadcasts) = callback_rows(&t.events());
        assert_eq!(jobs, case["jobs"], "{case}");
        assert_eq!(
            sorted_callbacks(&broadcasts),
            sorted_callbacks(&expected_callbacks(&case["broadcasts"])),
            "{case}"
        );
    }
}
