use std::sync::Arc;
use axum::http::{Method, StatusCode};
use campfire_db::{ChannelThread, Message, NewChannelThread, NewMessage, ThreadMembership};
use campfire_kit::clock::FrozenClock;
use serde_json::Value;
use crate::controllers::presenters::test_support::*;

fn oracle() -> Value { serde_json::from_str(include_str!("../../../../../vectors/messaging/thread-pages.json")).unwrap() }

#[tokio::test]
async fn seeded_pull_request_thread_header_matches_rails_bytes_through_http() {
    let app = TestApp::boot_frozen().await.expect("default seed required");
    let oracle: Value = serde_json::from_str(include_str!("../../../../../vectors/github_seed_fragments.json")).unwrap();
    let row = &oracle["thread"];
    let room = row["room_id"].as_i64().unwrap();
    let thread = row["thread_id"].as_i64().unwrap();
    let response = app.david().get(&format!("/rooms/{room}/threads/{thread}")).await;
    assert_eq!(response.status, StatusCode::OK);
    let expected = row["header"].as_str().unwrap();
    assert!(response.text().contains(expected), "real HTTP thread page omits or changes Rails PR header/card/files/actions bytes");
}

#[tokio::test]
async fn pull_request_thread_pages_match_four_rails_http_responses() {
    let oracle: Value = serde_json::from_str(include_str!("../../../../../vectors/messaging/pr-thread-http.json")).unwrap();
    for row in oracle["rows"].as_array().unwrap() {
        let app = TestApp::boot_frozen().await.expect("default seed required");
        let private = row["private"].as_bool();
        let mapped = row["mapped"].as_bool().unwrap();
        let pr = oracle["pull_request_id"].as_i64().unwrap();
        let thread = oracle["thread_id"].as_i64().unwrap();
        app.db().write(move |tx| {
            tx.conn().execute("UPDATE github_pull_requests SET private=? WHERE id=?", (private, pr))?;
            if !mapped { tx.conn().execute("DELETE FROM github_pull_request_threads WHERE channel_thread_id=?", [thread])?; }
            Ok(())
        }).await.unwrap();
        let response = with_fixed_render_secrets(app.david().get(row["path"].as_str().unwrap())).await;
        assert_eq!(response.status.as_u16(), row["status"].as_u64().unwrap() as u16);
        let text = response.text();
        let start = text.find("<main class=\"thread\"").unwrap();
        let end = text[start..].find("</main>").unwrap() + start + "</main>".len();
        let actual = &text[start..end];
        let expected = row["body"].as_str().unwrap();
        if actual != expected { rails_mismatch(actual, expected, row["name"].as_str().unwrap()); }
    }
}

