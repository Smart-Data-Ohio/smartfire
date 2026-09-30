use super::*;
use crate::slash_commands::{self as slash, Context};
use serde_json::{Value, json};
fn vectors() -> Value {
    serde_json::from_str(include_str!("ws8_slash_vectors.json")).unwrap()
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
        TestClock::frozen_at(crate::Timestamp::parse_db("2026-09-23 12:00:00").unwrap()),
        4,
    );
    let place = case["place"].as_str().unwrap().to_owned();
    let text = case["text"].as_str().unwrap().to_owned();
    let result=t.write(move |tx| {
        let user=identify("david"); let room=identify("watercooler");
        tx.conn().execute("UPDATE users SET time_zone='America/New_York',updated_at=? WHERE id=?",rusqlite::params![tx.now(),user])?;
        if place=="board" {tx.conn().execute("UPDATE rooms SET type='Rooms::Board' WHERE id=?",[room])?;}
        let thread=if place=="thread" || place=="locked" {let th=crate::ChannelThread::create(tx,crate::NewChannelThread{room_id:room,creator_id:user,name:Some("Slash test".into()),..Default::default()})?; if place=="locked" { tx.conn().execute("UPDATE channel_threads SET locked_at=? WHERE id=?",rusqlite::params![tx.now(),th.id])?; } Some(th.id)} else {None};
        t_clear_events(tx);
        let result=slash::dispatch(tx,&Context{user_id:user,room_id:room,thread_id:thread,huddles_configured:false},&text)?;
        let mut user_state=serde_json::Map::new();
        for key in ["custom_status_emoji","custom_status_text","custom_status_expires_at","dnd_enabled","dnd_until","ooo_until","ooo_note","ooo_broadcast","updated_at"] {
            let value: rusqlite::types::Value=tx.conn().query_row(&format!("SELECT {key} FROM users WHERE id=?"),[user],|r|r.get(0))?;
            user_state.insert(key.into(),match value {rusqlite::types::Value::Null=>Value::Null,rusqlite::types::Value::Integer(n)=>json!(n!=0),rusqlite::types::Value::Text(s)=>json!(s),_=>unreachable!()});
        }
        let message=if let Some(id)=result.message_id { let m=crate::Message::find(tx.conn(),id)?;json!({"action":m.action,"system_note":m.system_note,"streaming":m.streaming,"markdown_source":m.markdown_source,"created_at":m.created_at.to_db(),"updated_at":m.updated_at.to_db()})}else{Value::Null};
        let saved=if let Some(id)=result.message_id {crate::SavedItem::find_by_user_and_message(tx.conn(),user,id)?.map(|s|json!({"remind_at":stamp(s.remind_at),"status":s.status,"reminded_at":stamp(s.reminded_at)})).unwrap_or(Value::Null)}else{Value::Null};
        Ok(json!({"result":result,"user":user_state,"message":message,"saved":saved}))
    });
    (t, result)
}
fn t_clear_events(_tx: &mut Tx<'_>) {}
#[test]
fn slash_dispatch_and_rows_match_rails() {
    for case in vectors()["rows"].as_array().unwrap() {
        let (_t, actual) = run_case(case);
        for key in ["result", "user", "message", "saved"] {
            assert_eq!(actual[key], case[key], "{key}: {case}");
        }
    }
}
fn callback_rows(events: &[Event]) -> (Value, Value) {
    use crate::broadcasts::{Broadcast, Partial, TurboAction};
    let mut jobs = Vec::<String>::new();
    let mut broadcasts = Vec::<Value>::new();
    for event in events {
        match event {
            Event::PushMessage { .. } => jobs.push("Room::PushMessageJob".into()),
            Event::DeliverWebhook { .. } => jobs.push("Bot::WebhookJob".into()),
            Event::Job(job) => jobs.push(job.class.into()),
            Event::Broadcast(Broadcast::Cable { stream, payload }) => {
                broadcasts.push(json!({"method":"cable","stream":stream,"payload":payload}))
            }
            Event::Broadcast(Broadcast::Turbo(s)) => {
                let partial = match &s.partial {
                    Some(Partial::Message { .. }) => Some("messages/message"),
                    Some(Partial::UserStatus { .. }) => Some("users/statuses/badge"),
                    Some(Partial::OooNotice { .. }) => Some("rooms/show/ooo_notice_line"),
                    _ => None,
                };
                let method = match s.action {
                    TurboAction::Update => "broadcast_update_to",
                    TurboAction::Append => "broadcast_append_to",
                    TurboAction::Replace => "broadcast_replace_to",
                    _ => "other",
                };
                broadcasts.push(json!({"method":method,"streams":s.streamables.iter().map(|s|s.to_param()).collect::<Vec<_>>(),"target":s.target,"partial":partial}));
            }
            _ => {}
        }
    }
    jobs.sort();
    (json!(jobs), json!(broadcasts))
}
fn sorted_callbacks(value: &Value) -> Value {
    let mut rows = value.as_array().unwrap().clone();
    rows.sort_by_key(Value::to_string);
    json!(rows)
}
#[test]
fn slash_callbacks_match_rails() {
    for case in vectors()["rows"].as_array().unwrap() {
        let (t, _) = run_case(case);
        let (jobs, broadcasts) = callback_rows(&t.events());
        assert_eq!(jobs, case["jobs"], "jobs: {case}");
        assert_eq!(
            sorted_callbacks(&broadcasts),
            sorted_callbacks(&case["broadcasts"]),
            "broadcasts: {case}"
        );
    }
}
