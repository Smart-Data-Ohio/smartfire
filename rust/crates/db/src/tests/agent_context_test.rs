use super::*;
use crate::models::agent_context;
use crate::{AgentGrant, NewGrant, Timestamp};
use rusqlite::params;
use serde_json::{Value, json};
const ROOM: i64 = 900080020;
const ROOT: i64 = 900080001;
const THREAD: i64 = 900080010;
fn setup() -> TestDb {
    let t = super::channel_thread_test::frozen();
    t.clock
        .travel_to(Timestamp::parse_db("2026-03-02 16:00:00").unwrap());
    t.write(|tx| {
        tx.conn().execute("DELETE FROM agent_grants",[])?;
        tx.conn().execute("INSERT INTO rooms(id,type,name,creator_id,created_at,updated_at) VALUES (?,'Rooms::Open','Context room',?,?,?)",params![ROOM,id("david"),tx.now(),tx.now()])?;
        crate::Room::find(tx.conn(),ROOM)?.grant_to(tx,&[id("david"),id("bender")])?;
        tx.conn().execute("INSERT INTO channel_threads(id,room_id,creator_id,parent_message_id,name,last_activity_at,created_at,updated_at) VALUES (?,?,?,?,?,?,?,?)",params![THREAD,ROOM,id("david"),Option::<i64>::None,"Context",tx.now(),tx.now(),tx.now()])?;
        for (n,creator,thread,streaming) in [(0,id("david"),None,false),(1,id("david"),Some(THREAD),false),(2,id("bender"),Some(THREAD),false),(3,id("david"),Some(THREAD),true)] {
            tx.conn().execute("INSERT INTO messages(id,room_id,creator_id,thread_id,streaming,client_message_id,created_at,updated_at) VALUES (?,?,?,?,?,?,?,?)",params![ROOT+n,ROOM,creator,thread,streaming,format!("ws11-context-{n}"),tx.now(),tx.now()])?;
        }
        tx.conn().execute("UPDATE channel_threads SET parent_message_id=? WHERE id=?",params![ROOT,THREAD])?;
        Ok(())
    });
    t
}
fn gold() -> Value {
    serde_json::from_str(include_str!(
        "../../../../vectors/agents_context_contract.json"
    ))
    .unwrap()
}
fn check(
    tx: &Tx<'_>,
    key: &str,
    message: Option<i64>,
    thread: Option<i64>,
    limit: Option<Value>,
) -> Result<()> {
    let result = agent_context::build(
        tx.conn(),
        id("bender_agent"),
        message,
        thread,
        limit.as_ref(),
        tx.now(),
        |m| {
            let creator = m.creator(tx.conn())?;
            Ok(
                json!({"id":m.id,"creator":{"id":creator.id,"name":creator.name,"preserved":"yes"},"streaming":m.streaming}),
            )
        },
    )?;
    assert_eq!(
        json!({"status":result.status,"payload":result.payload,"error":result.error}),
        gold()["results"][key],
        "{key}"
    );
    Ok(())
}
#[test]
fn ws11_context_window_authors_trigger_root_and_authorization_order_match_rails() {
    let t = setup();
    t.write(|tx| {
        for (key, message, thread, limit) in [
            ("required", None, None, None),
            ("missing_message", Some(0), None, None),
            ("missing_thread", None, Some(0), None),
            ("root", Some(ROOT), None, Some(json!("-2"))),
            ("thread", None, Some(THREAD), Some(json!("2tail"))),
            ("trigger", Some(ROOT + 2), None, Some(json!("bad"))),
            ("mismatch", Some(ROOT), Some(THREAD), None),
        ] {
            check(tx, key, message, thread, limit)?;
        }
        let mut grant = AgentGrant::create(
            tx,
            NewGrant {
                agent_id: id("bender_agent"),
                room_id: Some(ROOM),
                capability: "read_messages".into(),
                granted_by_id: id("david"),
                ..Default::default()
            },
        )?;
        grant.revoke(tx)?;
        check(tx, "revoked_mismatch", Some(ROOT), Some(THREAD), None)?;
        tx.conn().execute(
            "DELETE FROM memberships WHERE room_id=? AND user_id=?",
            params![ROOM, id("bender")],
        )?;
        check(tx, "nonmember_mismatch", Some(ROOT), Some(THREAD), None)?;
        Ok(())
    });
}
#[test]
fn ws11_context_limit_cap_soft_delete_and_nil_presenter() {
    let t = setup();
    t.write(|tx| {
        for n in 0..107 {
            tx.conn().execute("INSERT INTO messages(id,room_id,creator_id,thread_id,client_message_id,created_at,updated_at) VALUES (?,?,?,?,?,?,?)",params![ROOT+100+n,ROOM,id("david"),THREAD,format!("ws11-context-extra-{n}"),tx.now(),tx.now()])?;
        }
        let result = agent_context::build(
            tx.conn(),
            id("bender_agent"),
            None,
            Some(THREAD),
            Some(&json!(10000)),
            tx.now(),
            |_| Ok(Value::Null),
        )?;
        assert_eq!(result.payload.as_ref().unwrap()["messages"].as_array().unwrap().len(),100);
        assert_eq!(
            result.payload.as_ref().unwrap()["authors"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        tx.conn().execute(
            "UPDATE rooms SET deleted_at=? WHERE id=?",
            params![tx.now(), ROOM],
        )?;
        let result = agent_context::build(
            tx.conn(),
            id("bender_agent"),
            Some(ROOT),
            Some(THREAD),
            None,
            tx.now(),
            |_| panic!("unavailable room never presents"),
        )?;
        assert_eq!(result.status, 404);
        Ok(())
    });
}
