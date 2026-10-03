//! Each original human handoff declaration, with its full persisted assertion projection.
use super::*;
use crate::models::channel_thread::{WORK_UPDATE_FORBIDDEN, WorkChanges};
use crate::models::{agent_delivery::AgentEvent, audit_log};
use crate::{
    Agent, ChannelThread, HandoffPackage, NewChannelThread, Room, RoomType, User, WorkHandoff,
    WorkThreadEvent,
};
use rusqlite::params;
use serde_json::{Value, json};

fn run(key: &'static str) {
    let oracle: Value =
        serde_json::from_str(include_str!("../../../../vectors/ws12_handoff_named.json")).unwrap();
    let expected = oracle["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["key"] == key)
        .unwrap()["facts"]
        .clone();
    let t = channel_thread_test::frozen();
    t.clock.travel_to(crate::Timestamp::from_second(1772467200));
    let (board_id,thread_id,previous)=t.write(move|tx|{
        let board=Room::create_for(tx,RoomType::Board,Some("Launch"),id("david"),&[id("david"),id("jz"),id("bender")])?;
        for cap in ["read_messages","post_messages","manage_threads"] {
            tx.conn().execute("INSERT INTO agent_grants(agent_id,room_id,capability,granted_by_id,created_at,updated_at) VALUES(?,?,?,?,?,?)",params![id("bender_agent"),board.id,cap,id("david"),tx.now(),tx.now()])?;
        }
        let mut thread=ChannelThread::create_board_post(tx,NewChannelThread{room_id:board.id,creator_id:id("david"),name:Some("Ship it".into()),work_status:Some("in_progress".into()),work_owner_id:Some(id("david")),..Default::default()},None)?;
        let manager=User::find(tx.conn(),id("david"))?;
        let mut previous=id("david");
        match key {
            "unassign"=>{
                previous=901840001;
                tx.conn().execute("INSERT INTO users(id,name,email_address,role,status,created_at,updated_at) VALUES(?,'Clippy','clippy@example.com',2,0,?,?)",params![previous,tx.now(),tx.now()])?;
                tx.conn().execute("INSERT INTO agents(id,user_id,owner_id,kind,status,created_at,updated_at) VALUES(901840002,?,?,'workspace','idle',?,?)",params![previous,id("david"),tx.now(),tx.now()])?;
                board.grant_to(tx,&[previous])?;
                for cap in ["read_messages","post_messages"] {tx.conn().execute("INSERT INTO agent_grants(agent_id,room_id,capability,granted_by_id,created_at,updated_at) VALUES(?,?,?,?,?,?)",params![901840002,board.id,cap,id("david"),tx.now(),tx.now()])?;}
                thread.update_work(tx,&manager,WorkChanges{owner_id:Some(json!(previous)),..Default::default()})?;
            }
            "current_owner"|"stale_agent"=>{
                thread.update_work(tx,&manager,WorkChanges{owner_id:Some(json!(id("bender"))),..Default::default()})?;
                if key=="stale_agent" {thread.update_work(tx,&manager,WorkChanges{owner_id:Some(json!(id("jz"))),..Default::default()})?;}
            }
            "stale_human"=>{
                thread.update_work(tx,&manager,WorkChanges{owner_id:Some(json!(id("jz"))),..Default::default()})?;
                tx.conn().execute("DELETE FROM memberships WHERE room_id=? AND user_id=?",params![board.id,id("jz")])?;
            }
            "untracked"=>thread=ChannelThread::create(tx,NewChannelThread{room_id:id("designers"),creator_id:id("david"),name:Some("Chat".into()),..Default::default()})?,
            "receiver_access"=>{tx.conn().execute("UPDATE agent_grants SET revoked_at=? WHERE agent_id=? AND room_id=? AND capability='post_messages'",params![tx.now(),id("bender_agent"),board.id])?;}
            _=>()
        }
        Ok((board.id,thread.id,previous))
    });
    let (before_event, before_ledger) = t.read(move |conn| {
        Ok((
            conn.query_row(
                "SELECT COALESCE(MAX(id),0) FROM work_thread_events",
                [],
                |r| r.get::<_, i64>(0),
            )?,
            conn.query_row("SELECT COALESCE(MAX(id),0) FROM agent_events", [], |r| {
                r.get::<_, i64>(0)
            })?,
        ))
    });
    t.sink.take();
    let error = t.write(move |tx| {
        let mut thread = ChannelThread::find(tx.conn(), thread_id)?;
        let sender = User::find(
            tx.conn(),
            id(match key {
                "stale_agent" => "bender",
                "stale_human" => "jz",
                _ => "david",
            }),
        )?;
        if key == "stale_sender_profile" {
            tx.conn().execute(
                "UPDATE users SET name='Renamed sender' WHERE id=?",
                [sender.id],
            )?;
        }
        let package = match key {
            "history" => HandoffPackage {
                summary: "Halfway there".into(),
                links: json!(["https://example.com/spec"]),
                open_questions: json!(["Which API?"]),
            },
            "ledger" => HandoffPackage {
                summary: "Halfway".into(),
                links: json!(["https://example.com/a"]),
                open_questions: json!(["Why?"]),
            },
            "package" => HandoffPackage {
                summary: "x".repeat(2001),
                ..Default::default()
            },
            _ => HandoffPackage {
                summary: "Yours now".into(),
                ..Default::default()
            },
        };
        Ok(
            match thread.hand_off(
                tx,
                &sender,
                &Agent::find(tx.conn(), id("bender_agent"))?.unwrap(),
                package,
                &audit_log::Context::default(),
            ) {
                Ok(_) => Value::Null,
                Err(crate::Error::RecordInvalid(errors)) => {
                    json!({"kind":"invalid","messages":errors.full_messages()})
                }
                Err(crate::Error::RecordNotFound(_)) => json!({"kind":"not_found"}),
                Err(crate::Error::Other(message)) if message == WORK_UPDATE_FORBIDDEN => {
                    json!({"kind":"forbidden"})
                }
                Err(error) => return Err(error),
            },
        )
    });
    let webhook_jobs = t
        .sink
        .take()
        .iter()
        .filter(|e| matches!(e,crate::Event::Job(job) if job.class=="Agent::EventWebhookJob"))
        .count();
    let actual=t.read(move|conn|{
        let thread=ChannelThread::find(conn,thread_id)?;
        let handoffs=WorkHandoff::for_thread(conn,thread_id)?;
        let handoff=handoffs.first();
        let history=WorkThreadEvent::for_thread(conn,thread_id)?.iter().filter(|e|e.id>before_event).map(|e|json!({"kind":e.event_type,"actor":e.actor_id,"from_owner_is_previous":e.from_owner_id==Some(previous),"to_owner":e.to_owner_id,"handoff_matches":e.metadata["handoff_id"].as_i64()==handoff.map(|h|h.id),"summary":e.metadata.get("handoff_summary"),"links_count":e.metadata.get("handoff_links_count"),"questions_count":e.metadata.get("handoff_questions_count")})).collect::<Vec<_>>();
        let ledger_ids=crate::sql::query_all(conn,"SELECT id FROM agent_events WHERE id>? ORDER BY id",[before_ledger],|r|r.get::<_,i64>(0))?;
        let ledger=ledger_ids.into_iter().map(|id|{let e=AgentEvent::find(conn,id)?.unwrap();Ok(json!({"receiver":e.agent_id==super::id("bender_agent"),"kind":e.event_type,"outcome":e.outcome,"room_matches":e.room_id==Some(board_id),"actor":e.actor_id,"thread_matches":e.metadata["thread_id"].as_i64()==Some(thread_id),"summary":e.metadata["handoff"].get("summary"),"links":e.metadata["handoff"].get("links"),"questions":e.metadata["handoff"].get("open_questions"),"webhook":e.webhook_status}))}).collect::<Result<Vec<_>>>()?;
        let audit=crate::sql::query_all(conn,"SELECT actor_id,target_type,target_id,details FROM audit_logs WHERE action='work.handoff' AND target_id=?",[thread_id],|r|{let details:Value=serde_json::from_str(&r.get::<_,String>(3)?).unwrap();Ok(json!({"actor":r.get::<_,Option<i64>>(0)?,"target_type":r.get::<_,String>(1)?,"target_matches":r.get::<_,i64>(2)?==thread_id,"to_owner":details["to_owner"],"from_owner":details["from_owner"]}))})?;
        Ok(json!({"error":error,"owner":thread.work_owner_id,"handoff_count":handoffs.len(),"handoff_thread_matches":handoff.map(|h|h.channel_thread_id==thread_id),"history":history,"ledger":ledger,"audit":audit,"webhook_jobs":webhook_jobs}))
    });
    assert_eq!(actual, expected, "human handoff declaration {key}");
}
macro_rules! cases {($($name:ident=>$key:literal),* $(,)?)=>{$(#[test] fn $name(){run($key);})*};}
cases! {
ws12_human_handoff_named_history=>"history",
ws12_human_handoff_named_ledger=>"ledger",
ws12_human_handoff_named_unassign=>"unassign",
ws12_human_handoff_named_audit=>"audit",
ws12_human_handoff_named_webhook=>"webhook",
ws12_human_handoff_named_current_owner=>"current_owner",
ws12_human_handoff_named_stale_agent=>"stale_agent",
ws12_human_handoff_named_stale_human=>"stale_human",
ws12_human_handoff_named_untracked=>"untracked",
ws12_human_handoff_named_package=>"package",
ws12_human_handoff_named_receiver_access=>"receiver_access",
ws12_human_handoff_stale_sender_profile_keeps_current_owner_audit=>"stale_sender_profile",
}
