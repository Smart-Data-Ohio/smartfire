//! Named WorkPayload reader comparisons. Persisted work fixtures represent Rails'
//! producer output; WS12 still owns mutation callbacks and assignment validation.
use super::*;
use crate::models::agent_payloads::{RepositoryAccess, work_payload};
use crate::{ChannelThread, ThreadTag, Timestamp};
use rusqlite::params;
use serde_json::Value;
fn setup() -> TestDb {
    let t = super::channel_thread_test::frozen();
    t.clock
        .travel_to(Timestamp::parse_db("2026-03-02 16:00:00").unwrap());
    t.write(|tx| {
        tx.conn().execute("INSERT INTO rooms(id,type,creator_id,name,created_at,updated_at) VALUES (900150010,'Rooms::Board',?,'Launch',?,?)",params![id("david"),tx.now(),tx.now()])?;
        for (tid,room,title,status,owner,result,run_url) in [
            (900150001,id("watercooler"),"Payload work","in_progress",Some(id("bender")),Some("## Outcome"),None),
            (900150002,id("watercooler"),"Human work","planned",Some(id("jason")),None,None),
            (900150003,id("watercooler"),"Unowned work","planned",None,None,None),
            (900150004,900150010,"Board payload","in_progress",Some(id("bender")),None,Some("https://example.com/runs/1")),
        ] {
            tx.conn().execute("INSERT INTO channel_threads(id,room_id,creator_id,name,last_activity_at,work_status,work_owner_id,result_markdown,result_updated_at,result_updated_by_id,run_url,created_at,updated_at) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?)",params![tid,room,if tid==900150004 {id("bender")} else {id("david")},title,tx.now(),status,owner,result,result.map(|_|tx.now()),result.map(|_|id("david")),run_url,tx.now(),tx.now()])?;
        }
        ThreadTag::create(tx,900150004,"api")?;ThreadTag::create(tx,900150004,"launch")?;Ok(())
    });
    t
}
fn compare(t: &TestDb, id: i64, key: &str) {
    t.read(|conn| {
        let oracle: Value = serde_json::from_str(include_str!(
            "../../../../vectors/agents_work_payload_named_cases.json"
        ))
        .unwrap();
        assert_eq!(
            work_payload(
                conn,
                &ChannelThread::find(conn, id)?,
                None,
                &RepositoryAccess::default()
            )?,
            oracle["results"][key]
        );
        Ok(())
    });
}
#[test]
fn ws11_work_payload_case_complete_thread_shape() {
    compare(&setup(), 900150001, "thread");
}
#[test]
fn ws11_work_payload_case_human_and_null_owners() {
    let t = setup();
    compare(&t, 900150002, "human");
    compare(&t, 900150003, "unowned");
}
#[test]
fn ws11_work_payload_case_board_tags_run_and_links() {
    compare(&setup(), 900150004, "board");
}
