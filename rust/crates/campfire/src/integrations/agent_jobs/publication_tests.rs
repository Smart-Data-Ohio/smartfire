//! Real reader/writer connections interleaved at the deletion commit boundary.
use crate::controllers::presenters::test_support::{ALL_TALK, BENDER, DAVID, TestApp};
use campfire_db::models::{agent_delivery::{AgentEvent, NewEvent}, agent_event_polling};
use campfire_db::{Agent, ChannelThread, NewChannelThread, Room, Timestamp};
use serde_json::{Value, json};
use std::time::Duration;

fn types(page: &Value) -> Vec<Value> {
    page["events"].as_array().unwrap().iter().map(|event|event["event_type"].clone()).collect()
}

#[tokio::test]
async fn ws11_publication_concurrent_poller_resumes_after_interleaved_commits() {
    let (app,_dir)=TestApp::boot().await.expect("default seed").stop_jobs().await;
    let (agent_id,thread_id,since)=app.db.write(|tx| {
        let agent=Agent::for_user(tx.conn(),BENDER)?.unwrap();
        tx.conn().execute("DELETE FROM agent_grants WHERE agent_id=?",[agent.id])?;
        Room::find(tx.conn(),ALL_TALK)?.grant_to(tx,&[BENDER])?;
        assert!(campfire_db::Webhook::find_by_user(tx.conn(),BENDER)?.is_some());
        let thread=ChannelThread::create(tx,NewChannelThread {room_id:ALL_TALK,creator_id:DAVID,
            name:Some("Concurrent deletion publication".into()),work_status:Some("planned".into()),..Default::default()})?;
        tx.conn().execute("UPDATE channel_threads SET work_owner_id=? WHERE id=?",rusqlite::params![BENDER,thread.id])?;
        Ok((agent.id,thread.id,tx.conn().query_row("SELECT COALESCE(max(id),0) FROM agent_events",[],|r|r.get::<_,i64>(0))?))
    }).await.unwrap();
    let (committed_tx,committed_rx)=tokio::sync::oneshot::channel();
    let (release_tx,release_rx)=std::sync::mpsc::channel();
    let db=app.db.clone();
    let deleting=tokio::spawn(async move {db.write(move |tx| {
        tx.after_commit(move |_| {
            let _=committed_tx.send(());
            release_rx.recv_timeout(Duration::from_secs(20)).map_err(|e|campfire_db::Error::Other(e.to_string()))?;
            Ok(())
        });
        ChannelThread::find(tx.conn(),thread_id)?.destroy(tx)?;
        AgentEvent::create(tx,NewEvent {agent_id,room_id:Some(ALL_TALK),event_type:"github_action_completed".into(),
            outcome:Some("delivered".into()),metadata:json!({"status":"completed"}),..Default::default()})?;
        Ok(())
    }).await});
    tokio::time::timeout(Duration::from_secs(10),committed_rx).await.unwrap().unwrap();
    let now=Timestamp::parse_db("2026-03-02 16:00:00").unwrap();
    let first=app.db.read(move |conn| agent_event_polling::poll(conn,agent_id,Some(&json!(since)),None,
        now,&Default::default(),|message|Ok(json!({"id":message.id})))).await;
    // A second real writer commits while the primary writer's after-commit callback waits.
    let path=app.db.path().to_path_buf();
    let env=app.db.env().clone();
    let interleaved=tokio::task::spawn_blocking(move || {
        let conn=campfire_db::Connection::open(path)?;
        campfire_db::run_write(&conn,&env,|tx| Ok(AgentEvent::create(tx,NewEvent {agent_id,
            room_id:Some(ALL_TALK),event_type:"fizzy_action_completed".into(),outcome:Some("delivered".into()),
            metadata:json!({"status":"completed"}),..Default::default()})?.id))
    }).await;
    let cursor=first.as_ref().ok().and_then(|p|p["next_since"].as_i64()).unwrap_or(since);
    let second=app.db.read(move |conn| agent_event_polling::poll(conn,agent_id,Some(&json!(cursor)),None,
        now,&Default::default(),|message|Ok(json!({"id":message.id})))).await;
    // Release before checking any results: a failed assertion cannot strand the writer.
    release_tx.send(()).unwrap();
    deleting.await.unwrap().unwrap();
    let first=first.unwrap();
    let second=second.unwrap();
    let interleaved=interleaved.unwrap().unwrap();
    let cursor=second["next_since"].clone();
    let actual=app.db.read(move |conn| {
        let deleted:i64=conn.query_row("SELECT id FROM agent_events WHERE agent_id=? AND event_type='work_unassigned' AND json_extract(metadata,'$.thread_id')=?",
            rusqlite::params![agent_id,thread_id],|r|r.get(0))?;
        let resumed=agent_event_polling::poll(conn,agent_id,Some(&cursor),None,now,&Default::default(),|message|Ok(json!({"id":message.id})))?;
        Ok(json!({"first_types":types(&first),"second_types":types(&second),"second_cursor_is_interleaved_event":second["next_since"]==interleaved,
            "deletion_id_after_interleaved_event":deleted>interleaved,"resumed_types":types(&resumed),"resumed_cursor_is_deletion":resumed["next_since"]==deleted,
            "deletion_count":conn.query_row("SELECT COUNT(*) FROM agent_events WHERE agent_id=? AND event_type='work_unassigned' AND json_extract(metadata,'$.thread_id')=?",rusqlite::params![agent_id,thread_id],|r|r.get::<_,i64>(0))?,
            "thread_exists":ChannelThread::find_by_id(conn,thread_id)?.is_some()}))
    }).await.unwrap();
    println!("WS11 deletion publication concurrent commits: {actual}");
    let oracle:Value=serde_json::from_str(include_str!("../../../../../vectors/agents_deletion_publication_contract.json")).unwrap();
    assert_eq!(actual,oracle["results"]["concurrent_commits"]);
}
