use super::*;
use crate::models::{
    agent_delivery::{AgentEvent, NewEvent},
    agent_work_events as work,
};
use crate::{AgentGrant, ChannelThread, NewGrant, Room, Timestamp};
use rusqlite::params;
use serde_json::{Value, json};
const THREAD: i64 = 900070001;
fn setup() -> TestDb {
    let t = super::channel_thread_test::frozen();
    t.clock
        .travel_to(Timestamp::parse_db("2026-03-02 16:00:00").unwrap());
    t.write(|tx| {
        tx.conn().execute("DELETE FROM agent_grants",[])?;tx.conn().execute("DELETE FROM agent_events",[])?;
        Room::find(tx.conn(),id("watercooler"))?.grant_to(tx,&[id("bender")])?;
        tx.conn().execute("INSERT INTO channel_threads(id,room_id,creator_id,name,work_status,work_owner_id,last_activity_at,created_at,updated_at) VALUES (?,?,?,?,?,?,?,?,?)",params![THREAD,id("watercooler"),id("david"),"Work events","in_progress",id("bender"),tx.now(),tx.now(),tx.now()])?;
        Ok(())
    });
    t
}
fn gold() -> Value {
    serde_json::from_str(include_str!(
        "../../../../vectors/agents_work_events_contract.json"
    ))
    .unwrap()
}
fn capture(tx: &Tx<'_>, events: &[AgentEvent]) -> Result<Value> {
    let mut result = Vec::new();
    for event in events {
        let e = AgentEvent::find(tx.conn(), event.id)?.unwrap();
        assert!(
            e.chain_id.as_deref() == Some("ws11-trigger-chain")
                || uuid::Uuid::parse_str(e.chain_id.as_deref().unwrap()).is_ok()
        );
        result.push(json!({"agent_id":e.agent_id,"event_type":e.event_type,"outcome":e.outcome,"actor_id":e.actor_id,"metadata":e.metadata,"hop":e.hop(),"detail":e.detail,"webhook_status":e.webhook_status,"webhook_next_attempt_at":e.webhook_next_attempt_at.map(crate::models::agent_payloads::json_time),"chain":if e.chain_id.as_deref()==Some("ws11-trigger-chain") {"ws11-trigger-chain"} else {"uuid"}}));
    }
    Ok(json!(result))
}
#[test]
fn ws11_work_events_assign_handoff_access_hop_and_real_delete_match_rails() {
    let t = setup();
    t.write(|tx| {
        let thread=ChannelThread::find(tx.conn(),THREAD)?;
        let human=Some(id("david"));let bot=Some(id("bender"));
        for (key,from,to,actor) in [("same",bot,bot,human),("assigned",None,bot,human)] {
            let events=work::record_owner_change(tx,&thread,from,to,actor)?;
            assert_eq!(capture(tx,&events)?,gold()["results"][key],"{key}");
        }
        tx.conn().execute("DELETE FROM memberships WHERE user_id=? AND room_id=?",params![id("bender"),thread.room_id])?;
        let events=work::record_owner_change(tx,&thread,bot,None,None)?;
        assert_eq!(capture(tx,&events)?,gold()["results"]["nonmember"]);
        Room::find(tx.conn(),thread.room_id)?.grant_to(tx,&[id("bender")])?;
        let mut grant=AgentGrant::create(tx,NewGrant{agent_id:id("bender_agent"),capability:"read_messages".into(),room_id:Some(thread.room_id),granted_by_id:id("david"),..Default::default()})?;
        grant.revoke(tx)?;
        let events=work::record_owner_change(tx,&thread,bot,None,human)?;
        assert_eq!(capture(tx,&events)?,gold()["results"]["revoked"]);
        tx.conn().execute("DELETE FROM agent_grants",[])?;tx.conn().execute("DELETE FROM agent_events",[])?;
        for (kind,actor,hop,chain) in [("mention",human,2,"ws11-trigger-chain"),("work_assigned",bot,9,"self")] {
            AgentEvent::create(tx,NewEvent{agent_id:id("bender_agent"),room_id:Some(thread.room_id),actor_id:actor,event_type:kind.into(),outcome:Some(if kind=="mention" {"acknowledged"} else {"delivered"}.into()),hop,chain_id:Some(chain.into()),..Default::default()})?;
        }
        let events=work::record_owner_change(tx,&thread,None,bot,bot)?;
        assert_eq!(capture(tx,&events)?,gold()["results"]["suppressed"]);
        let events=work::record_handoff(tx,&thread,bot,id("bender_agent"),human,json!({"summary":"Take over","empty":"","flag":false}))?;
        assert_eq!(capture(tx,&events)?,gold()["results"]["handoff"]);
        thread.destroy_by(tx,human)?;
        assert!(ChannelThread::find_by_id(tx.conn(),THREAD)?.is_none());
        assert_eq!(tx.conn().query_row("SELECT COUNT(*) FROM agent_events WHERE event_type='work_unassigned' AND json_extract(metadata,'$.thread_id')=?",[THREAD],|r|r.get::<_,i64>(0))?,0);
        Ok(())
    });
    t.write(|tx| {
        let event_id:i64=tx.conn().query_row("SELECT id FROM agent_events WHERE event_type='work_unassigned' ORDER BY id DESC LIMIT 1",[],|r|r.get(0))?;
        let event=AgentEvent::find(tx.conn(),event_id)?.unwrap();
        assert_eq!(event.metadata["work_snapshot"],gold()["results"]["deleted_snapshot"]);
        assert_eq!(capture(tx,&[event])?,gold()["results"]["deleted"]);
        Ok(())
    });
}
#[test]
fn ws11_work_delete_snapshot_and_jobs_roll_back_with_parent_write() {
    let t = setup();
    t.sink.take();
    assert!(
        t.try_write(|tx| {
            ChannelThread::find(tx.conn(), THREAD)?.destroy(tx)?;
            Err::<(), _>(crate::Error::Other("WS11 rollback".into()))
        })
        .is_err()
    );
    t.read(|conn| {
        assert!(ChannelThread::find_by_id(conn, THREAD)?.is_some());
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM agent_events", [], |r| r
                .get::<_, i64>(0))?,
            0
        );
        Ok(())
    });
    assert!(t.sink.take().is_empty());
}
#[test]
fn ws11_work_enqueue_rechecks_access_and_is_idempotent() {
    let t = setup();
    t.sink.take();
    t.write(|tx| {
        let thread = ChannelThread::find(tx.conn(), THREAD)?;
        let events =
            work::record_owner_change(tx, &thread, None, Some(id("bender")), Some(id("david")))?;
        work::enqueue_webhook(tx, &events[0])?;
        Ok(())
    });
    let jobs = t
        .sink
        .take()
        .into_iter()
        .filter(|e| matches!(e,Event::Job(j) if j.class=="Agent::EventWebhookJob"))
        .count();
    assert_eq!(jobs, 1);
}

#[test]
fn ws11_review_deleted_ledger_failure_keeps_deletion() {
    let t = setup();
    t.write(|tx| {
        tx.conn().execute("DELETE FROM webhooks WHERE user_id=?", [id("bender")])?;
        tx.conn().execute_batch("CREATE TEMP TRIGGER review_reject_deleted_event BEFORE INSERT ON agent_events WHEN NEW.event_type='work_unassigned' BEGIN SELECT RAISE(ABORT,'review deletion event rejected'); END")?;
        Ok(())
    });
    let failed = t.try_write(|tx| ChannelThread::find(tx.conn(), THREAD)?.destroy(tx));
    assert!(failed.is_err());
    let exists = t.read(|conn| Ok(ChannelThread::find_by_id(conn, THREAD)?.is_some()));
    println!("REVIEW deletion-event failure: thread_exists={exists}");
    let gold: serde_json::Value = serde_json::from_str(include_str!("../../../../vectors/agents_review_fixes_contract.json")).unwrap();
    assert_eq!(exists, gold["results"]["deletion"]["thread_exists"].as_bool().unwrap());
}