#[tokio::test]
async fn pull_request_thread_without_starter_refreshes_once_and_survives_queue_failure() {
    let app = TestApp::boot_frozen().await.expect("default seed required").without_job_runner().await;
    app.db().write(|tx| {
        tx.conn().execute_batch("DELETE FROM background_jobs; UPDATE channel_threads SET parent_message_id=NULL WHERE id=8; UPDATE github_pull_requests SET fetched_at=NULL,fetch_requested_at=NULL WHERE id IN (SELECT github_pull_request_id FROM github_pull_request_threads WHERE channel_thread_id=8)")?;
        Ok(())
    }).await.unwrap();
    let mut browser = app.david();
    for _ in 0..2 {
        let response = browser.get("/rooms/654632876/threads/8").await;
        assert_eq!(response.status, StatusCode::OK);
        assert!(response.text().contains("id=\"github_pr_header_channel_thread_8\""));
        let jobs = app.db().read(|conn| Ok(conn.query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='Github::FetchPullRequestJob'", [], |r| r.get::<_, i64>(0))?)).await.unwrap();
        assert_eq!(jobs, 1);
    }
    app.db().write(|tx| {
        tx.conn().execute_batch("DELETE FROM background_jobs; UPDATE github_pull_requests SET fetch_requested_at=NULL; CREATE TRIGGER reject_thread_pr_refresh BEFORE INSERT ON background_jobs WHEN NEW.job_class='Github::FetchPullRequestJob' BEGIN SELECT RAISE(ABORT,'queue unavailable'); END;")?;
        Ok(())
    }).await.unwrap();
    let response = browser.get("/rooms/654632876/threads/8").await;
    assert_eq!(response.status, StatusCode::OK);
    assert!(response.text().contains("id=\"github_pr_header_channel_thread_8\""));
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
async fn thread_state_lists_and_standalone_reads_match_rails_bytes() {
    let (app, parent, threads) = fixture().await;
    let mut browser = app.david();
    for row in oracle()["rows"].as_array().unwrap() {
        let name = row["name"].as_str().unwrap();
        if name == "show_deleted_parent" {
            app.db().write(move |tx| Message::find(tx.conn(), parent)?.destroy(tx)).await.unwrap();
        }
        let mut request = Req::new(Method::GET, row["path"].as_str().unwrap());
        if let Some(frame) = row["frame"].as_str() { request = request.header("turbo-frame", frame); }
        let response = browser.send(request).await;
        assert_eq!(response.status.as_u16(), row["status"].as_u64().unwrap() as u16, "{name}: {}", response.text());
        assert_eq!(response.header("cache-control"), row["cache_control"].as_str(), "{name}");
        if response.status.is_success() {
            assert_eq!(response.header("content-type"), row["content_type"].as_str(), "{name}");
            let expected = row["body"].as_str().unwrap();
            if row["html"] == true {
                if !response.text().contains(expected) { rails_mismatch(&response.text(), expected, name); }
                if row["frame"].is_string() { assert!(!response.text().contains("<title>Smartfire</title>")); }
                else {
                    assert!(response.text().contains("<!DOCTYPE html>"));
                    assert!(response.text().contains(row["title"].as_str().unwrap()), "{name}: page title missing");
                }
            } else if response.text() != expected { rails_mismatch(&response.text(), expected, name); }
        }
    }
    assert!(app.db().read(move |conn| ThreadMembership::find_by_thread_and_user(conn, threads[0], DAVID)).await.unwrap().is_none());
}

#[tokio::test]
async fn complete_standalone_thread_templates_match_rails_layout_bytes() {
    use askama::Template;
    use campfire_views::{helpers as h, layouts, ViewContext};
    use crate::controllers::presenters::{page, Presenter, view_context};
    struct Tokens;
    impl h::request_forgery::AuthenticityTokens for Tokens {
        fn global(&self) -> String { "GLOBAL".into() }
        fn for_form(&self, action: &str, method: &str) -> String { format!("{method}:{action}") }
    }
    let (app, parent_id, _) = fixture().await;
    for row in oracle()["rows"].as_array().unwrap().iter().filter(|row| row["html"] == true && row["name"].as_str().unwrap().starts_with("show_")) {
        let row = row.clone();
        let id = row["path"].as_str().unwrap().rsplit('/').next().unwrap().parse::<i64>().unwrap();
        let path = row["path"].as_str().unwrap().to_string();
        let frame = row["frame"].is_string();
        if row["name"] == "show_deleted_parent_html" {app.db().write(move |tx| Message::find(tx.conn(), parent_id)?.destroy(tx)).await.unwrap();}
        let runtime = app.booted.app.clone();
        let actual = app.db().read(move |conn| {
            let p = Presenter::new(conn, &runtime, None);
            let thread = ChannelThread::find(conn, id)?;
            let records = Message::last_page(conn, campfire_db::Timeline::Thread(id))?;
            let parent = thread.parent_message_id.map(|id| Message::find(conn, id)).transpose()?.as_ref().map(|m| p.message_item(m)).transpose()?;
            let items = p.messages(&records)?;
            let viewer = campfire_db::User::find(conn, DAVID)?;
            let account = campfire_db::Account::first(conn)?;
            let preferences = conn.query_row("SELECT theme,text_size,time_zone,time_zone_explicit,tour_completed_at IS NOT NULL,voice_mode,push_to_talk_key FROM users WHERE id = ?", [DAVID], |r| Ok(layouts::UserPreferences {
                theme: r.get(0)?, text_size: r.get(1)?, time_zone: r.get(2)?, time_zone_explicit: r.get(3)?, tour_completed: r.get(4)?, voice_mode: r.get(5)?, push_to_talk_key: r.get(6)?, ..Default::default()
            }))?;
            let mut current = view_context::current_user(&runtime.secrets, &viewer);
            current.preferences = preferences;
            current.preferences.google_drive = !matches!(p.composer_drive_flow(&viewer, false)?, campfire_views::messages::composer::DriveFlow::None);
            current.preferences.notification_sounds.quiet_hours = conn.query_row("SELECT CASE WHEN quiet_hours_enabled THEN quiet_hours_start_minute END, CASE WHEN quiet_hours_enabled THEN quiet_hours_end_minute END FROM users WHERE id = ?", [DAVID], |r| {
                Ok(r.get::<_, Option<i64>>(0)?.zip(r.get::<_, Option<i64>>(1)?))
            })?;
            let searches = campfire_db::Search::ordered_for_user(conn, DAVID)?;
            let header = super::render_thread_pull_request_header(&p, &thread)?;
            Ok(page::render_detached_at(&runtime, account.as_ref(), "http://campfire.test", |ctx| {
                // Explicit owner chrome inputs, as in WS6/WS9's full-page oracle contract.
                let ctx = ViewContext { current_user: Some(current), account: ctx.account.clone(), flash_notice: None, flash_alert: None,
                    platform: campfire_views::Platform {mac: true, chrome: true, desktop: true, browser: "Chrome".into(), operating_system: "macOS".into(), ..Default::default()},
                    vapid_public_key: include_str!("../../../../../parity/.env.reference").lines().find_map(|s| s.strip_prefix("VAPID_PUBLIC_KEY=")).map(str::to_string),
                    asset_path: ctx.asset_path, importmap_tags: ctx.importmap_tags, stylesheet_tags: ctx.stylesheet_tags, custom_styles: ctx.custom_styles.clone(), cable_url: ctx.cable_url.clone(),
                    base_url: ctx.base_url.clone(), request_url: format!("http://campfire.test{path}"), referrer: None,
                    last_room_visited_id: Some(ALL_TALK), app_version: "parity".into(), signed_stream_name: ctx.signed_stream_name, time_zone: ctx.time_zone.clone(),
                    chrome: layouts::Chrome {service_worker_auto_register: true, brand_icon_names: oracle()["brand_icon_names"].as_array().unwrap().iter().map(|s| s.as_str().unwrap().into()).collect(),
                        recent_searches: searches.iter().take(10).map(|s| layouts::RecentSearch {id: s.id, query: s.query.clone()}).collect(), ..Default::default()}
                };
                h::request_forgery::rendering_with(h::request_forgery::RequestSecrets {tokens: Box::new(Tokens), csp_nonce: Some("NONCE".into())}, || {
                    let body = h::raw(campfire_views::channel_threads::Show {ctx: &ctx, name: &thread.name, status: thread.status(conn, campfire_db::Timestamp::from_jiff(p.now)).unwrap().name(),
                        count: thread.message_count(conn).unwrap(), pull_request_header: &header, parent: parent.as_ref(), messages: &items}.render().unwrap());
                    if frame {layouts::FrameLayout {ctx: &ctx, head: h::empty(), content: body}.render().unwrap()}
                    else {let mut page = layouts::Application::new(&ctx, body);page.page_title = Some(thread.name.clone());page.render().unwrap()}
                })
            }))
        }).await.unwrap();
        let expected = row["full_body"].as_str().unwrap();
        if !crate::app::asset_goldens::compare(row["name"].as_str().unwrap(),&actual,expected) {rails_mismatch(&actual, expected, row["name"].as_str().unwrap());}
    }
}

#[tokio::test]
async fn ordinary_work_html_remains_pending_and_board_posts_render() {
    let (app, _, threads) = fixture().await;
    assert_eq!(app.sign_in(KEVIN).await.get(&format!("/rooms/{ALL_TALK}/threads/{}", threads[5])).await.status, StatusCode::NOT_FOUND);
    assert_eq!(app.david().get(&format!("/rooms/{ALL_TALK}/threads/{}", threads[5])).await.status, StatusCode::NOT_IMPLEMENTED);
    assert_eq!(app.sign_in(KEVIN).await.get(&format!("/rooms/{ALL_TALK}/threads/{}/content", threads[5])).await.status, StatusCode::NOT_FOUND);
    assert_eq!(app.david().get(&format!("/rooms/{ALL_TALK}/threads/{}/content", threads[5])).await.status, StatusCode::NOT_IMPLEMENTED);
    let (room, thread) = app.db().write(|tx| {
        let room = campfire_db::Room::create_for(tx, campfire_db::RoomType::Board, Some("Board seam"), DAVID, &[DAVID])?;
        let thread = ChannelThread::create(tx, NewChannelThread {room_id: room.id, creator_id: DAVID, name: Some("Board post".into()), work_status: Some("planned".into()), ..Default::default()})?;
        Ok((room.id, thread.id))
    }).await.unwrap();
    let response = app.david().get(&format!("/rooms/{room}/threads/{thread}")).await;
    assert_eq!(response.status, StatusCode::OK);
    assert!(response.text().contains("class=\"board-post\""));
    assert_eq!(app.david().get(&format!("/rooms/{room}/threads/{thread}/content")).await.status, StatusCode::NOT_IMPLEMENTED);
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
