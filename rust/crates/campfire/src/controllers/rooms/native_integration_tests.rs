//! Native request integration, without borrowed message/composer HTML.
use crate::controllers::presenters::test_support::*;
use axum::http::{StatusCode};
use campfire_db::{Message, Membership};
#[tokio::test]
async fn native_room_page_mounts_the_selected_owner_list_and_composer() {
 let app=TestApp::boot_frozen().await.unwrap();let mut browser=app.sign_in(DAVID).await;
 let reply=browser.get(&format!("/rooms/{ALL_TALK}")).await;assert_eq!(reply.status,StatusCode::OK);let html=reply.text();
 let records=app.db().read(|conn|crate::controllers::presenters::room_shell::find_messages(conn,ALL_TALK,None)).await.unwrap();
 assert_eq!(records.len(),40);
 for message in records {assert!(html.contains(&format!("id=\"message_{}\"",message.client_message_id)),"selected root {} missing",message.id);}
 assert!(html.contains("name=\"message[markdown_source]\""));
 assert!(html.contains("data-controller=\"schedule-send\""));
 assert!(html.contains("data-user-id=\"127326141\""));
 assert_eq!(html.matches("data-messages-target=\"template\"").count(),1);
 let token=browser.real_authenticity_token().unwrap();
 for item in super::tests::session_bound(&html,&format!("/rooms/{ALL_TALK}")) {
  if let super::tests::SessionBound::Token{value,path,method}=item {assert!(token.is_valid(&value,&path,&method));}
 }
}
#[tokio::test]
async fn native_room_anchor_mounts_owner_messages_around_the_root() {
 let app=TestApp::boot_frozen().await.unwrap();
 let first=app.db().read(|conn|Ok(Message::for_room(conn,ALL_TALK)?.into_iter().min_by_key(|m|m.created_at).unwrap())).await.unwrap();
 let reply=app.david().get(&format!("/rooms/{ALL_TALK}/@{}",first.id)).await;assert_eq!(reply.status,StatusCode::OK);
 let records=app.db().read(move|conn|crate::controllers::presenters::room_shell::find_messages(conn,ALL_TALK,Some(first.id))).await.unwrap();
 assert_eq!(records.len(),41);for message in records {assert!(reply.text().contains(&format!("id=\"message_{}\"",message.client_message_id)));}
}
#[tokio::test]
async fn native_room_lists_are_per_viewer_even_when_fragments_are_warm() {
 let app=TestApp::boot_frozen().await.unwrap();
 let boundary=app.db().read(|conn|Ok(crate::controllers::presenters::room_shell::find_messages(conn,ALL_TALK,None)?[35].clone())).await.unwrap();
 let boundary_id=boundary.id;
 app.db().write(move|tx| {
  Membership::find_by_room_and_user(tx.conn(),ALL_TALK,DAVID)?.unwrap().mark_unread_before(tx,&boundary)?;
  Membership::find_by_room_and_user(tx.conn(),ALL_TALK,JASON)?.unwrap().read(tx)?;
  Ok(())
 }).await.unwrap();
 for _ in 0..2 {
  let david=app.david().get(&format!("/rooms/{ALL_TALK}")).await.text();
  let jason=app.sign_in(JASON).await.get(&format!("/rooms/{ALL_TALK}")).await.text();
  assert!(david.contains("class=\"unread-divider\""),"David divider {boundary_id} missing");
  assert!(!jason.contains("class=\"unread-divider\""));
 }
}

