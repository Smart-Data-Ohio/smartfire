//! Drive search, share grants, and removing a Drive file on edit (`/api/v1/drive`,
//! `/api/v1/rooms/:id/drive`).

use std::sync::Arc;

use axum::http::{Method, StatusCode};
use campfire_api_types as api;
use serde_json::{json, Value};

use super::api_tests::{get, json_body, parse, tag};
use crate::app::google_api_tests::{self, Recorded};
use crate::controllers::presenters::test_support::{
    seed_clock, TestApp, ALL_TALK, BENDER, DAVID, JASON, KEVIN,
};

const FILE: &str = "1AbcDefGhIjKlMnOpQrSt";

async fn app() -> Option<(TestApp, Arc<Recorded>)> {
    let clock = Arc::new(campfire_kit::FrozenClock::new(seed_clock().now()));
    let app = TestApp::boot_seed_with_env("default", clock, &[("SPA_ENABLED", "1")]).await?;
    let recorded = Recorded::new(vec![]);
    google_api_tests::install(&app, recorded.clone()).await;
    app.booted.app.google.drive().install_picker(true);
    app.db()
        .write(|tx| {
            tx.conn().execute("DELETE FROM google_accounts", [])?;
            Ok(())
        })
        .await
        .unwrap();
    Some((app, recorded))
}

async fn grant(app: &TestApp, user_id: i64) {
    google_api_tests::grant(
        app,
        user_id,
        campfire_db::Timestamp::from_jiff(app.booted.app.clock.now())
            .since(jiff::SignedDuration::from_hours(1)),
        true,
    )
    .await;
}

fn drive_card(message: &api::MessageDTO) -> Option<&api::DriveFileCard> {
    message.cards.iter().find_map(|card| match card {
        api::MessageCard::Drive(file) => Some(file),
        _ => None,
    })
}

#[tokio::test]
async fn drive_search_is_per_viewer_and_404_when_not_connected() {
    let Some((app, recorded)) = app().await else {
        return;
    };
    let mut david = app.sign_in(DAVID).await;
    let missing = david.send(get("/api/v1/drive/files?q=plan")).await;
    assert_eq!(missing.status, StatusCode::NOT_FOUND);
    assert!(missing.body.is_empty());
    assert!(recorded.calls.lock().unwrap().is_empty());

    grant(&app, DAVID).await;
    recorded.answer(200, google_api_tests::vectors()["drive_list"].clone());
    let listed = david.send(get("/api/v1/drive/files?q=%20plan%20")).await;
    assert_eq!(listed.status, StatusCode::OK, "{}", listed.text());
    let files: api::DriveFileList = parse(&listed);
    assert_eq!(files.files.len(), 2);
    assert_eq!(files.files[0].name.as_deref(), Some("Q3 Planning"));
    assert_eq!(files.files[0].kind, "document");
    assert_eq!(files.files[0].owner.as_deref(), Some("Riel"));
    assert_eq!(files.files[1].kind, "spreadsheet");

    let path = format!("/api/v1/drive/files/{FILE}");
    recorded.answer(200, google_api_tests::vectors()["drive_file"].clone());
    let shown = david.send(get(&path)).await;
    assert_eq!(shown.status, StatusCode::OK, "{}", shown.text());
    let file: api::DriveFile = parse(&shown);
    assert_eq!(file.name.as_deref(), Some("Q3 Planning"));
    assert_eq!(file.kind, "document");
    assert_eq!(david.send(get(&path)).await.status, StatusCode::OK);
    assert_eq!(recorded.calls.lock().unwrap().len(), 2);

    grant(&app, JASON).await;
    let mut jason = app.sign_in(JASON).await;
    recorded.answer(
        200,
        json!({
            "id": FILE,
            "name": "Jason's copy",
            "mimeType": "application/vnd.google-apps.document",
            "modifiedTime": "2026-09-16T10:30:00.000Z",
            "owners": [{"displayName": "Jason"}],
            "webViewLink": "https://docs.google.com/document/d/1AbcDefGhIjKlMnOpQrSt/edit"
        }),
    );
    let jasons: api::DriveFile = parse(&jason.send(get(&path)).await);
    assert_eq!(jasons.name.as_deref(), Some("Jason's copy"));
    assert_eq!(recorded.calls.lock().unwrap().len(), 3);

    recorded.answer(500, json!({}));
    let down = david.send(get("/api/v1/drive/files")).await;
    assert_eq!(down.status, StatusCode::BAD_GATEWAY);
    assert_eq!(down.json(), json!({"error": "drive_unavailable"}));
}

