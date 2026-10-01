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

#[test]
fn ws11_review_deletion_event_preserves_captured_agent_after_outer_removal() {
    let t=setup();
    t.write(|tx| {tx.conn().execute("DELETE FROM webhooks WHERE user_id=?",[id("bender")])?;Ok(())});
    let result=t.try_write(|tx| {
        ChannelThread::find(tx.conn(),THREAD)?.destroy(tx)?;
        tx.conn().execute("DELETE FROM agents WHERE id=?",[id("bender_agent")])?;
        Ok(())
    });
    assert!(result.is_ok());
    let actual=t.read(|c|Ok(json!({"error":null,"thread_exists":ChannelThread::find_by_id(c,THREAD)?.is_some(),"agent_exists":crate::Agent::find(c,id("bender_agent"))?.is_some(),"events":c.query_row("SELECT COUNT(*) FROM agent_events WHERE json_extract(metadata,'$.thread_id')=?",[THREAD],|r|r.get::<_,i64>(0))?})));
    let gold:Value=serde_json::from_str(include_str!("../../../../vectors/agents_review_fixes_contract.json")).unwrap();
    println!("WS11 captured-agent deletion event: {actual}");
    assert_eq!(actual,gold["results"]["deletion_removed_agent"]);
}

#[test]
fn ws11_publication_deletion_id_follows_events_committed_in_parent() {
    use crate::models::agent_event_polling;
    use std::sync::{Arc, Mutex};
    let t = setup();
    let observed = Arc::new(Mutex::new(None));
    let capture = observed.clone();
    let later = t.write(move |tx| {
        assert!(crate::Webhook::find_by_user(tx.conn(), id("bender"))?.is_some());
        tx.after_commit(move |tx| {
            let page = agent_event_polling::poll(tx.conn(), id("bender_agent"), None, None,
                tx.now(), &Default::default(), |message| Ok(json!({"id":message.id})))?;
            *capture.lock().unwrap() = Some(page);
            Ok(())
        });
        ChannelThread::find(tx.conn(), THREAD)?.destroy(tx)?;
        Ok(AgentEvent::create(tx, NewEvent {agent_id:id("bender_agent"),
            room_id:Some(id("watercooler")), event_type:"github_action_completed".into(),
            outcome:Some("delivered".into()), metadata:json!({"status":"completed"}),
            ..Default::default()})?.id)
    });
    let first = observed.lock().unwrap().take().unwrap();
    let actual = t.read(move |conn| {
        let deleted = AgentEvent::for_agent(conn, id("bender_agent"))?.into_iter()
            .find(|event| event.event_type == "work_unassigned").unwrap();
        let resumed = agent_event_polling::poll(conn,id("bender_agent"),Some(&first["next_since"]),None,
            Timestamp::parse_db("2026-03-02 16:00:00").unwrap(),&Default::default(),|message| Ok(json!({"id":message.id})))?;
        Ok(json!({"first_types":first["events"].as_array().unwrap().iter().map(|e|e["event_type"].clone()).collect::<Vec<_>>(),
            "first_cursor_is_committed_event":first["next_since"]==later,
            "deletion_id_after_committed_event":deleted.id>later,
            "resumed_types":resumed["events"].as_array().unwrap().iter().map(|e|e["event_type"].clone()).collect::<Vec<_>>(),
            "resumed_cursor_is_deletion":resumed["next_since"]==deleted.id,
            "deletion_count":conn.query_row("SELECT COUNT(*) FROM agent_events WHERE event_type='work_unassigned'",[],|r|r.get::<_,i64>(0))?,
            "thread_exists":ChannelThread::find_by_id(conn,THREAD)?.is_some()}))
    });
    println!("WS11 deletion publication same transaction: {actual}");
    let oracle:Value=serde_json::from_str(include_str!("../../../../vectors/agents_deletion_publication_contract.json")).unwrap();
    assert_eq!(actual,oracle["results"]["same_transaction"]);
}