#[tokio::test]
async fn native_component_capture_matches_rails_root_selection() {
 let app=TestApp::boot_frozen().await.unwrap();
 let mut captures=vec![];
 let fixtures:serde_json::Value=serde_json::from_str(include_str!("../../../../views/tests/golden/rooms/native_components.json")).unwrap();
 for row in fixtures["rows"].as_array().unwrap() {
  let user_id=row["user_id"].as_i64().unwrap();let room_id=row["room_id"].as_i64().unwrap();
  let state=app.booted.app.clone();
  let (ids,list,composer,template)=app.db().read(move|conn| {
   let room=campfire_db::Room::find(conn,room_id)?;let user=campfire_db::User::find(conn,user_id)?;
   let records=crate::controllers::presenters::room_shell::find_messages(conn,room_id,None)?;
   let mut presenter=crate::controllers::presenters::Presenter::new(conn,&state,Some("campfire.test".into()));presenter.cache_base_url=Some("http://campfire.test".into());
   let list=crate::controllers::presenters::room_native::message_list(&presenter,&records,None,0)?;
   let flow=presenter.composer_drive_flow(&user,false)?;
   let facts=presenter.composer_facts(&room,&user,None,flow)?;
   let viewer=crate::controllers::presenters::user_view(&state.secrets,&user);
   let account=campfire_db::Account::first(conn)?;
   use campfire_views::helpers::request_forgery::{rendering_with,RequestSecrets};
   let (composer,template)=rendering_with(RequestSecrets{tokens:Box::new(ComponentTokens),csp_nonce:None},||crate::controllers::presenters::page::render_detached_at(&state,account.as_ref(),"http://campfire.test",|ctx|crate::controllers::presenters::room_native::components(ctx,&viewer,&facts))).map_err(|e|campfire_db::Error::Other(e.to_string()))?;
   Ok((records.iter().map(|m|m.id).collect::<Vec<_>>(),list,composer,template))
  }).await.unwrap();
  assert_eq!(serde_json::json!(ids),row["root_ids"]);
  for (name,actual) in [("composer",&composer),("pending_template",&template)] {
   assert!(crate::app::asset_goldens::compare(name,actual,row[name].as_str().unwrap()),"room {room_id} {name}");
  }
  {
   assert!(crate::app::asset_goldens::compare("message list",&list,row["message_list"].as_str().unwrap()),"room {room_id} list");
  }
  captures.push(serde_json::json!({"room_id":room_id,"user_id":user_id,"message_list":list,"composer":composer,"pending_template":template}));
 }
 println!("WS8BR_NATIVE_COMPONENTS:{}",serde_json::json!(captures));
}

struct ComponentTokens;
impl campfire_views::helpers::request_forgery::AuthenticityTokens for ComponentTokens {
 fn global(&self)->String {"GLOBAL".into()}
 fn for_form(&self,action:&str,method:&str)->String {format!("{method}:{action}")}
}

// Preserve the complete source bytes of a nested card container. This is a byte
// comparison, not a DOM serialization or a replacement in the production page.
fn card_container<'a>(html: &'a str, selector: &str) -> &'a str {
    let start = html.find(&format!("<div id=\"{selector}\"")).expect("card target missing");
    let mut depth = 0;
    for tag in regex::Regex::new(r"</?div\b[^>]*>").unwrap().find_iter(&html[start..]) {
        depth += if tag.as_str().starts_with("</") { -1 } else { 1 };
        if depth == 0 { return &html[start..start + tag.end()]; }
    }
    panic!("unclosed card target: {selector}");
}

#[tokio::test]
async fn native_room_page_provider_cards_match_rails_bytes() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    let fixtures: serde_json::Value = serde_json::from_str(include_str!("../../../../views/tests/golden/rooms/native_components.json")).unwrap();
    let row = &fixtures["rows"][0];
    let room_id = row["room_id"].as_i64().unwrap();
    assert_eq!(row["user_id"], DAVID);
    let expected = row["message_list"].as_str().unwrap();
    let reply = app.david().get(&format!("/rooms/{room_id}")).await;
    assert_eq!(reply.status, StatusCode::OK);
    let actual = reply.text();
    let targets = regex::Regex::new(r#"<div id="((?:fizzy_cards|link_embed_cards|linkedin_cards)_message_[^"]+)""#).unwrap();
    let mut populated = 0;
    for target in targets.captures_iter(expected) {
        let selector = &target[1];
        let expected_card = card_container(expected, selector);
        assert!(crate::app::asset_goldens::compare(selector, card_container(&actual, selector), expected_card), "{selector}");
        if !expected_card.ends_with("></div>") { populated += 1; }
    }
    assert_eq!(populated, 3, "the seed must exercise all three merged provider bodies");
}

