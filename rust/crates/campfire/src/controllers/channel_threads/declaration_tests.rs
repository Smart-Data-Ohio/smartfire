//! The remaining ordinary-thread declarations, using actual requests and SQL tracing.
use std::sync::Arc;
use axum::http::{Method, StatusCode};
use campfire_db::{ChannelThread, NewChannelThread, Timestamp};
use campfire_kit::clock::FrozenClock;
use crate::controllers::presenters::test_support::*;

async fn app() -> TestApp { TestApp::boot_with_test_clock(Arc::new(FrozenClock::new(SEED_NOW.parse().unwrap()))).await.unwrap() }

async fn add(app: &TestApp, n: i64) {
    app.db().write(move |tx| {
        for i in 0..n {
            let thread=ChannelThread::create(tx,NewChannelThread {room_id:ALL_TALK,creator_id:JASON,name:Some(format!("Query thread {i}")),..Default::default()})?;
            // Different creators and owners exercise both preload paths.
            tx.conn().execute("UPDATE channel_threads SET work_owner_id=?, work_status='planned' WHERE id=?",(if i%2==0 {DAVID}else{JASON},thread.id))?;
        }
        Ok(())
    }).await.unwrap();
}

async fn queries(app: &TestApp, path: &str) -> Vec<String> {
    let mut viewer=app.david();viewer.authenticity_token().await;
    let log=app.db().capture_read_queries();
    let response=viewer.get(path).await;
    app.db().stop_capturing_read_queries();
    assert_eq!(response.status,StatusCode::OK,"{}",response.text());
    let captured=log.lock().unwrap().clone();
    assert!(!captured.is_empty());captured
}

#[tokio::test]
async fn thread_index_query_count_stays_constant_as_threads_grow() {
    let app=app().await;
    add(&app,2).await;
    let path=format!("/rooms/{ALL_TALK}/threads");
    let small=queries(&app,&path).await;
    add(&app,4).await;
    let large=queries(&app,&path).await;
    assert_eq!(small.len(),large.len(),"small {} vs large {} reader SQL\n{large:#?}",small.len(),large.len());
}

#[tokio::test]
async fn closed_listing_uses_one_thread_query_including_locked_and_stale_without_writes() {
    let app=app().await;
    let ids=app.db().write(|tx| {
        let mut ids=Vec::new();
        for (name,hours,closed,locked) in [("Recent closed",1,true,false),("Locked",2,true,true),("Stale",3,false,false),("Old closed",4,true,false),("Active",0,false,false)] {
            let thread=ChannelThread::create(tx,NewChannelThread {room_id:ALL_TALK,creator_id:JASON,name:Some(name.into()),auto_archive_after_minutes:Some(60),..Default::default()})?;
            let activity=tx.now().since(jiff::SignedDuration::from_hours(-hours));
            tx.conn().execute("UPDATE channel_threads SET last_activity_at=?,closed_at=?,locked_at=? WHERE id=?",(activity,closed.then_some(activity),locked.then_some(activity),thread.id))?;
            ids.push(thread.id);
        }
        Ok(ids)
    }).await.unwrap();
    let path=format!("/rooms/{ALL_TALK}/threads.json?state=closed");
    let captured=queries(&app,&path).await;
    let selects=captured.iter().filter(|sql|sql.to_lowercase().contains("from \"channel_threads\"")||sql.to_lowercase().contains("from channel_threads")).collect::<Vec<_>>();
    assert_eq!(selects.len(),1,"{selects:#?}");
    let response=app.david().get(&path).await;
    let actual=response.json()["threads"].as_array().unwrap().iter().filter_map(|r|r["id"].as_i64()).collect::<Vec<_>>();
    assert_eq!(actual,ids[..4]);
    let oracle:serde_json::Value=serde_json::from_str(include_str!("../../../../../vectors/messaging/thread-declarations.json")).unwrap();
    let expected=oracle["rows"][0]["body"].as_str().unwrap();
    if response.text()!=expected {rails_mismatch(&response.text(),expected,"closed thread declaration");}
    let stale=ids[2];assert!(app.db().read(move |conn|ChannelThread::find(conn,stale)).await.unwrap().closed_at.is_none());
    let mut viewer=app.david();
    assert_eq!(viewer.write(Req::new(Method::POST,&format!("/rooms/{DIRECT_DAVID_JASON}/threads.json")).form(&[("thread[name]","Not permitted")])).await.status,StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn stale_unlock_resets_archive_clock_and_post_sweeps_stale_siblings() {
    let app=app().await;
    let (locked,stale,live)=app.db().write(|tx| {
        let mut locked=ChannelThread::create(tx,NewChannelThread {room_id:ALL_TALK,creator_id:JASON,name:Some("Locked stale".into()),auto_archive_after_minutes:Some(60),..Default::default()})?;
        locked.lock_conversation(tx)?;
        let stale=ChannelThread::create(tx,NewChannelThread {room_id:ALL_TALK,creator_id:JASON,name:Some("Stale sibling".into()),auto_archive_after_minutes:Some(60),..Default::default()})?;
        let live=ChannelThread::create(tx,NewChannelThread {room_id:ALL_TALK,creator_id:JASON,name:Some("Live sibling".into()),..Default::default()})?;
        let past=tx.now().since(jiff::SignedDuration::from_hours(-2));
        tx.conn().execute("UPDATE channel_threads SET last_activity_at=? WHERE id IN (?,?)",(past,locked.id,stale.id))?;
        Ok((locked.id,stale.id,live.id))
    }).await.unwrap();
    let mut viewer=app.david();
    let response=viewer.write(Req::new(Method::PATCH,&format!("/rooms/{ALL_TALK}/threads/{locked}.json")).form(&[("thread[status]","active")])).await;
    assert_eq!(response.status,StatusCode::OK,"{}",response.text());
    let record=app.db().read(move |conn|ChannelThread::find(conn,locked)).await.unwrap();
    assert!(record.locked_at.is_none()&&record.closed_at.is_none());
    assert_eq!(record.last_activity_at,Timestamp::from_jiff(SEED_NOW.parse().unwrap()));
    assert_eq!(viewer.write(Req::new(Method::POST,&format!("/rooms/{ALL_TALK}/threads/{live}/messages.json")).form(&[("message[markdown_source]","Hello"),("message[client_message_id]","sweep-trigger")])).await.status,StatusCode::CREATED);
    let record=app.db().read(move |conn|ChannelThread::find(conn,stale)).await.unwrap();assert!(record.closed_at.is_some());
}
