//! The nine previously deferred poll/quote controller behaviors with integrated WS11 invocation.
use crate::controllers::presenters::test_support::*;
use axum::http::{Method, StatusCode};
use campfire_db::Message;
use serde_json::{Value, json};

fn oracle() -> Value {
    serde_json::from_str(include_str!("../../../../../vectors/messaging/quote_integration.json")).unwrap()
}
async fn app() -> TestApp {
    app_rows(oracle()["rows"].clone()).await
}
pub(super) async fn app_rows(rows: Value) -> TestApp {
    app_rows_with_job_runner(rows, false).await
}
async fn app_rows_with_job_runner(rows: Value, run_jobs: bool) -> TestApp {
    let app = TestApp::boot_with_test_clock(std::sync::Arc::new(campfire_kit::clock::FrozenClock::new(SEED_NOW.parse().unwrap())))
        .await.expect("WS8bm2 requires default seed");
    let app = if run_jobs { app } else { app.without_job_runner().await };
    insert_rows(&app, rows).await;
    app
}
pub(super) async fn insert_rows(app: &TestApp, rows: Value) {
    app.db().write(move |tx| {
        for table in ["users", "rooms", "memberships", "webhooks", "calendar_meeting_caches", "events", "event_attendances", "event_calendar_entries", "twitter_posts", "channel_threads", "github_pull_requests", "fizzy_cards", "messages", "action_text_rich_texts", "message_references", "polls", "poll_options", "poll_votes", "message_pins", "github_pull_request_references", "fizzy_card_references", "github_pull_request_threads", "link_embeds", "link_embed_references", "event_references", "twitter_post_references", "saved_items", "scheduled_messages", "activity_items"] {
            for row in rows[table].as_array().into_iter().flatten() {
                let row = row.as_object().unwrap();
                let columns = row.keys().map(|k| format!("\"{k}\"")).collect::<Vec<_>>().join(",");
                let placeholders = vec!["?";row.len()].join(",");
                let values = row.values().map(|v| match v {
                    Value::Null => rusqlite::types::Value::Null,
                    Value::Number(n) => rusqlite::types::Value::Integer(n.as_i64().unwrap()),
                    Value::String(s) => rusqlite::types::Value::Text(s.clone()),
                    _ => panic!("SQL value {v}"),
                });
                tx.conn().execute(&format!("INSERT INTO {table} ({columns}) VALUES ({placeholders})"), rusqlite::params_from_iter(values))?;
            }
        }
        Ok(())
    }).await.unwrap();
}
fn id(key: &str, i: usize) -> i64 { oracle()[key][i].as_i64().unwrap() }

fn write(method:Method,path:String,body:Value) -> Req {
    Req::new(method,&path).header("content-type","application/json").body(serde_json::to_vec(&body).unwrap())
}


#[tokio::test]
async fn deleting_source_clears_cards_and_touches_quoting_message() {
    let app=app().await; let source=id("sources",1); let quote=id("quotes",1);
    app.db().write(move |tx| { tx.conn().execute("UPDATE messages SET updated_at='2026-03-01 16:00:00' WHERE id=?",[quote])?;Ok(()) }).await.unwrap();

    let response=app.david().write(Req::new(Method::DELETE,&format!("/rooms/{ALL_TALK}/messages/{source}")).header("accept","text/vnd.turbo-stream.html")).await;
    assert_eq!(response.status,StatusCode::NO_CONTENT,"{}",response.text());
    app.db().read(move |conn| {
        assert_eq!(conn.query_row("SELECT count(*) FROM message_references WHERE message_id=?",[quote],|r|r.get::<_,i64>(0))?,0);
        assert!(Message::find(conn,quote)?.updated_at.jiff()>"2026-03-01T16:00:00Z".parse().unwrap());Ok(())
    }).await.unwrap();
}
#[tokio::test]
async fn editing_plain_message_to_add_permalink_replaces_its_own_container() {
    let app=app().await; let message=oracle()["empty"].as_i64().unwrap(); let source=id("sources",1);

    let response=app.david().write(write(Method::PATCH,format!("/rooms/{QUIET_CORNER}/messages/{message}"),json!({"message":{"markdown_source":format!("now quoting /rooms/{ALL_TALK}/@{source}")}}))).await;
    assert_eq!(response.status,StatusCode::FOUND,"{}",response.text());
    app.db().read(move |conn| { assert_eq!(conn.query_row("SELECT referenced_message_id FROM message_references WHERE message_id=?",[message],|r|r.get::<_,i64>(0))?,source);Ok(()) }).await.unwrap();

}