async fn zone_fixture_app() -> (TestApp, serde_json::Value) {
    let app = TestApp::boot_frozen().await.expect("default seed required");
    let oracle: serde_json::Value = serde_json::from_str(include_str!("event_zones.json")).unwrap();
    let rows = oracle["rows"].clone();
    let github = oracle["github"].clone();
    // Persist the Rails fixture records, not rendered provider HTML. Every HTTP path
    // resolves cards through the real Rust presenter and owner partials.
    app.db().write(move |tx| {
        for table in ["events", "messages", "action_text_rich_texts", "event_references"] {
            for row in rows[table].as_array().unwrap() {
                let row = row.as_object().unwrap();
                let columns = row.keys().map(|name| format!("\"{name}\"")).collect::<Vec<_>>().join(",");
                let parameters = vec!["?"; row.len()].join(",");
                let values = row.values().map(|value| match value {
                    serde_json::Value::Null => rusqlite::types::Value::Null,
                    serde_json::Value::Bool(value) => rusqlite::types::Value::Integer(i64::from(*value)),
                    serde_json::Value::Number(value) => rusqlite::types::Value::Integer(value.as_i64().unwrap()),
                    serde_json::Value::String(value) => rusqlite::types::Value::Text(value.clone()),
                    _ => panic!("unexpected SQL fixture value {value}"),
                });
                tx.conn().execute(&format!("INSERT INTO {table} ({columns}) VALUES ({parameters})"), rusqlite::params_from_iter(values))?;
            }
        }
        tx.conn().execute("UPDATE github_pull_requests SET github_updated_at=? WHERE id=?",
            (github["updated_at"].as_str().unwrap(), github["id"].as_i64().unwrap()))?;
        Ok(())
    }).await.unwrap();
    (app, oracle)
}

async fn cards_in_viewer_zones(zones: &[Option<&str>], thread: bool) {
    let (app, oracle) = zone_fixture_app().await;
    let mut browser = app.david();
    for zone in zones {
        let stored = zone.map(str::to_owned);
        app.db().write(move |tx| {
            tx.conn().execute("UPDATE users SET time_zone=? WHERE id=?", (stored, DAVID))?;
            Ok(())
        }).await.unwrap();
        for case in oracle["cases"].as_array().unwrap().iter().filter(|case| case["zone"].as_str() == *zone && case["path"].as_str().unwrap().contains("/threads/") == thread) {
            let path = case["path"].as_str().unwrap();
            let response = browser.send(Req::new(axum::http::Method::GET, path).header("accept",
                if path.contains("refresh") { "text/vnd.turbo-stream.html" } else { "text/html" })).await;
            assert_eq!(response.status.as_u16(), case["status"].as_u64().unwrap() as u16, "{zone:?}: {path}");
            let body = response.text();
            for card in case["cards"].as_array().unwrap() {
                let target = card["target"].as_str().unwrap();
                let actual = card_container(&body, target);
                let expected = card["html"].as_str().unwrap();
                if actual != expected { rails_mismatch(actual, expected, &format!("viewer zone {zone:?}: {path}: {target}")); }
                assert_eq!(campfire_cable::turbo::session_bound(actual), None);
            }
        }
    }
}

#[tokio::test]
async fn native_room_event_cards_match_rails_hawaii_and_warm_viewer_changes() {
    cards_in_viewer_zones(&[Some("Hawaii"), Some("UTC"), Some("Hawaii")], false).await;
}

#[tokio::test]
async fn native_room_event_cards_match_rails_both_eastern_dst_transitions() {
    cards_in_viewer_zones(&[Some("Eastern Time (US & Canada)"), Some("Hawaii"), Some("Eastern Time (US & Canada)")], false).await;
}

#[tokio::test]
async fn native_room_event_cards_match_rails_utc_and_invalid_zone_fallbacks() {
    cards_in_viewer_zones(&[Some("UTC"), None, Some(""), Some("Not a real zone"), Some("UTC")], false).await;
}

#[tokio::test]
async fn native_thread_header_matches_rails_viewer_zones() {
    cards_in_viewer_zones(&[Some("Hawaii"), Some("Eastern Time (US & Canada)"), Some("UTC"), Some("Hawaii")], true).await;
}

fn write_zone_oracle() -> serde_json::Value {
    serde_json::from_str(include_str!("card_write_zones.json")).unwrap()
}

fn zone_cases<'a>(oracle: &'a serde_json::Value, kind: &str, defaults: bool) -> Vec<&'a serde_json::Value> {
    oracle["cases"].as_array().unwrap().iter().filter(|case| {
        case["kind"] == kind && matches!(case["zone"].as_str(), None | Some("UTC" | "" | "Not a real zone")) == defaults
    }).collect()
}