#[tokio::test]
async fn drive_share_grants_reader_access_without_email_and_404_when_not_connected() {
    let Some((app, recorded)) = app().await else {
        return;
    };
    let mut david = app.sign_in(DAVID).await;
    let recipients = david
        .send(get(&format!("/api/v1/rooms/{ALL_TALK}/drive/recipients")))
        .await;
    assert_eq!(recipients.status, StatusCode::OK, "{}", recipients.text());
    assert_eq!(recipients.header("cache-control"), Some("no-store"));
    let list: api::DriveRecipientList = parse(&recipients);
    assert_eq!(
        list.recipients,
        vec![api::DriveRecipient {
            id: JASON,
            name: "Jason".into(),
            email: "jason@37signals.com".into(),
        }]
    );
    assert!(recorded.calls.lock().unwrap().is_empty());

    let validate = format!("/api/v1/rooms/{ALL_TALK}/drive/recipients/validate");
    let outsider = david
        .write(json_body(
            Method::POST,
            &validate,
            &json!({"userIds": [KEVIN.to_string(), DAVID.to_string(), BENDER.to_string()]}),
        ))
        .await;
    assert_eq!(outsider.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(tag(&outsider), "Validation");
    let envelope: api::ApiErrorResponse = parse(&outsider);
    match envelope.error {
        api::ApiError::Validation { message, fields } => {
            assert_eq!(message, "invalid_recipients");
            assert_eq!(
                fields["userIds"],
                vec![KEVIN.to_string(), DAVID.to_string(), BENDER.to_string()]
            );
        }
        other => panic!("expected validation, got {other:?}"),
    }

    let ok = david
        .write(json_body(
            Method::POST,
            &validate,
            &json!({"userIds": [format!(" {JASON} ")]}),
        ))
        .await;
    assert_eq!(ok.status, StatusCode::OK, "{}", ok.text());
    let selected: api::DriveRecipientList = parse(&ok);
    assert_eq!(selected.recipients.len(), 1);
    assert_eq!(selected.recipients[0].id, JASON);

    let share = format!("/api/v1/rooms/{ALL_TALK}/drive/shares");
    let disconnected = david
        .write(json_body(
            Method::POST,
            &share,
            &json!({"fileId": FILE, "userIds": [JASON.to_string()]}),
        ))
        .await;
    assert_eq!(disconnected.status, StatusCode::NOT_FOUND);
    assert!(disconnected.body.is_empty());
    assert!(recorded.calls.lock().unwrap().is_empty());

    grant(&app, DAVID).await;
    recorded.answer(200, json!({"id": "permission-1", "role": "reader"}));
    let granted = david
        .write(json_body(
            Method::POST,
            &share,
            &json!({"fileId": FILE, "userIds": [JASON.to_string()]}),
        ))
        .await;
    assert_eq!(granted.status, StatusCode::OK, "{}", granted.text());
    let shared: api::DriveShare = parse(&granted);
    assert_eq!(shared.file_id, FILE);
    assert_eq!(shared.recipients[0].email, "jason@37signals.com");
    let calls = recorded.calls.lock().unwrap().clone();
    assert_eq!(calls.len(), 1);
    let path = calls[0]["path"].as_str().unwrap();
    assert!(path.contains("/permissions"), "{path}");
    assert!(path.contains("sendNotificationEmail=false"), "{path}");
    let body: Value = serde_json::from_str(calls[0]["body"].as_str().unwrap()).unwrap();
    assert_eq!(body["role"], "reader");
    assert_eq!(body["type"], "user");
    assert_eq!(body["emailAddress"], "jason@37signals.com");
}

#[tokio::test]
async fn editing_a_message_removes_its_drive_file_and_only_the_author_can() {
    let Some((app, _)) = app().await else {
        return;
    };
    let mut david = app.sign_in(DAVID).await;
    let mut jason = app.sign_in(JASON).await;
    let created = david
        .write(json_body(
            Method::POST,
            &format!("/api/v1/rooms/{ALL_TALK}/messages"),
            &json!({
                "clientMessageId": "0192f0c4-7e8a-7b3c-9d0a-6f3b2d1e8c91",
                "markdownSource": "See the file",
                "driveFileIds": [FILE]
            }),
        ))
        .await;
    assert_eq!(created.status, StatusCode::CREATED, "{}", created.text());
    let message: api::MessageDTO = parse(&created);
    let card = drive_card(&message).expect("drive card");
    assert_eq!(card.file_id, FILE);
    assert_eq!(card.url, format!("https://drive.google.com/open?id={FILE}"));
    assert!(message.edited_at.is_none());

    let path = format!("/api/v1/messages/{}", message.id);
    let stolen = jason
        .write(json_body(
            Method::PATCH,
            &path,
            &json!({"markdownSource": "See the file", "removeDriveFileIds": [FILE]}),
        ))
        .await;
    assert_eq!(
        (stolen.status, tag(&stolen)),
        (StatusCode::FORBIDDEN, "Forbidden".into())
    );
    let still: i64 = app
        .db()
        .read(move |conn| {
            Ok(conn.query_row(
                "SELECT COUNT(*) FROM drive_attachments WHERE message_id=?",
                [message.id],
                |row| row.get(0),
            )?)
        })
        .await
        .unwrap();
    assert_eq!(still, 1);

    let edited = david
        .write(json_body(
            Method::PATCH,
            &path,
            &json!({"markdownSource": "See the file", "removeDriveFileIds": [FILE]}),
        ))
        .await;
    assert_eq!(edited.status, StatusCode::OK, "{}", edited.text());
    let updated: api::MessageDTO = parse(&edited);
    assert!(drive_card(&updated).is_none());
    assert!(updated.edited_at.is_none());
    let left: i64 = app
        .db()
        .read(move |conn| {
            Ok(conn.query_row(
                "SELECT COUNT(*) FROM drive_attachments WHERE message_id=?",
                [message.id],
                |row| row.get(0),
            )?)
        })
        .await
        .unwrap();
    assert_eq!(left, 0);
}
