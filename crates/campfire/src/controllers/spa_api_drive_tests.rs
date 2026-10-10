//! Drive search, share grants, and removing a Drive file on edit (`/api/v1/drive`,
//! `/api/v1/rooms/:id/drive`).

use std::sync::Arc;

use axum::http::{Method, StatusCode};
use campfire_api_types as api;
use serde_json::{Value, json};

use super::api_tests::{get, json_body, parse, tag};
use crate::app::google_api_tests::{self, Recorded};
use crate::controllers::presenters::test_support::{
    ALL_TALK, BENDER, DAVID, DIRECT_KEVIN_BENDER, JASON, KEVIN, TestApp, seed_clock,
};

const FILE: &str = "1AbcDefGhIjKlMnOpQrSt";
const FILE_TWO: &str = "2BcDefGhIjKlMnOpQrStu";
/// Thread 1 in Designers. David created the room, so he can reply.
const THREAD: i64 = 1;

fn shareable(mime: &str, can_share: bool) -> Value {
    json!({
        "id": FILE,
        "name": "Q3 Planning",
        "mimeType": mime,
        "capabilities": {"canShare": can_share}
    })
}

fn share_body(file_id: &str, recipients: &[(i64, &str)], attached: &[&str]) -> Value {
    json!({
        "fileId": file_id,
        "recipients": recipients
            .iter()
            .map(|(id, email)| json!({"id": id.to_string(), "email": email}))
            .collect::<Vec<_>>(),
        "attachedFileIds": attached,
    })
}

async fn attachment_ids(app: &TestApp, message_id: i64) -> Vec<String> {
    app.db()
        .read(move |conn| {
            campfire_db::models::DriveAttachment::for_message(conn, message_id)
                .map(|rows| rows.into_iter().map(|row| row.file_id).collect())
        })
        .await
        .unwrap()
}