async fn set_audit_zone(app: &TestApp, case: &serde_json::Value) {
    let zone = case["zone"].as_str().map(str::to_owned);
    app.db().write(move |tx| {
        tx.conn().execute("UPDATE users SET time_zone=? WHERE id=?", (zone, DAVID))?;
        Ok(())
    }).await.unwrap();
}

fn assert_audit_card(body: &str, case: &serde_json::Value, expected: &str) {
    let actual = card_container(body, case["target"].as_str().unwrap());
    if actual != expected { rails_mismatch(actual, expected, &format!("{} {:?}: {}", case["kind"], case["zone"], case["path"])); }
    assert_eq!(campfire_cable::turbo::session_bound(actual), None);
}

async fn audit_message_creation(defaults: bool) {
    let (app, _) = zone_fixture_app().await;
    let oracle = write_zone_oracle();
    let (mut clients, server) = audit_subscribers(&app).await;
    let mut browser = app.david();
    for case in zone_cases(&oracle, "message_create", defaults) {
        set_audit_zone(&app, case).await;
        let sequence = case["message_id"].as_i64().unwrap() - 1;
        app.db().write(move |tx| {
            tx.conn().execute("UPDATE sqlite_sequence SET seq=? WHERE name='messages'", [sequence])?;
            Ok(())
        }).await.unwrap();
        let response = browser.write(Req::new(axum::http::Method::POST, case["path"].as_str().unwrap())
            .header("accept", "text/vnd.turbo-stream.html").form(&[
                ("message[client_message_id]", case["client_id"].as_str().unwrap()),
                ("message[markdown_source]", case["source"].as_str().unwrap()),
            ])).await;
        assert_eq!(response.status.as_u16(), case["status"].as_u64().unwrap() as u16);
        assert_audit_card(&response.text(), case, case["html"].as_str().unwrap());
        for client in &mut clients {
            let frame: serde_json::Value = serde_json::from_str(&client.next_text().await).unwrap();
            assert_audit_card(frame["message"].as_str().unwrap(), case, case["broadcast_html"].as_str().unwrap());
        }
        let reload = browser.get(&format!("/rooms/{ALL_TALK}")).await;
        assert_eq!(reload.status, StatusCode::OK);
        assert_audit_card(&reload.text(), case, case["html"].as_str().unwrap());
        // Retries render in the current request zone without repeating the creation.
        let retry = browser.write(Req::new(axum::http::Method::POST, case["path"].as_str().unwrap())
            .header("accept", "text/vnd.turbo-stream.html").form(&[("message[client_message_id]", case["client_id"].as_str().unwrap())])).await;
        assert_eq!(retry.status, StatusCode::OK);
        assert_audit_card(&retry.text(), case, case["html"].as_str().unwrap());
    }
    for client in &mut clients { client.assert_silent().await; }
    drop(clients);
    server.abort();
}

async fn audit_github_messages(defaults: bool) {
    let (app, _) = zone_fixture_app().await;
    let oracle = write_zone_oracle();
    let mut browser = app.david();
    let cases = zone_cases(&oracle, "github_message", defaults);
    // Revisit the first viewer after other zones warm the same message caches.
    for case in cases.iter().copied().chain(cases.iter().take(4).copied()) {
        set_audit_zone(&app, case).await;
        let path = case["path"].as_str().unwrap();
        let response = browser.send(Req::new(axum::http::Method::GET, path).header("accept",
            if path.contains("refresh") { "text/vnd.turbo-stream.html" } else { "text/html" })).await;
        assert_eq!(response.status.as_u16(), case["status"].as_u64().unwrap() as u16, "{path}");
        assert_audit_card(&response.text(), case, case["html"].as_str().unwrap());
    }
}