fn callback_order_oracle() -> Value {
    serde_json::from_str(include_str!("../../../../vectors/agents_deletion_callback_contract.json")).unwrap()
}
fn callback_thread(tx: &mut Tx<'_>, name: &str) -> Result<ChannelThread> {
    let thread = ChannelThread::create(tx, crate::NewChannelThread {
        room_id:id("watercooler"), creator_id:id("david"), name:Some(name.into()),
        work_status:Some("planned".into()), ..Default::default()
    })?;
    tx.conn().execute("UPDATE channel_threads SET work_owner_id=? WHERE id=?",params![id("bender"),thread.id])?;
    ChannelThread::find(tx.conn(),thread.id)
}
fn callback_observation(t: &TestDb) -> Value {
    t.read(|conn| {
        let events=AgentEvent::for_agent(conn,id("bender_agent"))?;
        let mut cursor=json!(0);let mut titles=Vec::new();
        loop {
            let page=crate::models::agent_event_polling::poll(conn,id("bender_agent"),Some(&cursor),Some(&json!(1)),
                Timestamp::parse_db("2026-03-02 16:00:00").unwrap(),&Default::default(),|m|Ok(json!({"id":m.id})))?;
            if page["events"].as_array().unwrap().is_empty() {break}
            titles.extend(page["events"].as_array().unwrap().iter().map(|e|e["work"]["title"].clone()));
            cursor=page["next_since"].clone();
        }
        Ok(json!({"ledger":events.iter().map(|e|e.metadata["title"].clone()).collect::<Vec<_>>(),"polled":titles,
            "threads_remaining":conn.query_row("SELECT COUNT(*) FROM channel_threads WHERE name IN ('A','B','A edited')",[],|r|r.get::<_,i64>(0))?}))
    })
}
fn callback_setup(webhook: bool) -> TestDb {
    let t=setup();
    t.write(move|tx| {
        tx.conn().execute("DELETE FROM channel_threads WHERE id=?",[THREAD])?;
        if !webhook {tx.conn().execute("DELETE FROM webhooks WHERE user_id=?",[id("bender")])?;}
        Ok(())
    });
    t
}
#[test]
fn ws11_r4_callback_created_first_matches_rails() {
    for webhook in [false,true] {
        let t=callback_setup(webhook);
        t.write(|tx| {let a=callback_thread(tx,"A")?;let b=callback_thread(tx,"B")?;b.destroy(tx)?;a.destroy(tx)});
        let actual=callback_observation(&t);
        println!("WS11 r4 created_first webhook={webhook}: {actual}");
        assert_eq!(actual,callback_order_oracle()["results"][format!("created_same_transaction_{webhook}")]);
    }
}
#[test]
fn ws11_r4_callback_updated_first_matches_rails() {
    for webhook in [false,true] {
        let t=callback_setup(webhook);
        let (mut a,b)=t.write(|tx|Ok((callback_thread(tx,"A")?,callback_thread(tx,"B")?)));
        t.write(move|tx| {a.update_settings(tx,Some("A edited"),None)?;b.destroy(tx)?;a.destroy(tx)});
        let actual=callback_observation(&t);
        println!("WS11 r4 updated_first webhook={webhook}: {actual}");
        assert_eq!(actual,callback_order_oracle()["results"][format!("updated_first_{webhook}")]);
    }
}
#[test]
fn ws11_r4_callback_plain_reverse_deletion_matches_rails() {
    for webhook in [false,true] {
        let t=callback_setup(webhook);
        let (a,b)=t.write(|tx|Ok((callback_thread(tx,"A")?,callback_thread(tx,"B")?)));
        t.write(move|tx| {b.destroy(tx)?;a.destroy(tx)});
        assert_eq!(callback_observation(&t),callback_order_oracle()["results"][format!("plain_{webhook}")]);
    }
}
#[test]
fn ws11_r4_callback_failure_stops_remaining_publications() {
    let t=callback_setup(false);
    let (a,b)=t.write(|tx|Ok((callback_thread(tx,"A")?,callback_thread(tx,"B")?)));
    let failed=t.try_write(move|tx| {
        tx.conn().execute_batch(&format!("CREATE TEMP TRIGGER ws11_reject_first BEFORE INSERT ON agent_events WHEN NEW.event_type='work_unassigned' AND json_extract(NEW.metadata,'$.thread_id')={} BEGIN SELECT RAISE(ABORT,'WS11 rejected ledger'); END",b.id))?;
        b.destroy(tx)?;a.destroy(tx)
    });
    assert!(failed.is_err());
    let actual=callback_observation(&t);
    println!("WS11 r4 first_callback_failure: {actual}");
    assert_eq!(actual,callback_order_oracle()["results"]["first_ledger_failure"]);
}

#[test]
fn ws11_r4_callback_savepoint_discards_rolled_back_deletion() {
    for webhook in [false,true] {
        let t=callback_setup(webhook);
        let (mut a,b)=t.write(|tx|Ok((callback_thread(tx,"A")?,callback_thread(tx,"B")?)));
        t.write(move|tx| {
            a.update_settings(tx,Some("A edited"),None)?;
            let failed=tx.savepoint(|tx| {let c=callback_thread(tx,"Rolled back C")?;c.destroy(tx)?;Err::<(),_>(crate::Error::Other("WS11 nested rollback".into()))});
            assert!(failed.is_err());
            assert!(ChannelThread::find_by_id(tx.conn(),a.id)?.is_some());
            b.destroy(tx)?;a.destroy(tx)
        });
        assert_eq!(callback_observation(&t),callback_order_oracle()["results"][format!("savepoint_{webhook}")]);
    }
}