async fn app() -> Option<(TestApp, Arc<Recorded>)> {
    let clock = Arc::new(campfire_kit::FrozenClock::new(seed_clock().now()));
    let app = TestApp::boot_seed_with_env(
        "default",
        clock,
        &[
            ("SPA_ENABLED", "1"),
            ("GOOGLE_CLIENT_ID", "picker-client"),
            ("GOOGLE_PICKER_API_KEY", "picker-key"),
            ("GOOGLE_CLOUD_PROJECT_NUMBER", "123456789"),
        ],
    )
    .await?;
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
async fn drive_picker_config_is_public_only_and_requires_a_connected_viewer() {
    let Some((app, recorded)) = app().await else {
        return;
    };
    let path = "/api/v1/drive/picker";
    let mut guest = app.anonymous();
    let missing = guest.send(get(path)).await;
    assert_eq!(missing.status, StatusCode::NOT_FOUND);
    assert!(missing.body.is_empty());

    let mut david = app.sign_in(DAVID).await;
    let missing = david.send(get(path)).await;
    assert_eq!(missing.status, StatusCode::NOT_FOUND);
    assert!(missing.body.is_empty());
    grant(&app, DAVID).await;
    let response = david.send(get(path)).await;
    assert_eq!(response.status, StatusCode::OK, "{}", response.text());
    assert_eq!(response.header("cache-control"), Some("no-store"));
    assert_eq!(
        response.json(),
        json!({
            "clientId": "picker-client", "apiKey": "picker-key", "projectNumber": "123456789"
        })
    );
    assert!(recorded.calls.lock().unwrap().is_empty());
    let mut jason = app.sign_in(JASON).await;
    assert_eq!(jason.send(get(path)).await.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn drive_picker_without_configuration_is_an_empty_404() {
    let clock = Arc::new(campfire_kit::FrozenClock::new(seed_clock().now()));
    let Some(app) = TestApp::boot_seed_with_env(
        "default",
        clock,
        &[("SPA_ENABLED", "1"), ("GOOGLE_PICKER_API_KEY", "")],
    )
    .await
    else {
        return;
    };
    grant(&app, DAVID).await;
    let mut david = app.sign_in(DAVID).await;
    let missing = david.send(get("/api/v1/drive/picker")).await;
    assert_eq!(missing.status, StatusCode::NOT_FOUND);
    assert!(missing.body.is_empty());
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

    recorded.answer(
        200,
        json!({
            "files": [{
                "id": "shortcutId00",
                "name": "Alias",
                "mimeType": "application/vnd.google-apps.shortcut",
                "modifiedTime": "2026-09-16T10:30:00.000Z",
                "owners": [{"displayName": "Riel"}],
                "webViewLink": "https://drive.google.com/open?id=shortcutId00"
            }]
        }),
    );
    let shortcuts: api::DriveFileList =
        parse(&david.send(get("/api/v1/drive/files?q=alias")).await);
    assert_eq!(shortcuts.files[0].kind, "shortcut");

    let path = format!("/api/v1/drive/files/{FILE}");
    recorded.answer(200, google_api_tests::vectors()["drive_file"].clone());
    let shown = david.send(get(&path)).await;
    assert_eq!(shown.status, StatusCode::OK, "{}", shown.text());
    let file: api::DriveFile = parse(&shown);
    assert_eq!(file.name.as_deref(), Some("Q3 Planning"));
    assert_eq!(file.kind, "document");
    assert_eq!(david.send(get(&path)).await.status, StatusCode::OK);
    assert_eq!(recorded.calls.lock().unwrap().len(), 3);

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
    assert_eq!(recorded.calls.lock().unwrap().len(), 4);

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
            &share_body(FILE, &[(JASON, "jason@37signals.com")], &[]),
        ))
        .await;
    assert_eq!(disconnected.status, StatusCode::NOT_FOUND);
    assert!(disconnected.body.is_empty());
    assert!(recorded.calls.lock().unwrap().is_empty());

    grant(&app, DAVID).await;
    recorded.answer(200, shareable("application/vnd.google-apps.document", true));
    recorded.answer(200, json!({"permissions": []}));
    recorded.answer(200, json!({"id": "permission-1", "role": "reader"}));
    let granted = david
        .write(json_body(
            Method::POST,
            &share,
            &share_body(FILE, &[(JASON, "jason@37signals.com")], &[]),
        ))
        .await;
    assert_eq!(granted.status, StatusCode::OK, "{}", granted.text());
    let shared: api::DriveShare = parse(&granted);
    assert_eq!(shared.outcome, "shared");
    assert_eq!(shared.file_id, FILE);
    assert!(shared.blocked.is_none());
    assert_eq!(shared.results.len(), 1);
    assert_eq!(shared.results[0].status, "granted");
    assert_eq!(shared.results[0].recipient.email, "jason@37signals.com");
    let calls = recorded.calls.lock().unwrap().clone();
    assert_eq!(calls.len(), 3);
    assert!(
        calls[0]["path"].as_str().unwrap().contains("canShare"),
        "{}",
        calls[0]["path"]
    );
    assert_eq!(calls[1]["method"], "GET");
    assert!(calls[1]["path"].as_str().unwrap().contains("/permissions"));
    let path = calls[2]["path"].as_str().unwrap();
    assert_eq!(calls[2]["method"], "POST");
    assert!(path.contains("/permissions"), "{path}");
    assert!(path.contains("sendNotificationEmail=false"), "{path}");
    let body: Value = serde_json::from_str(calls[2]["body"].as_str().unwrap()).unwrap();
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

    let both = david
        .write(json_body(
            Method::POST,
            &format!("/api/v1/rooms/{ALL_TALK}/messages"),
            &json!({
                "clientMessageId": "0192f0c4-7e8a-7b3c-9d0a-6f3b2d1e8c92",
                "markdownSource": "Two files",
                "driveFileIds": [FILE, FILE_TWO]
            }),
        ))
        .await;
    assert_eq!(both.status, StatusCode::CREATED, "{}", both.text());
    let both: api::MessageDTO = parse(&both);
    assert_eq!(attachment_ids(&app, both.id).await, vec![FILE, FILE_TWO]);
    let kept = david
        .write(json_body(
            Method::PATCH,
            &format!("/api/v1/messages/{}", both.id),
            &json!({"markdownSource": "Two files", "removeDriveFileIds": [FILE]}),
        ))
        .await;
    assert_eq!(kept.status, StatusCode::OK, "{}", kept.text());
    assert_eq!(attachment_ids(&app, both.id).await, vec![FILE_TWO]);

    let reply = david
        .write(json_body(
            Method::POST,
            &format!("/api/v1/threads/{THREAD}/messages"),
            &json!({
                "clientMessageId": "0192f0c4-7e8a-7b3c-9d0a-6f3b2d1e8c93",
                "markdownSource": "In the thread",
                "driveFileIds": [FILE]
            }),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let reply: api::MessageDTO = parse(&reply);
    assert_eq!(reply.thread_id, Some(THREAD));
    assert_eq!(attachment_ids(&app, reply.id).await, vec![FILE]);
    let dropped = david
        .write(json_body(
            Method::PATCH,
            &format!("/api/v1/messages/{}", reply.id),
            &json!({"markdownSource": "In the thread", "removeDriveFileIds": [FILE]}),
        ))
        .await;
    assert_eq!(dropped.status, StatusCode::OK, "{}", dropped.text());
    assert!(attachment_ids(&app, reply.id).await.is_empty());
}

#[tokio::test]
async fn anonymous_search_is_an_empty_404_and_lists_are_rate_limited() {
    let Some((app, recorded)) = app().await else {
        return;
    };
    let mut guest = app.anonymous();
    let missing = guest.send(get("/api/v1/drive/files?q=plan")).await;
    assert_eq!(missing.status, StatusCode::NOT_FOUND);
    assert!(missing.body.is_empty());
    assert!(recorded.calls.lock().unwrap().is_empty());

    grant(&app, DAVID).await;
    let mut david = app.sign_in(DAVID).await;
    for _ in 0..30 {
        recorded.answer(200, json!({"files": []}));
        let listed = david.send(get("/api/v1/drive/files?q=plan")).await;
        assert_eq!(listed.status, StatusCode::OK, "{}", listed.text());
    }
    assert_eq!(recorded.calls.lock().unwrap().len(), 30);
    let limited = david.send(get("/api/v1/drive/files?q=plan")).await;
    assert_eq!(limited.status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(limited.json(), json!({"error": "rate_limited"}));
    assert_eq!(recorded.calls.lock().unwrap().len(), 30);
}

#[tokio::test]
async fn validate_rejects_malformed_oversized_and_stale_recipients() {
    let Some((app, _)) = app().await else {
        return;
    };
    let mut david = app.sign_in(DAVID).await;
    let path = format!("/api/v1/rooms/{ALL_TALK}/drive/recipients/validate");
    let malformed = david
        .write(json_body(
            Method::POST,
            &path,
            &json!({"userIds": ["nope", "12x"]}),
        ))
        .await;
    assert_eq!(malformed.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(tag(&malformed), "Validation");
    let envelope: api::ApiErrorResponse = parse(&malformed);
    match envelope.error {
        api::ApiError::Validation { message, fields } => {
            assert_eq!(message, "invalid_recipients");
            assert_eq!(fields["userIds"], vec!["invalid_recipients".to_string()]);
        }
        other => panic!("expected validation, got {other:?}"),
    }

    let ids = (1..=101).map(|id| id.to_string()).collect::<Vec<_>>();
    let oversized = david
        .write(json_body(Method::POST, &path, &json!({"userIds": ids})))
        .await;
    assert_eq!(oversized.status, StatusCode::UNPROCESSABLE_ENTITY);
    let envelope: api::ApiErrorResponse = parse(&oversized);
    match envelope.error {
        api::ApiError::Validation { fields, .. } => {
            assert_eq!(fields["userIds"], vec!["too_many_recipients".to_string()]);
        }
        other => panic!("expected validation, got {other:?}"),
    }

    app.db()
        .write(|tx| {
            tx.conn()
                .execute("UPDATE users SET status=1 WHERE id=?", [JASON])?;
            Ok(())
        })
        .await
        .unwrap();
    let stale = david
        .write(json_body(
            Method::POST,
            &path,
            &json!({"userIds": [JASON.to_string()]}),
        ))
        .await;
    assert_eq!(stale.status, StatusCode::UNPROCESSABLE_ENTITY);
    let envelope: api::ApiErrorResponse = parse(&stale);
    match envelope.error {
        api::ApiError::Validation { fields, .. } => {
            assert_eq!(fields["userIds"], vec![JASON.to_string()]);
        }
        other => panic!("expected validation, got {other:?}"),
    }
}

#[tokio::test]
async fn share_checks_the_file_and_reports_partial_grants() {
    let Some((app, recorded)) = app().await else {
        return;
    };
    grant(&app, DAVID).await;
    app.db()
        .write(|tx| {
            tx.conn().execute(
                "INSERT INTO memberships (room_id, user_id, created_at, updated_at, involvement, connections)
                 SELECT ?, ?, created_at, updated_at, involvement, 0 FROM memberships WHERE room_id=? AND user_id=?",
                rusqlite::params![ALL_TALK, KEVIN, ALL_TALK, DAVID],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let kevin_email: String = app
        .db()
        .read(|conn| {
            Ok(conn.query_row(
                "SELECT email_address FROM users WHERE id=?",
                [KEVIN],
                |row| row.get(0),
            )?)
        })
        .await
        .unwrap();
    let mut david = app.sign_in(DAVID).await;
    let share = format!("/api/v1/rooms/{ALL_TALK}/drive/shares");
    let jason = &[(JASON, "jason@37signals.com")];

    recorded.answer(200, shareable("application/vnd.google-apps.folder", true));
    let folder = david
        .write(json_body(
            Method::POST,
            &share,
            &share_body(FILE, jason, &[]),
        ))
        .await;
    assert_eq!(folder.status, StatusCode::OK, "{}", folder.text());
    let folder: api::DriveShare = parse(&folder);
    assert_eq!(folder.outcome, "blocked");
    assert_eq!(folder.blocked.as_deref(), Some("folder"));
    assert!(folder.results.is_empty());

    recorded.answer(200, shareable("application/vnd.google-apps.shortcut", true));
    let shortcut = david
        .write(json_body(
            Method::POST,
            &share,
            &share_body(FILE, jason, &[]),
        ))
        .await;
    let shortcut: api::DriveShare = parse(&shortcut);
    assert_eq!(shortcut.blocked.as_deref(), Some("shortcut"));
    assert!(shortcut.results.is_empty());

    recorded.answer(
        200,
        shareable("application/vnd.google-apps.document", false),
    );
    let unshareable = david
        .write(json_body(
            Method::POST,
            &share,
            &share_body(FILE, jason, &[]),
        ))
        .await;
    let unshareable: api::DriveShare = parse(&unshareable);
    assert_eq!(unshareable.blocked.as_deref(), Some("capability"));
    assert!(unshareable.results.is_empty());
    assert_eq!(recorded.calls.lock().unwrap().len(), 3);

    recorded.answer(200, shareable("application/vnd.google-apps.document", true));
    recorded.answer(
        200,
        json!({"permissions": [{
            "id": "perm-jason",
            "type": "user",
            "role": "writer",
            "emailAddress": "Jason@37signals.com",
            "deleted": false
        }]}),
    );
    let already = david
        .write(json_body(
            Method::POST,
            &share,
            &share_body(FILE, jason, &[]),
        ))
        .await;
    let already: api::DriveShare = parse(&already);
    assert_eq!(already.outcome, "shared");
    assert_eq!(already.results[0].status, "already");
    assert!(already.results[0].reason.is_none());
    assert_eq!(recorded.calls.lock().unwrap().len(), 5);

    recorded.answer(200, shareable("application/vnd.google-apps.document", true));
    recorded.answer(200, json!({"permissions": []}));
    recorded.answer(200, json!({"id": "permission-1", "role": "reader"}));
    recorded.answer(
        403,
        json!({"error": {"errors": [{"reason": "domainPolicy"}]}}),
    );
    let partial = david
        .write(json_body(
            Method::POST,
            &share,
            &share_body(
                FILE,
                &[(JASON, "jason@37signals.com"), (KEVIN, &kevin_email)],
                &[],
            ),
        ))
        .await;
    assert_eq!(partial.status, StatusCode::OK, "{}", partial.text());
    let partial: api::DriveShare = parse(&partial);
    assert_eq!(partial.results.len(), 2);
    assert_eq!(partial.results[0].status, "granted");
    assert_eq!(partial.results[0].recipient.id, JASON);
    assert_eq!(partial.results[1].status, "failed");
    assert_eq!(partial.results[1].reason.as_deref(), Some("denied"));
    assert_eq!(partial.results[1].recipient.id, KEVIN);
    let calls = recorded.calls.lock().unwrap().clone();
    assert_eq!(calls.len(), 9);
    assert_eq!(calls[7]["method"], "POST");
    assert_eq!(calls[8]["method"], "POST");

    let full_ids = (0..10)
        .map(|index| format!("fullFileId{index}xxxx"))
        .collect::<Vec<_>>();
    let full_refs = full_ids.iter().map(String::as_str).collect::<Vec<_>>();
    let full = david
        .write(json_body(
            Method::POST,
            &share,
            &share_body(FILE, jason, &full_refs),
        ))
        .await;
    let full: api::DriveShare = parse(&full);
    assert_eq!(full.outcome, "full");
    assert!(full.results.is_empty());
    assert_eq!(recorded.calls.lock().unwrap().len(), 9);

    app.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE users SET email_address='jason.changed@37signals.com' WHERE id=?",
                [JASON],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let changed = david
        .write(json_body(
            Method::POST,
            &share,
            &share_body(FILE, jason, &[]),
        ))
        .await;
    let changed: api::DriveShare = parse(&changed);
    assert_eq!(changed.outcome, "confirmation_required");
    assert_eq!(changed.changed_ids, vec![JASON]);
    assert!(changed.results.is_empty());
    assert!(
        changed
            .recipients
            .iter()
            .any(|member| { member.id == JASON && member.email == "jason.changed@37signals.com" })
    );
    assert_eq!(recorded.calls.lock().unwrap().len(), 9);

    let outsider = david
        .write(json_body(
            Method::POST,
            &format!("/api/v1/rooms/{DIRECT_KEVIN_BENDER}/drive/shares"),
            &share_body(FILE, jason, &[]),
        ))
        .await;
    assert_eq!(
        (outsider.status, tag(&outsider)),
        (StatusCode::NOT_FOUND, "NotFound".into())
    );
    assert_eq!(recorded.calls.lock().unwrap().len(), 9);

    app.db()
        .write(|tx| {
            tx.conn()
                .execute("UPDATE users SET status=1 WHERE id=?", [KEVIN])?;
            Ok(())
        })
        .await
        .unwrap();
    let deactivated = david
        .write(json_body(
            Method::POST,
            &share,
            &share_body(FILE, &[(KEVIN, &kevin_email)], &[]),
        ))
        .await;
    assert_eq!(deactivated.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(tag(&deactivated), "Validation");
    assert_eq!(recorded.calls.lock().unwrap().len(), 9);
}

#[tokio::test]
async fn share_rejects_a_non_member_recipient_before_any_grant() {
    let Some((app, recorded)) = app().await else {
        return;
    };
    grant(&app, DAVID).await;
    let kevin_email: String = app
        .db()
        .read(|conn| {
            Ok(conn.query_row(
                "SELECT email_address FROM users WHERE id=?",
                [KEVIN],
                |row| row.get(0),
            )?)
        })
        .await
        .unwrap();
    let mut david = app.sign_in(DAVID).await;
    let rejected = david
        .write(json_body(
            Method::POST,
            &format!("/api/v1/rooms/{ALL_TALK}/drive/shares"),
            &share_body(
                FILE,
                &[(JASON, "jason@37signals.com"), (KEVIN, &kevin_email)],
                &[],
            ),
        ))
        .await;
    assert_eq!(rejected.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(tag(&rejected), "Validation");
    let envelope: api::ApiErrorResponse = parse(&rejected);
    match envelope.error {
        api::ApiError::Validation { message, fields } => {
            assert_eq!(message, "invalid_recipients");
            assert_eq!(fields["recipients"], vec![KEVIN.to_string()]);
        }
        other => panic!("expected validation, got {other:?}"),
    }
    let calls = recorded.calls.lock().unwrap();
    assert!(
        calls.is_empty(),
        "a non-member recipient must not reach Google: {calls:?}"
    );
}
