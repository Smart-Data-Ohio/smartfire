//! Whole owner partial bytes, plus real request token and viewer/neutral-name integration.
use crate::controllers::presenters::{page, test_support::*};
use askama::Template;
use campfire_views::helpers::request_forgery::{AuthenticityTokens, RequestSecrets, rendering_with};
use serde_json::Value;
struct Tokens;
impl AuthenticityTokens for Tokens {
    fn global(&self) -> String { "GLOBAL".into() }
    fn for_form(&self, action: &str, method: &str) -> String { format!("{method}:{action}") }
}
#[tokio::test]
async fn native_owner_panels_match_fourteen_complete_rails_partials() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    let panels: Value = serde_json::from_str(include_str!("owner_panels.json")).unwrap();
    let mut compared = 0;
    for row in panels["cases"].as_array().unwrap().iter().filter(|c| matches!(c["partial"].as_str(), Some("thread_panel" | "poll_builder"))) {
        let input = &row["input"];
        // These panels have no voice/stage behavior: use the shell's existing fallback
        // type while preserving original route IDs, neutral labels and every golden byte.
        let room = campfire_views::rooms::RoomView {
            id: input["room"]["id"].as_i64().unwrap(),
            kind: if input["room"]["kind"] == "direct" { campfire_views::messages::RoomKind::Direct } else { campfire_views::messages::RoomKind::Closed },
            name: input["room"]["name"].as_str().map(str::to_owned),
            display_name: input["room"]["display_name"].as_str().unwrap().into(),
            involvement: "everything".into(), header: None,
        };
        let actual = page::render_detached_at(&app.booted.app, None, "http://campfire.test", |ctx| rendering_with(
            RequestSecrets {tokens:Box::new(Tokens),csp_nonce:Some("NONCE".into())}, || {
                if row["partial"] == "thread_panel" { campfire_views::rooms::panels::ThreadPanel {ctx,room:&room,neutral_name:input["neutral_name"].as_str()}.render().unwrap() }
                else { campfire_views::rooms::panels::PollBuilder {ctx,room:&room}.render().unwrap() }
            }));
        assert!(crate::app::asset_goldens::compare(row["name"].as_str().unwrap(), &actual, row["html"].as_str().unwrap()));
        compared += 1;
    }
    let pins: Value = serde_json::from_str(include_str!("owner_pins.json")).unwrap();
    for row in pins["panels"].as_array().unwrap() {
        let room_id = row["room_id"].as_i64().unwrap();
        let key = if room_id == DIRECT_DAVID_JASON { "rooms_direct" } else { "rooms_closed" };
        for (field,count) in [("empty",0),("populated",2)] {
            let actual = page::render_detached_at(&app.booted.app,None,"http://campfire.test",|ctx| campfire_views::pins::PanelPartial {ctx,room_id,room_param_key:key,count}.render().unwrap());
            assert!(crate::app::asset_goldens::compare(field,&actual,row[field].as_str().unwrap()));
            compared += 1;
        }
    }
    assert_eq!(compared,14);
}
#[tokio::test]
async fn native_owner_panels_use_request_secrets_and_neutral_direct_names() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    let mut browser = app.sign_in(DAVID).await;
    let reply = browser.get(&format!("/rooms/{DIRECT_DAVID_JASON}")).await;
    assert_eq!(reply.status,axum::http::StatusCode::OK);
    let html=reply.text();
    let content=campfire_richtext::Content::wrap(&html).unwrap();
    let dom=&content.dom;
    let panel=dom.descendants(content.root).into_iter().find(|&n|dom.attr(n,"id")==Some("thread-panel")).unwrap();
    assert_eq!(dom.attr(panel,"data-thread-panel-channel-name"),Some("David, Jason"));
    assert_eq!(html.matches("id=\"poll-builder\"").count(),1);
    assert_eq!(html.matches("class=\"pins-panel-wrap\"").count(),1);
    assert!(html.contains(&format!("id=\"pins_frame_rooms_direct_{DIRECT_DAVID_JASON}\"")));
    let token=browser.real_authenticity_token().unwrap();
    let form=dom.descendants(content.root).into_iter().find(|&n|dom.name(n)=="form" && dom.attr(n,"action")==Some(format!("/rooms/{DIRECT_DAVID_JASON}/polls").as_str())).unwrap();
    let hidden=dom.descendants(form).into_iter().find(|&n|dom.attr(n,"name")==Some("authenticity_token")).unwrap();
    let value=dom.attr(hidden,"value").unwrap();
    assert!(token.is_valid(value,&format!("/rooms/{DIRECT_DAVID_JASON}/polls"),"POST"));
    let mut other=app.sign_in(JASON).await;
    other.get(&format!("/rooms/{DIRECT_DAVID_JASON}")).await;
    assert!(!other.real_authenticity_token().unwrap().is_valid(value,&format!("/rooms/{DIRECT_DAVID_JASON}/polls"),"POST"));
}

#[tokio::test]
async fn native_picker_configuration_reaches_layout_and_owner_composer() {
    let oracle:Value=serde_json::from_str(include_str!("picker_config.json")).unwrap();
    for row in oracle["picker_cases"].as_array().unwrap() {
        let pairs=row["input"].as_object().unwrap().iter().map(|(k,v)|(k.as_str(),v.as_str().unwrap())).collect::<Vec<_>>();
        let app=TestApp::boot_frozen_with_env(&pairs).await.expect("seed required");
        let html=app.david().get(&format!("/rooms/{ALL_TALK}")).await.text();
        let user=DAVID;
        let state=app.booted.app.clone();
        let footer=app.db().read(move|conn| {
            let viewer=campfire_db::User::find(conn,user)?;
            let room=campfire_db::Room::find(conn,ALL_TALK)?;
            let presenter=crate::controllers::presenters::Presenter::new(conn,&state,None);
            let drive=presenter.composer_drive_flow(&viewer,state.config.google_picker.is_some())?;
            let facts=presenter.composer_facts(&room,&viewer,None,drive)?;
            let viewer=crate::controllers::presenters::user_view(&state.secrets,&viewer);
            let account=campfire_db::Account::first(conn)?;
            let result=page::render_detached_at(&state,account.as_ref(),"http://campfire.test",|ctx| rendering_with(RequestSecrets{tokens:Box::new(Tokens),csp_nonce:None},||super::super::presenters::room_native::components(ctx,&viewer,&facts)));
            result.map(|(footer,_)|footer).map_err(|e|campfire_db::Error::Other(e.to_string()))
        }).await.unwrap();
        assert!(crate::app::asset_goldens::compare("configured native composer",&footer,row["composer"].as_str().unwrap()));
        let expected=row["configured"].as_bool().unwrap();
        assert_eq!(html.contains("name=\"google-drive-share\" content=\"enabled\""),expected,"Picker configuration {:?}",row["input"]);
        assert_eq!(html.contains("data-controller=\"drive-share\""),expected,"composer Picker {:?}",row["input"]);
        if expected {assert!(html.contains("content=\"public-client&lt;&amp;&gt;\""));}
        assert_eq!(html.contains("data-controller=\"drive-picker\""),!expected,"seed's metadata consent yields to the public Picker");
    }
}
