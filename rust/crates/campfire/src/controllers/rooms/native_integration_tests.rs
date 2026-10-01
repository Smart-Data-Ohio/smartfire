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

async fn cards_in_viewer_zones(zones: &[Option<&str>], thread: bool) {
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
