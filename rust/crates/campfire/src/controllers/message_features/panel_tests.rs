//! Structural HTTP mounts, deliberately not a browser harness.
use super::quote_integration_tests::app_rows;
use crate::controllers::presenters::{page, test_support::*};
use askama::Template;
use axum::http::StatusCode;
use serde_json::Value;
fn oracle()->Value { serde_json::from_str(include_str!("../../../../../vectors/messaging/panels.json")).unwrap() }
#[tokio::test]
async fn empty_and_populated_pin_panels_match_rails_and_mount_with_sti_targets() {
    let app=app_rows(serde_json::json!({})).await;
    for row in oracle()["panels"].as_array().unwrap() {
        let room_id=row["room_id"].as_i64().unwrap();
        for count in [0,2] {
            let state=app.booted.app.clone();
            let expected=row[if count==0 {"empty"} else {"populated"}].as_str().unwrap().to_owned();
            let rendered=app.db().read(move |conn| {
                let room=campfire_db::Room::find(conn,room_id)?;
                Ok(page::render_detached_at(&state,None,"http://campfire.test",|ctx|
                    campfire_views::pins::PanelPartial { ctx,room_id,
                        room_param_key:&campfire_db::broadcasts::room_param_key(room.room_type),count }
                        .render().unwrap()))
            }).await.unwrap();
            assert_eq!(rendered,expected);
        }
        let response=app.david().get(&format!("/rooms/{room_id}")).await;
        assert_eq!(response.status,StatusCode::OK);
        assert!(response.text().contains(row["empty"].as_str().unwrap()));
    }
}
#[tokio::test]
async fn sidebar_mounts_real_saved_and_scheduled_destination_links() {
    let app=app_rows(serde_json::json!({})).await;
    let response=app.david().get("/users/me/sidebar").await;
    assert_eq!(response.status,StatusCode::OK,"{}",response.text());
    for row in oracle()["links"].as_array().unwrap() {
        assert!(response.text().contains(row["html"].as_str().unwrap()),"{}",response.text());
        let linked=app.david().get(row["path"].as_str().unwrap()).await;
        assert_eq!(linked.status,StatusCode::OK);
    }
}
#[tokio::test]
async fn inaccessible_rooms_never_mount_pin_panels_or_load_pin_rows() {
    let app=app_rows(serde_json::json!({})).await;
    let response=app.sign_in(KEVIN).await.get(&format!("/rooms/{ALL_TALK}")).await;
    assert_eq!(response.status,StatusCode::FOUND);
    assert!(!response.text().contains("pins-panel"));
    assert_eq!(app.sign_in(KEVIN).await.get(&format!("/rooms/{ALL_TALK}/pins")).await.status,StatusCode::NOT_FOUND);
}
#[tokio::test]
async fn pin_header_targets_preserve_voice_stage_and_board_sti_identity() {
    let app=app_rows(serde_json::json!({})).await;
    for (room,kind) in [(699448330,"rooms_voice"),(699448331,"rooms_stage"),(699448332,"rooms_board")] {
        let response=app.david().get(&format!("/rooms/{room}")).await;
        assert_eq!(response.status,StatusCode::OK,"{}",response.text());
        assert!(response.text().contains(&format!("id=\"pins_count_{kind}_{room}\"")));
        assert!(response.text().contains(&format!("id=\"pins_frame_{kind}_{room}\"")));
    }
}
