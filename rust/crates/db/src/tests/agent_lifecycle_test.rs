use super::*;
use crate::models::{agent_lifecycle as lifecycle, audit_log::Context};
use crate::{
    Agent, AgentApproval, AgentGrant, ChannelThread, Message, NewApproval, NewGrant, Room,
    Timestamp, User,
};
use rusqlite::params;
use serde_json::{Value, json};
const ROOT: i64 = 900090001;
const REPLY: i64 = 900090002;
const THREAD: i64 = 900090010;
fn setup() -> TestDb {
    let t = super::channel_thread_test::frozen();
    t.clock
        .travel_to(Timestamp::parse_db("2026-03-02 16:00:00").unwrap());
    t.write(|tx| {
        tx.conn().execute("UPDATE agents SET owner_id=?,suspended_at=NULL WHERE id=?",params![id("david"),id("bender_agent")])?;
        Room::find(tx.conn(),id("watercooler"))?.grant_to(tx,&[id("bender")])?;
        tx.conn().execute("DELETE FROM agent_events",[])?;tx.conn().execute("DELETE FROM agent_grants",[])?;
        tx.conn().execute("DELETE FROM activity_items WHERE source_type='AgentApproval'",[])?;tx.conn().execute("DELETE FROM agent_approvals",[])?;
        Agent::find(tx.conn(),id("bender_agent"))?.unwrap().set_working_presence(tx,Some("Thinking"))?;
        AgentGrant::create(tx,NewGrant {agent_id:id("bender_agent"),capability:"read_messages".into(),room_id:Some(id("watercooler")),granted_by_id:id("david"),..Default::default()})?;
        tx.conn().execute("INSERT INTO messages(id,room_id,creator_id,markdown_source,client_message_id,streaming,streaming_updated_at,created_at,updated_at) VALUES (?,?,?,?,?,1,?,?,?)",params![ROOT,id("watercooler"),id("bender"),"Draft","ws11-quiet-root",tx.now(),tx.now(),tx.now()])?;
        tx.conn().execute("INSERT INTO channel_threads(id,room_id,creator_id,parent_message_id,name,last_activity_at,locked_at,created_at,updated_at) VALUES (?,?,?,?,?,?,?,?,?)",params![THREAD,id("watercooler"),id("david"),ROOT,"Quiet",tx.now(),tx.now(),tx.now(),tx.now()])?;
        tx.conn().execute("INSERT INTO messages(id,room_id,creator_id,thread_id,markdown_source,client_message_id,streaming,streaming_updated_at,created_at,updated_at) VALUES (?,?,?,?,?,?,1,?,?,?)",params![REPLY,id("watercooler"),id("bender"),THREAD,"Reply","ws11-quiet-reply",tx.now(),tx.now(),tx.now()])?;
        // lifecycle_contract.rb creates approvals with their final IDs. Keep those
        // IDs stable for the source snapshots captured by after_create_commit.
        tx.conn().execute_batch("DELETE FROM sqlite_sequence WHERE name='agent_approvals'; INSERT INTO sqlite_sequence(name,seq) VALUES ('agent_approvals',900090099);")?;
        for (fixed_id,summary,status) in [(900090100,"Pending","pending"),(900090101,"Due","pending"),(900090102,"Approved","approved")] {
            let a=AgentApproval::create(tx,NewApproval{agent_id:id("bender_agent"),action:"deploy".into(),summary:summary.into(),expires_at:Some(tx.now().since(jiff::SignedDuration::from_hours(1))),..Default::default()})?;
            assert_eq!(a.id,fixed_id);
            tx.conn().execute("UPDATE agent_approvals SET status=? WHERE id=?",params![status,a.id])?;
        }
        tx.conn().execute("UPDATE agent_approvals SET expires_at=? WHERE id=900090101",[tx.now().ago(jiff::SignedDuration::from_secs(1))])?;
        Ok(())
    });
    t.sink.take();
    t
}
fn audit(tx: &Tx<'_>) -> Result<Context> {
    Ok(Context {
        actor: Some((&User::find(tx.conn(), id("david"))?).into()),
        ..Default::default()
    })
}
fn snapshot(conn: &Connection, count: usize) -> Result<Value> {
    let agent = Agent::find(conn, id("bender_agent"))?.unwrap();
    let grants = crate::sql::query_all(
        conn,
        "SELECT revoked_at IS NOT NULL FROM agent_grants WHERE agent_id=? ORDER BY id",
        [agent.id],
        |r| r.get::<_, bool>(0),
    )?;
    let mut messages = Vec::new();
    for id in [ROOT, REPLY] {
        let m = Message::find(conn, id)?;
        messages.push(json!({"id":m.id,"streaming":m.streaming,"updated_at":crate::models::agent_payloads::json_time(m.updated_at),"streaming_updated_at":m.streaming_updated_at.map(crate::models::agent_payloads::json_time)}));
    }
    let approvals = crate::sql::query_all(
        conn,
        "SELECT id,status FROM agent_approvals ORDER BY id",
        [],
        |r| Ok(json!({"id":r.get::<_,i64>(0)?,"status":r.get::<_,String>(1)?})),
    )?;
    let audits = crate::sql::query_all(
        conn,
        "SELECT action,actor_id,actor_label,target_label,details FROM audit_logs WHERE target_type='Agent' AND target_id=? AND action IN ('agent.suspend','agent.kill_switch') ORDER BY id",
        [agent.id],
        |r| {
            Ok(
                json!({"action":r.get::<_,String>(0)?,"actor_id":r.get::<_,Option<i64>>(1)?,"actor_label":r.get::<_,Option<String>>(2)?,"target_label":r.get::<_,Option<String>>(3)?,"details":r.get::<_,Value>(4)?}),
            )
        },
    )?;
    Ok(
        json!({"cancelled":count,"suspended":agent.suspended(),"working_presence":agent.working_presence,"working_presence_expires_at":agent.working_presence_expires_at.map(crate::models::agent_payloads::json_time),"grants_revoked":grants,"messages":messages,"thread_count":ChannelThread::find(conn,THREAD)?.messages_count,"approvals":approvals,"event_count":conn.query_row("SELECT COUNT(*) FROM agent_events WHERE agent_id=?",[agent.id],|r|r.get::<_,i64>(0))?,"audits":audits}),
    )
}
#[test]
fn ws11_kill_switch_quiet_streams_expiry_cancellation_audit_and_idempotency_match_rails() {
    let t = setup();
    for key in ["first", "second"] {
        let count = t.write(|tx| {
            let audit = audit(tx)?;
            lifecycle::kill_switch(tx, id("bender_agent"), &audit)
        });
        t.read(move |conn| {
            assert_eq!(snapshot(conn, count)?, gold()["results"][key]);
            Ok(())
        });
    }
    t.write(|tx| {
        for id in [ROOT, REPLY] {
            assert!(!Message::find(tx.conn(), id)?.finalize_stream_quietly(tx)?);
        }
        Ok(())
    });
    t.read(|conn| {
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM message_search_index WHERE rowid IN (?,?)",params![ROOT,REPLY],|r|r.get::<_,i64>(0))?,0);
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM activity_items WHERE source_type='AgentApproval' AND source_id IN (900090100,900090101) AND handled_at IS NULL",[],|r|r.get::<_,i64>(0))?,0);Ok(())
    });
    let broadcasts = t
        .sink
        .take()
        .into_iter()
        .filter_map(|e| match e {
            Event::Broadcast(r) => r
                .decode::<crate::broadcasts::Broadcast>()
                .and_then(|r| r.ok()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(broadcasts.iter().filter(|b|matches!(b,crate::broadcasts::Broadcast::Turbo(s) if matches!(s.partial,Some(crate::broadcasts::Partial::MessageReplace{..})))).count(),2);
    assert_eq!(broadcasts.iter().filter(|b|matches!(b,crate::broadcasts::Broadcast::Turbo(s) if matches!(s.partial,Some(crate::broadcasts::Partial::ThreadIndicator{..})))).count(),1);
    assert!(!broadcasts.iter().any(|b|matches!(b,crate::broadcasts::Broadcast::Cable{stream,..} if stream.contains("_unread_"))));
}
fn gold() -> Value {
    serde_json::from_str(include_str!(
        "../../../../vectors/agents_lifecycle_contract.json"
    ))
    .unwrap()
}
#[test]
fn ws11_kill_switch_rollback_never_quiet_finalizes_or_revokes() {
    let t = setup();
    assert!(
        t.try_write(|tx| {
            let audit = audit(tx)?;
            lifecycle::kill_switch(tx, id("bender_agent"), &audit)?;
            Err::<(), _>(crate::Error::Other("WS11 rollback".into()))
        })
        .is_err()
    );
    t.read(|conn| {
        assert!(!Agent::find(conn, id("bender_agent"))?.unwrap().suspended());
        assert!(Message::find(conn, ROOT)?.streaming);
        assert!(Message::find(conn, REPLY)?.streaming);
        assert_eq!(
            conn.query_row(
                "SELECT COUNT(*) FROM agent_grants WHERE revoked_at IS NOT NULL",
                [],
                |r| r.get::<_, i64>(0)
            )?,
            0
        );
        assert_eq!(
            AgentApproval::find(conn, 900090100)?.unwrap().status,
            "pending"
        );
        Ok(())
    });
    assert!(t.sink.take().is_empty());
}
#[test]
fn ws11_suspend_continues_after_one_quiet_failure_and_owner_deactivation_uses_it() {
    let t = setup();
    t.write(|tx| {
        tx.conn().execute_batch("CREATE TRIGGER ws11_fail_one_finalize BEFORE UPDATE OF streaming ON messages WHEN OLD.id=900090001 AND NEW.streaming=0 BEGIN SELECT RAISE(ABORT,'WS11 one quiet failure'); END;")?;
        let audit=audit(tx)?;User::find(tx.conn(),id("david"))?.deactivate_with_audit(tx,&audit)
    });
    t.read(|conn| {
        assert!(Agent::find(conn, id("bender_agent"))?.unwrap().suspended());
        assert!(Message::find(conn, ROOT)?.streaming);
        assert!(!Message::find(conn, REPLY)?.streaming);
        assert_eq!(ChannelThread::find(conn, THREAD)?.messages_count, 1);
        Ok(())
    });
}
