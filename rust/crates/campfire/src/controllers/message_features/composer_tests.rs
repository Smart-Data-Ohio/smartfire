use crate::controllers::presenters::test_support::*;
use axum::http::StatusCode;
#[tokio::test]
async fn room_http_mounts_markdown_slash_and_schedule_controls() {
    let app = super::quote_integration_tests::app_rows(serde_json::json!({})).await;
    let response = app.david().get(&format!("/rooms/{QUIET_CORNER}")).await;
    assert_eq!(response.status, StatusCode::OK);
    let html = response.text();
    assert!(html.contains("name=\"message[markdown_source]\""));
    assert!(html.contains("data-controller=\"markdown-autocomplete\""));
    assert!(html.contains("data-controller=\"schedule-send\""));
    assert!(html.contains("data-composer-slash-commands-value="));
    assert!(html.contains("name=\"authenticity_token\""));
    assert!(!html.contains("<lexxy-editor"));
    assert!(html.contains("data-controller=\"drive-picker\""));
}

#[tokio::test]
async fn complete_markdown_composers_match_actual_rails_partials_and_sti_identifiers() {
    use askama::Template;
    use campfire_views::{
        helpers::raw,
        messages::{
            RoomKind,
            composer::{Composer, DriveFlow, Facts, Thread},
        },
        scheduled_messages::ComposerButton,
    };
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/composer.json"
    ))
    .unwrap();
    let app = super::quote_integration_tests::app_rows(serde_json::json!({})).await;
    let sti: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/composer_sti.json"
    ))
    .unwrap();
    for case in oracle["cases"]
        .as_array()
        .unwrap()
        .iter()
        .chain(sti["cases"].as_array().unwrap())
    {
        let facts = Facts {
            room_id: case["room_id"].as_i64().unwrap(),
            room_kind: if case["room_kind"] == "rooms_direct" {
                RoomKind::Direct
            } else {
                RoomKind::Closed
            },
            room_param_key: Some(case["room_kind"].as_str().unwrap().into()),
            room_name: case["room_name"].as_str().unwrap().into(),
            thread: case["thread"].as_object().map(|_| Thread {
                id: case["thread"]["id"].as_i64().unwrap(),
                name: case["thread"]["name"].as_str().unwrap().into(),
            }),
            slash_commands: serde_json::from_value(case["commands"].clone()).unwrap(),
            drive: if case["picker"] == true {
                DriveFlow::Share
            } else {
                DriveFlow::Metadata
            },
        };
        let html = crate::controllers::presenters::page::render_detached_at(
            &app.booted.app,
            None,
            "http://campfire.test",
            |ctx| {
                let control = raw(ComposerButton {
                    ctx,
                    room_id: facts.room_id,
                    thread_id: facts.thread.as_ref().map(|t| t.id),
                }
                .render()
                .unwrap());
                Composer {
                    ctx,
                    facts: &facts,
                    scheduled_control: &control,
                }
                .render()
                .unwrap()
            },
        );
        assert_eq!(html, case["html"].as_str().unwrap(), "{case}");
    }
}
#[tokio::test]
async fn room_composer_registry_reads_agent_metadata_and_is_outside_shared_fragments() {
    let app = super::quote_integration_tests::app_rows(serde_json::json!({})).await;
    app.db().write(|tx| {tx.conn().execute("INSERT INTO agent_slash_commands (room_id,agent_id,name,description,takes_arguments,created_at,updated_at) SELECT ?,id,'deploy','Deploy',1,?2,?2 FROM agents ORDER BY id LIMIT 1",rusqlite::params![QUIET_CORNER,tx.now()])?;Ok(())}).await.unwrap();
    let response = app.david().get(&format!("/rooms/{QUIET_CORNER}")).await;
    assert_eq!(response.status, StatusCode::OK);
    assert!(response.text().contains("&quot;deploy&quot;"));
    let start = response.text().find("<form id=\"composer\"").unwrap();
    let end = response.text()[start..].find("</form>").unwrap() + start;
    let form = &response.text()[start..end];
    assert!(form.contains("name=\"authenticity_token\""));
    assert!(form.contains("action=\"/rooms/699448326/messages\""));
}

#[tokio::test]
async fn room_http_preserves_each_sti_composer_reply_identifier() {
    let app = super::quote_integration_tests::app_rows(serde_json::json!({})).await;
    let mut browser = app.david();
    for (kind, param) in [
        ("Rooms::Open", "rooms_open"),
        ("Rooms::Closed", "rooms_closed"),
        ("Rooms::Voice", "rooms_voice"),
        ("Rooms::Stage", "rooms_stage"),
        ("Rooms::Board", "rooms_board"),
    ] {
        app.db()
            .write(move |tx| {
                tx.conn()
                    .execute("UPDATE rooms SET type=? WHERE id=?", (kind, QUIET_CORNER))?;
                Ok(())
            })
            .await
            .unwrap();
        let response = browser.get(&format!("/rooms/{QUIET_CORNER}")).await;
        assert_eq!(
            response.status,
            StatusCode::OK,
            "{kind}: {}",
            response.text()
        );
        assert!(
            response
                .text()
                .contains(&format!("id=\"reply_notify_{param}_{QUIET_CORNER}\"")),
            "{kind}"
        );
        assert!(
            response
                .text()
                .contains(&format!("for=\"reply_notify_{param}_{QUIET_CORNER}\"")),
            "{kind}"
        );
    }
    println!(
        "WS8bm2 STI composer HTTP: 5/5 room types preserve Rails reply-control ids and labels"
    );
}
