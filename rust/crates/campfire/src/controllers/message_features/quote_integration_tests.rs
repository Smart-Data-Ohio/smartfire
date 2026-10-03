//! The nine previously deferred poll/quote controller behaviors with integrated WS11 invocation.
use crate::controllers::presenters::{Presenter, page, test_support::*};
use axum::http::{Method, StatusCode};
use campfire_db::{Message, NewMessage, NewPoll, Poll};
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
        for table in ["users", "rooms", "memberships", "webhooks", "calendar_meeting_caches", "events", "event_attendances", "event_calendar_entries", "twitter_posts", "channel_threads", "github_pull_requests", "fizzy_cards", "messages", "action_text_rich_texts", "message_references", "polls", "poll_options", "poll_votes", "message_pins", "github_pull_request_references", "fizzy_card_references", "github_pull_request_threads", "link_embeds", "link_embed_references", "event_references", "twitter_post_references"] {
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

async fn poll_card(anonymous: bool) {
    let app = app().await;
    app.db().write(move |tx| {
        let message = Message::create(tx, NewMessage { room_id: ALL_TALK, creator_id:DAVID,
            markdown_source:Some("Room poll".into()),client_message_id:Some("integration-poll".into()),..Default::default() })?;
        let mut poll=Poll::create_for_message(tx,&message,NewPoll { labels:vec!["A".into(),"B".into()],anonymous,..Default::default() })?;
        poll.cast_vote(tx,DAVID,&[poll.options(tx.conn())?[0].id])?;
        Ok(())
    }).await.unwrap();
    let response=app.david().get(&format!("/rooms/{ALL_TALK}")).await;
    assert_eq!(response.status,StatusCode::OK,"{}",response.text());
    let body=response.text();
    assert!(body.contains("data-voter-ids="));
    let attribute=format!("data-voter-ids=\"{DAVID}\"");
    assert_eq!(body.contains(&attribute), !anonymous, "{body}");
}
#[tokio::test]
async fn anonymous_room_cards_carry_no_voter_ids() { poll_card(true).await; }
#[tokio::test]
async fn regular_room_cards_carry_voter_ids_for_client_marking() { poll_card(false).await; }

#[tokio::test]
async fn same_room_quote_renders_inline_and_matches_rails_container() {
    let app=app().await;
    let response=app.david().get(&format!("/rooms/{QUIET_CORNER}")).await;
    assert_eq!(response.status,StatusCode::OK);
    assert!(response.text().contains(oracle()["containers"][0]["html"].as_str().unwrap()),"{}",response.text());
}
#[tokio::test]
async fn cross_room_quote_renders_lazy_without_source_facts() {
    let app=app().await;
    let response=app.sign_in(KEVIN).await.get(&format!("/rooms/{QUIET_CORNER}")).await;
    assert_eq!(response.status,StatusCode::OK);
    assert!(response.text().contains(oracle()["containers"][1]["html"].as_str().unwrap()));
    assert!(!response.text().contains("quote integration 1"));
}
#[tokio::test]
async fn two_cached_direct_room_viewers_see_the_same_neutral_quote_label() {
    let app=app().await;
    for user in [DAVID,JASON] {
        let response=app.sign_in(user).await.get(&format!("/rooms/{DIRECT_DAVID_JASON}/messages")).await;
        assert_eq!(response.status,StatusCode::OK);
        assert!(response.text().contains(oracle()["containers"][2]["html"].as_str().unwrap()),"{}",response.text());
        assert!(response.text().contains("in a direct message"));
    }
}
#[tokio::test]
async fn preloaded_quote_cards_render_without_queries_for_distinct_direct_rooms() {
    use std::sync::{Arc,atomic::{AtomicUsize,Ordering}};
    let app=app().await;
    let messages=app.db().write(|tx| {
        let mut messages=vec![Message::find(tx.conn(),id("quotes",2))?];
        for (i, peer) in [KEVIN,BENDER].into_iter().enumerate() {
            let room=campfire_db::Room::create_for(tx,campfire_db::RoomType::Direct,None,DAVID,&[DAVID,peer])?;
            let source=Message::create(tx,NewMessage { room_id:room.id,creator_id:DAVID,markdown_source:Some(format!("distinct DM source {i}")),client_message_id:Some(format!("distinct-source-{i}")),..Default::default() })?;
            messages.push(Message::create(tx,NewMessage { room_id:room.id,creator_id:DAVID,markdown_source:Some(format!("see /rooms/{}/@{}",room.id,source.id)),client_message_id:Some(format!("distinct-quote-{i}")),..Default::default() })?);
        }
        Ok(messages)
    }).await.unwrap();
    let state=app.booted.app.clone();
    app.db().read(move |conn| {
        // Distinct room binds cannot hide behind SQLite's statement/query cache.
        for rows in [&messages[..1], &messages[..]] {
            let p=Presenter::new(conn,&state,None).preload_search(rows)?;
            conn.flush_prepared_statement_cache();
            let count=Arc::new(AtomicUsize::new(0)); let observed=count.clone();
            conn.authorizer(Some(move |ctx:rusqlite::hooks::AuthContext<'_>| {
                if matches!(ctx.action,rusqlite::hooks::AuthAction::Select) { observed.fetch_add(1,Ordering::SeqCst); }
                rusqlite::hooks::Authorization::Allow
            }));
            for row in rows {
                let view=p.message(row)?;
                let html=page::render_detached_at(&state,None,"http://campfire.test",|ctx|campfire_views::message_links::cards(ctx,&view).0);
                assert!(html.contains("message-link-cards"));
                assert!(html.contains("in a direct message"));
            }
            conn.authorizer(None::<fn(rusqlite::hooks::AuthContext<'_>)->rusqlite::hooks::Authorization>);
            assert_eq!(count.load(Ordering::SeqCst),0);
        }
        Ok(())
    }).await.unwrap();
}

pub(super) async fn stream(app:&TestApp) -> (crate::channels::tests::support::Client,tokio::task::JoinHandle<()>) {
    use crate::channels::tests::support::{Client,bind_listener,identifier};
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    let listener=bind_listener().await; let address=listener.local_addr().unwrap();
    let router=app.booted.router.clone();
    let server=tokio::spawn(async move { axum::serve(listener,router).await.unwrap() });
    let mut ws=format!("ws://{address}/cable").into_client_request().unwrap();
    for (key,value) in [("host","campfire.test"),("origin","http://campfire.test"),("cookie",david_cookie().as_str())] {
        ws.headers_mut().insert(key,value.parse().unwrap());
    }
    let (socket,_)=tokio_tungstenite::connect_async(ws).await.unwrap();
    let mut client=Client { socket }; assert_eq!(client.next_text().await,r#"{"type":"welcome"}"#);
    let room=app.db().read(|conn|campfire_db::Room::find(conn,QUIET_CORNER)).await.unwrap();
    let name=rails_compat::turbo::signed_stream_name(&app.booted.app.secrets,&[&crate::channels::room_gid(&room).to_param(),"messages"]);
    client.confirm(&identifier(json!({"channel":"RoomMessagesChannel","signed_stream_name":name}))).await;
    (client,server)
}
fn write(method:Method,path:String,body:Value) -> Req {
    Req::new(method,&path).header("content-type","application/json").body(serde_json::to_vec(&body).unwrap())
}
async fn quote_frame(client:&mut crate::channels::tests::support::Client,target:&str) -> String {
    for _ in 0..8 {
        let frame:Value=serde_json::from_str(&client.next_text().await).unwrap();
        if let Some(html)=frame["message"].as_str() && html.contains(&format!("target=\"{target}\"")) { return html.into(); }
    }
    panic!("no quote card frame for {target}");
}
#[tokio::test]
async fn editing_source_runs_registered_refresh_job_and_replaces_cards_on_real_stream() {
    let app=app_rows_with_job_runner(oracle()["rows"].clone(), true).await; let (mut client,server)=stream(&app).await;
    let response=app.david().write(write(Method::PATCH,format!("/rooms/{ALL_TALK}/messages/{}",id("sources",1)),json!({"message":{"markdown_source":"revised source"}}))).await;
    assert_eq!(response.status,StatusCode::FOUND,"{}",response.text());
    let html=quote_frame(&mut client,"message_link_cards_message_integration-quote-1").await;
    assert!(html.contains("action=\"replace\"") && html.contains("maintain_scroll=\"true\""));
    assert!(html.contains(oracle()["containers"][1]["html"].as_str().unwrap()));
    server.abort();
}
#[tokio::test]
async fn deleting_source_clears_cards_and_touches_quoting_message() {
    let app=app().await; let source=id("sources",1); let quote=id("quotes",1);
    app.db().write(move |tx| { tx.conn().execute("UPDATE messages SET updated_at='2026-03-01 16:00:00' WHERE id=?",[quote])?;Ok(()) }).await.unwrap();
    let (mut client,server)=stream(&app).await;
    let response=app.david().write(Req::new(Method::DELETE,&format!("/rooms/{ALL_TALK}/messages/{source}")).header("accept","text/vnd.turbo-stream.html")).await;
    assert_eq!(response.status,StatusCode::OK,"{}",response.text());
    let html=quote_frame(&mut client,"message_link_cards_message_integration-quote-1").await;
    assert!(html.contains("class=\"message-link-cards\"></div>"));
    app.db().read(move |conn| {
        assert_eq!(conn.query_row("SELECT count(*) FROM message_references WHERE message_id=?",[quote],|r|r.get::<_,i64>(0))?,0);
        assert!(Message::find(conn,quote)?.updated_at.jiff()>"2026-03-01T16:00:00Z".parse().unwrap());Ok(())
    }).await.unwrap(); server.abort();
}
#[tokio::test]
async fn editing_plain_message_to_add_permalink_replaces_its_own_container() {
    let app=app().await; let message=oracle()["empty"].as_i64().unwrap(); let source=id("sources",1);
    let (mut client,server)=stream(&app).await;
    let response=app.david().write(write(Method::PATCH,format!("/rooms/{QUIET_CORNER}/messages/{message}"),json!({"message":{"markdown_source":format!("now quoting /rooms/{ALL_TALK}/@{source}")}}))).await;
    assert_eq!(response.status,StatusCode::FOUND,"{}",response.text());
    assert!(quote_frame(&mut client,"message_link_cards_message_integration-empty").await.contains("message-link-frame"));
    app.db().read(move |conn| { assert_eq!(conn.query_row("SELECT referenced_message_id FROM message_references WHERE message_id=?",[message],|r|r.get::<_,i64>(0))?,source);Ok(()) }).await.unwrap();
    server.abort();
}