async fn audit_socket(app: &TestApp, url: &str, origin: &str, user: i64) -> crate::channels::tests::support::Client {
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    let browser = app.sign_in(user).await;
    let mut request = url.into_client_request().unwrap();
    request.headers_mut().insert("cookie", browser.cookie_header().parse().unwrap());
    request.headers_mut().insert("origin", origin.parse().unwrap());
    request.headers_mut().insert("sec-websocket-protocol", "actioncable-v1-json".parse().unwrap());
    let (socket, _) = tokio_tungstenite::connect_async(request).await.unwrap();
    let mut client = crate::channels::tests::support::Client { socket };
    assert_eq!(client.next_text().await, r#"{"type":"welcome"}"#);
    client
}

async fn audit_subscribers(app: &TestApp) -> ([crate::channels::tests::support::Client; 2], tokio::task::JoinHandle<()>) {
    app.db().write(|tx| {
        tx.conn().execute("UPDATE users SET time_zone='Asia/Kolkata' WHERE id=?", [JASON])?;
        Ok(())
    }).await.unwrap();
    let listener = crate::test_support::bind_listener().await;
    let address = listener.local_addr().unwrap();
    let router = app.booted.router.clone();
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let room = app.db().read(|conn| campfire_db::Room::find(conn, ALL_TALK)).await.unwrap();
    let stream = crate::channels::room_gid(&room).to_param();
    let signed = rails_compat::turbo::signed_stream_name(&app.booted.app.secrets, &[&stream, "messages"]);
    let channel = crate::channels::tests::support::identifier(serde_json::json!({"channel":"RoomMessagesChannel","signed_stream_name":signed}));
    let mut clients = [audit_socket(app, &format!("ws://{address}/cable"), &format!("http://{address}"), DAVID).await,
        audit_socket(app, &format!("ws://{address}/cable"), &format!("http://{address}"), JASON).await];
    for client in &mut clients { client.confirm(&channel).await; }
    (clients, server)
}

async fn assert_audit_frame(clients: &mut [crate::channels::tests::support::Client], frame: &serde_json::Value) {
    let expected = frame["payload"].as_str().unwrap();
    for client in clients {
        let message: serde_json::Value = serde_json::from_str(&client.next_text().await).unwrap();
        let actual = message["message"].as_str().unwrap();
        if actual != expected { rails_mismatch(actual, expected, "shared actor-zone event broadcast"); }
        assert_eq!(campfire_cable::turbo::session_bound(actual), None);
    }
}

async fn audit_event_broadcasts(defaults: bool) {
    let (app, _) = zone_fixture_app().await;
    let oracle = write_zone_oracle();
    let (mut clients, server) = audit_subscribers(&app).await;
    let room = app.db().read(|conn| campfire_db::Room::find(conn, ALL_TALK)).await.unwrap();
    let stream = crate::channels::room_gid(&room).to_param();
    let mut browser = app.david();
    let mut other = app.sign_in(JASON).await;
    for case in zone_cases(&oracle, "event_edit", defaults) {
        set_audit_zone(&app, case).await;
        assert_eq!(case["frame"]["stream"], format!("{stream}:messages"));
        let response = browser.write(Req::new(axum::http::Method::PATCH, case["path"].as_str().unwrap())
            .form(&[("event[title]", case["title"].as_str().unwrap()), ("event[starts_at]", "2026-03-08T06:30"),
                ("event[ends_at]", "2026-03-08T07:30"), ("event[time_zone]", "UTC")])).await;
        assert_eq!(response.status.as_u16(), case["status"].as_u64().unwrap() as u16);
        assert_audit_frame(&mut clients, &case["frame"]).await;
        let reload = other.get(&format!("/rooms/{ALL_TALK}")).await;
        assert_eq!(reload.status, StatusCode::OK);
        let body = reload.text();
        let actual = card_container(&body, "event_cards_message_ws8br-zone-spring");
        let expected = case["reload_other_html"].as_str().unwrap();
        if actual != expected { rails_mismatch(actual, expected, "actor edit then India viewer reload"); }
        // Validation failure must restore the writer scope too; later jobs have no actor.
        let invalid = browser.write(Req::new(axum::http::Method::PATCH, case["path"].as_str().unwrap())
            .form(&[("event[title]", "")])).await;
        assert_eq!(invalid.status, StatusCode::UNPROCESSABLE_ENTITY);
        app.db().write(|tx| campfire_db::CalendarEvent::update(tx, 8000000601,
            campfire_db::models::calendar_event::changes::EventChanges { title: Some("Background UTC".into()), ..Default::default() })).await.unwrap();
        assert_audit_frame(&mut clients, &oracle["background"]).await;
    }
    for client in &mut clients { client.assert_silent().await; }
    drop(clients);
    server.abort();
}

#[tokio::test]
async fn zone_audit_message_creation_matches_rails_and_reload() {
    audit_message_creation(false).await;
}

#[tokio::test]
async fn zone_audit_github_message_cards_match_rails_with_warm_zones() {
    audit_github_messages(false).await;
}

#[tokio::test]
async fn zone_audit_event_broadcast_matches_rails_actor_zone_for_every_recipient() {
    audit_event_broadcasts(false).await;
}

#[tokio::test]
async fn zone_audit_utc_and_invalid_zone_fallbacks_cover_all_three_paths() {
    audit_message_creation(true).await;
    audit_github_messages(true).await;
    audit_event_broadcasts(true).await;
}

fn cache_zone_oracle() -> serde_json::Value {
    serde_json::from_str(include_str!("cache_zones.json")).unwrap()
}

async fn assert_rails_collection_keys(app: &TestApp, case: &serde_json::Value) {
    let expected = case["keys"].clone();
    let room_id = case["room_id"].as_i64().unwrap();
    let state = app.booted.app.clone();
    app.db().read(move |conn| {
        let mut presenter = crate::controllers::presenters::Presenter::new(conn, &state, Some("campfire.test".into()));
        presenter.use_viewer_zone(DAVID)?;
        let records = crate::controllers::presenters::room_shell::find_messages(conn, room_id, None)?;
        let mut actual = serde_json::Map::new();
        for message in records {
            actual.insert(message.id.to_string(), serde_json::Value::String(presenter.message_collection_cache_key(&message)?));
        }
        assert_eq!(actual.len(), expected.as_object().unwrap().len());
        for (id, key) in actual {
            assert_eq!(key, expected[&id], "Rails collection timestamp components for message {id}");
        }
        Ok(())
    }).await.unwrap();
}

async fn audit_cache_sharing(kind: &str) {
    let app = TestApp::boot_frozen().await.expect("default seed required");
    let oracle = cache_zone_oracle();
    let pair = oracle["pairs"].as_array().unwrap().iter().find(|pair| pair["kind"] == kind).unwrap();
    let first = &pair["cases"][0];
    let second = &pair["cases"][1];
    assert_eq!(first["keys"], second["keys"]);
    assert!(first["written_fragments"].as_u64().unwrap() > 0);
    assert_eq!(first["written_fragments"], second["written_fragments"]);
    let mut browser = app.david();
    set_audit_zone(&app, first).await;
    let path = format!("/rooms/{}", first["room_id"]);
    assert_eq!(browser.get(&path).await.status, StatusCode::OK);
    let cold = app.booted.app.fragment_cache.len();
    assert!(cold > 0, "exercise actual collection fragments");
    set_audit_zone(&app, second).await;
    assert_eq!(browser.get(&path).await.status, StatusCode::OK);
    let warm = app.booted.app.fragment_cache.len();
    assert_eq!(warm, cold, "{kind}: Rails reuses identical keys; cold {cold}, warm {warm}");
    assert_rails_collection_keys(&app, second).await;
    set_audit_zone(&app, first).await;
    assert_eq!(browser.get(&path).await.status, StatusCode::OK);
    assert_eq!(app.booted.app.fragment_cache.len(), cold);
    assert_rails_collection_keys(&app, first).await;
}

#[tokio::test]
async fn zone_cache_hawaii_and_honolulu_share_github_fragments_like_rails() {
    audit_cache_sharing("github_alias").await;
}

#[tokio::test]
async fn zone_cache_hawaii_and_india_share_ordinary_fragments_like_rails() {
    audit_cache_sharing("ordinary").await;
}

#[tokio::test]
async fn zone_cache_timestamp_components_match_rails_across_dst_and_fractional_zones() {
    let app = TestApp::boot_frozen().await.expect("default seed required");
    let oracle = cache_zone_oracle();
    for case in oracle["seasonal"].as_array().unwrap() {
        let instant = case["instant"].as_str().unwrap().parse::<jiff::Timestamp>().unwrap();
        app.db().write(move |tx| {
            tx.conn().execute("UPDATE github_pull_requests SET updated_at=? WHERE id=1", [campfire_db::Timestamp::from_jiff(instant)])?;
            Ok(())
        }).await.unwrap();
        set_audit_zone(&app, case).await;
        assert_rails_collection_keys(&app, case).await;
    }
}
