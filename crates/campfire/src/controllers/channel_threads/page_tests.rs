use std::sync::Arc;
use axum::http::StatusCode;
use campfire_db::{ChannelThread, Message, NewChannelThread, NewMessage, ThreadMembership};
use campfire_kit::clock::FrozenClock;
use serde_json::Value;
use crate::controllers::presenters::test_support::*;

fn oracle() -> Value { serde_json::from_str(include_str!("../../../../../vectors/messaging/thread-pages.json")).unwrap() }

#[tokio::test]
async fn pull_request_thread_without_starter_refreshes_once_and_survives_queue_failure() {
    let app = TestApp::boot_frozen().await.expect("default seed required").without_job_runner().await;
    app.db().write(|tx| {
        tx.conn().execute_batch("DELETE FROM background_jobs; UPDATE channel_threads SET parent_message_id=NULL WHERE id=8; UPDATE github_pull_requests SET fetched_at=NULL,fetch_requested_at=NULL WHERE id IN (SELECT github_pull_request_id FROM github_pull_request_threads WHERE channel_thread_id=8)")?;
        Ok(())
    }).await.unwrap();
    let mut browser = app.david();
    for _ in 0..2 {
        let response = browser.send(Req::new(axum::http::Method::GET, "/api/v1/threads/8").header("accept", "application/json")).await;
        assert_eq!(response.status, StatusCode::OK);
        let jobs = app.db().read(|conn| Ok(conn.query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='Github::FetchPullRequestJob'", [], |r| r.get::<_, i64>(0))?)).await.unwrap();
        assert_eq!(jobs, 1);
    }
    app.db().write(|tx| {
        tx.conn().execute_batch("DELETE FROM background_jobs; UPDATE github_pull_requests SET fetch_requested_at=NULL; CREATE TRIGGER reject_thread_pr_refresh BEFORE INSERT ON background_jobs WHEN NEW.job_class='Github::FetchPullRequestJob' BEGIN SELECT RAISE(ABORT,'queue unavailable'); END;")?;
        Ok(())
    }).await.unwrap();
    let response = browser.send(Req::new(axum::http::Method::GET, "/api/v1/threads/8").header("accept", "application/json")).await;
    assert_eq!(response.status, StatusCode::OK);
    app.db().read(|conn| {
        let claim: Option<campfire_db::Timestamp> = conn.query_row("SELECT fetch_requested_at FROM github_pull_requests WHERE id IN (SELECT github_pull_request_id FROM github_pull_request_threads WHERE channel_thread_id=8)", [], |r| r.get(0))?;
        assert!(claim.is_none());
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='Github::FetchPullRequestJob'", [], |r| r.get::<_, i64>(0))?, 0);
        Ok(())
    }).await.unwrap();
}

async fn fixture() -> (TestApp, i64, Vec<i64>) {
    let app = TestApp::boot_with_test_clock(Arc::new(FrozenClock::new(SEED_NOW.parse().unwrap()))).await.unwrap();
    let (parent, threads) = app.db().write(|tx| {
        let parent = Message::create(tx, NewMessage { room_id: ALL_TALK, creator_id: DAVID,
            markdown_source: Some("Page starter".into()), client_message_id: Some("pages-parent".into()), ..Default::default() })?;
        let names = ["Pages <&> thread", "Empty thread", "Stale thread", "Closed thread", "Locked thread", "Work thread"];
        let threads = names.iter().enumerate().map(|(index, name)| ChannelThread::create(tx, NewChannelThread {
            room_id: ALL_TALK, creator_id: JASON, name: Some((*name).into()), parent_message_id: (index == 0).then_some(parent.id),
            auto_archive_after_minutes: (index == 2).then_some(60), work_status: (index == 5).then_some("planned".into()), ..Default::default()
        })).collect::<campfire_db::Result<Vec<_>>>()?;
        let stale = tx.now().since(jiff::SignedDuration::from_hours(-2));
        tx.conn().execute("UPDATE channel_threads SET last_activity_at = ? WHERE id = ?", (stale, threads[2].id))?;
        tx.conn().execute("UPDATE channel_threads SET closed_at = ? WHERE id = ?", (tx.now(), threads[3].id))?;
        tx.conn().execute("UPDATE channel_threads SET closed_at = ?, locked_at = ? WHERE id = ?", (tx.now(), tx.now(), threads[4].id))?;
        tx.conn().execute("UPDATE channel_threads SET work_owner_id = ? WHERE id = ?", (DAVID, threads[5].id))?;
        let ids = (0..45).map(|index| Message::create(tx, NewMessage { room_id: ALL_TALK, creator_id: JASON,
            thread_id: Some(threads[0].id), markdown_source: Some(format!("Page reply {index}")),
            client_message_id: Some(format!("pages-{index}")), ..Default::default() }).map(|message| message.id)).collect::<campfire_db::Result<Vec<_>>>()?;
        assert_eq!(serde_json::json!(ids), oracle()["message_ids"]);
        tx.conn().execute("UPDATE channel_threads SET last_activity_at = ?, closed_at = NULL WHERE id = ?", (stale, threads[2].id))?;
        ThreadMembership::join(tx, threads[0].id, JASON)?;
        tx.conn().execute("INSERT INTO work_thread_events (id, channel_thread_id, actor_id, event_type, from_status, to_status, to_owner_id, to_owner_name, metadata, created_at, updated_at) VALUES (?, ?, ?, 'work_assignment', 'planned', 'planned', ?, 'David', ?, ?, ?)",
            (oracle()["work_event_id"].as_i64().unwrap(), threads[5].id, DAVID, DAVID, r#"{"note":"Assigned <&>"}"#, tx.now(), tx.now()))?;
        Ok((parent.id, threads.into_iter().map(|thread| thread.id).collect::<Vec<_>>()))
    }).await.unwrap();
    assert_eq!(parent, oracle()["parent_id"].as_i64().unwrap());
    assert_eq!(serde_json::json!(threads), oracle()["thread_ids"]);
    (app, parent, threads)
}

#[tokio::test]
async fn stale_listing_reads_do_not_persist_closure() {
    let (app, _, threads) = fixture().await;
    let mut browser = app.david();
    for state in ["active", "closed", "all"] {
        assert_eq!(browser.get(&format!("/rooms/{ALL_TALK}/threads.json?state={state}")).await.status, StatusCode::OK);
    }
    assert!(app.db().read(move |conn| ChannelThread::find(conn, threads[2])).await.unwrap().closed_at.is_none());
}
