//! The real HTTP fact factory and owner seams, compared as entire rendered pages.
//! Deterministic token/nonce inputs precede rendering on both sides; no output masks.
use crate::controllers::presenters::{page, room_native, test_support::*, view_context};
use campfire_views::helpers::request_forgery::{AuthenticityTokens, RequestSecrets, rendering_with};
struct Tokens;
impl AuthenticityTokens for Tokens {
    fn global(&self) -> String { "GLOBAL".into() }
    fn for_form(&self, action: &str, method: &str) -> String { format!("{method}:{action}") }
}

#[tokio::test]
async fn empty_room_shell_and_all_owner_children_match_rails_through_http() {
    let env = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../parity/.env.reference")).unwrap();
    let vapid = env.lines().filter_map(|line| line.split_once('=')).filter(|(key,_)| matches!(*key, "VAPID_PUBLIC_KEY" | "VAPID_PRIVATE_KEY")).collect::<Vec<_>>();
    let oracle: serde_json::Value = serde_json::from_str(include_str!("empty_shell_http.json")).unwrap();
    for row in oracle["rows"].as_array().unwrap() {
        let app = TestApp::boot_frozen_with_env(&vapid).await.expect("default seed required");
        let room = row["room_id"].as_i64().unwrap();
        app.db().write(move |tx| {
            use rusqlite::OptionalExtension;
            let stamp = campfire_db::Room::find(tx.conn(), room)?.updated_at;
            while let Some(id) = tx.conn().query_row("SELECT id FROM messages WHERE room_id=? ORDER BY id LIMIT 1", [room], |r| r.get::<_, i64>(0)).optional()? {
                campfire_db::Message::find(tx.conn(), id)?.destroy(tx)?;
            }
            // Identical fixture contract in empty_shell_http.rb: preserve the seed's
            // refresh timestamp while testing the empty room's rendering integration.
            tx.conn().execute("UPDATE rooms SET updated_at=? WHERE id=?", (stamp, room))?;
            Ok(())
        }).await.unwrap();
        let mut browser = app.sign_in(row["user_id"].as_i64().unwrap()).await;
        let response = with_fixed_render_secrets(browser.send(Req::new(axum::http::Method::GET, row["path"].as_str().unwrap()).header("user-agent", "Mozilla"))).await;
        assert_eq!(response.status.as_u16(), row["status"].as_u64().unwrap() as u16);
        let actual = response.text();
        let expected = row["body"].as_str().unwrap();
        if !crate::app::asset_goldens::compare(row["name"].as_str().unwrap(), &actual, expected) {
            rails_mismatch(&actual, expected, row["name"].as_str().unwrap());
        }
    }
}
#[tokio::test]
async fn full_native_room_pages_match_four_complete_rails_pages() {
    let env = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../parity/.env.reference")).unwrap();
    let vapid = env.lines().filter_map(|line| line.split_once('=')).filter(|(key,_)| matches!(*key, "VAPID_PUBLIC_KEY" | "VAPID_PRIVATE_KEY")).collect::<Vec<_>>();
    let app = TestApp::boot_frozen_with_env(&vapid).await.expect("default seed required");
    let oracle: serde_json::Value = serde_json::from_str(include_str!("full_pages.json")).unwrap();
    let mut failures = Vec::new();
    for row in oracle["rows"].as_array().unwrap() {
        let user_id = row["user_id"].as_i64().unwrap();
        let room_id = row["room_id"].as_i64().unwrap();
        let state = app.booted.app.clone();
        let actual = app.db().read(move |conn| {
            let viewer = campfire_db::User::find(conn, user_id)?;
            let room = campfire_db::Room::find(conn, room_id)?;
            let native = room_native::load(conn, &state, &room, &viewer, None, Some("campfire.test".into()), "http://campfire.test".into())?;
            let account = campfire_db::Account::first(conn)?;
            let preferences = view_context::user_preferences(conn, user_id, state.db.env().now())?;
            let last = campfire_db::Room::original_for_user(conn, user_id)?.map(|r| r.id);
            let recent = campfire_db::Search::recent_for_user(conn, user_id)?;
            let icons = crate::controllers::presenters::client_icon_names(conn)?;
            let html = page::render_detached_at(&state, account.as_ref(), "http://campfire.test", |base| {
                let mut ctx = base.clone();
                ctx.current_user = Some(campfire_views::CurrentUser { preferences, ..view_context::current_user(&state.secrets, &viewer) });
                ctx.time_zone = campfire_views::time::Zone::for_user(ctx.current_user.as_ref().unwrap().preferences.time_zone.as_deref());
                ctx.custom_styles = account.as_ref().and_then(|a| a.custom_styles.clone());
                ctx.last_room_visited_id = last;
                ctx.chrome.service_worker_auto_register = true;
                ctx.chrome.brand_icon_names = icons;
                ctx.chrome.recent_searches = recent.into_iter().map(|s| campfire_views::layouts::RecentSearch {id:s.id,query:s.query}).collect();
                ctx.chrome.google_picker = state.config.google_picker.clone();
                ctx.platform.browser = "Mozilla".into();
                ctx.request_url = format!("http://campfire.test/rooms/{room_id}");
                rendering_with(RequestSecrets {tokens:Box::new(Tokens),csp_nonce:Some("NONCE".into())}, || room_native::render(&ctx, &native.show, &native.composer, false))
            }).map_err(|e| campfire_db::Error::Other(e.to_string()))?;
            Ok(html)
        }).await.unwrap();
        let label = format!("room-{room_id}-user-{user_id}");
        if !crate::app::asset_goldens::compare(&label, &actual, row["html"].as_str().unwrap()) {
            let parent = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/ws8br-full-pages");
            std::fs::create_dir_all(&parent).unwrap();
            std::fs::write(parent.join(format!("{label}.actual")), &actual).unwrap();
            std::fs::write(parent.join(format!("{label}.expected")), row["html"].as_str().unwrap()).unwrap();
            failures.push(label);
        }
    }
    assert!(failures.is_empty(), "full native pages differ: {failures:?}");
}
