use super::*;
use campfire_db::Message;
use campfire_db::Room;
use campfire_db::User;
use campfire_kit::StatusCode;
use campfire_mail::inbound;
use campfire_mail::jobs::MessageCreated;
use axum::{
    body::Body,
    http::{Request, header},
};
use tower::ServiceExt;

#[tokio::test]
async fn attachment_processing_imported_mail_recovers_after_inline_failure() {
    let (app, _, id, _) = crate::controllers::messages::attachment_processing_tests::setup(true).await;
    let now = app.db().read(move |c| Ok(Message::find(c, id)?.created_at)).await.unwrap();
    message_created(app.booted.app.clone(), MessageCreated { message_id: id }, campfire_jobs::Execution {
        id: 0, executions: 1, enqueued_at: now, scheduled_at: now,
    }).await.unwrap();
    let count = app.db().read(|c| Ok(c.query_row(
        "SELECT count(*) FROM background_jobs WHERE job_class='Message::AttachmentProcessingJob'",
        [], |r| r.get::<_,i64>(0),
    )?)).await.unwrap();
    assert_eq!(count, 1, "imported mail must keep the detached presenter's recovery request");
}
const PATH: &str = "/rails/action_mailbox/relay/inbound_emails";
const RAW: &str = "Message-ID: <ws10-http@example.com>\r\nFrom: person@example.com\r\nTo: room-token@mail.test\r\n\r\nHello";
async fn boot(domain: bool, password: bool) -> (crate::server::Booted, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let cfg = crate::config::Config::from_lookup(|key| match key {
        "SECRET_KEY_BASE" => Some("ws10-isolated-http-reference-secret".repeat(4)),
        "CAMPFIRE_STORAGE_PATH" => Some(dir.path().to_string_lossy().into_owned()),
        "DISABLE_SSL" => Some("true".into()),
        "INBOUND_EMAIL_DOMAIN" if domain => Some("mail.test".into()),
        "RAILS_INBOUND_EMAIL_PASSWORD" if password => Some("fixture-mail-password".into()),
        _ => None,
    })
    .unwrap();
    (crate::server::boot(cfg).await.unwrap(), dir)
}
async fn send(
    b: &crate::server::Booted,
    auth: Option<&str>,
    mime: &str,
) -> axum::response::Response {
    let mut request = Request::builder()
        .method("POST")
        .uri(PATH)
        .header(header::CONTENT_TYPE, mime);
    if let Some(auth) = auth {
        request = request.header(header::AUTHORIZATION, auth);
    }
    b.router
        .clone()
        .oneshot(request.body(Body::from(RAW)).unwrap())
        .await
        .unwrap()
}
#[tokio::test]
async fn ws8_mail_runtime_posts_with_shared_markdown_and_queue() {
    let (b, _dir) = boot(true, true).await;
    b.app.db.write(|tx| {
        campfire_db::fixtures::load(tx.conn(), &campfire_db::fixtures::reference_dir(), &campfire_db::fixtures::Options { now: tx.now(), bcrypt_cost: 4 })
    }).await.unwrap();
    let room_id = campfire_db::fixtures::identify("designers");
    let token = b.app.db.write(move |tx| Room::find(tx.conn(), room_id)?.regenerate_inbound_email_token(tx)).await.unwrap();
    let g: serde_json::Value = serde_json::from_str(include_str!("../../../db/src/tests/ws8_mail_merge_vectors.json")).unwrap();
    let raw = g["mail"]["raw"].as_str().unwrap().replace("ws8-mail-merge-token", &token);
    let id = inbound::accept(&b.app.db, b.app.storage.clone(), raw.into_bytes()).await.unwrap().unwrap();
    let message = tokio::time::timeout(crate::test_support::WAIT, async {
        loop {
            let posted = b.app.db.read(move |conn| {
                let status: i64 = conn.query_row("SELECT status FROM action_mailbox_inbound_emails WHERE id=?", [id], |r| r.get(0))?;
                if status == inbound::Status::Delivered as i64 {
                    let message_id = conn.query_row("SELECT id FROM messages WHERE markdown_source IS NOT NULL ORDER BY id DESC LIMIT 1", [], |r| r.get(0))?;
                    Ok(Some(Message::find(conn, message_id)?))
                } else { Ok(None) }
            }).await.unwrap();
            if let Some(message) = posted { break message; }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }).await.expect("durable RoutingJob must post with the boot-installed renderer");
    let rich_text = b.app.db.env().rich_text.clone();
    let saved = b.app.db.read(move |conn| Ok(serde_json::json!({
        "source": message.markdown_source,
        "body": message.body_html(conn)?,
        "plain": message.plain_text_body(conn, &*rich_text)?,
        "creator_name": User::find(conn, message.creator_id)?.name,
    }))).await.unwrap();
    for key in ["source", "body", "plain", "creator_name"] { assert_eq!(saved[key], g["mail"][key], "{key}"); }
    b.jobs.shutdown(crate::test_support::WAIT).await;
}

#[tokio::test]
async fn ws10_relay_http_auth_matrix_without_parity_seed() {
    let (b, _dir) = boot(true, true).await;
    use base64::Engine as _;
    let wrong = format!(
        "Basic {}",
        base64::engine::general_purpose::STANDARD.encode("actionmailbox:wrong")
    );
    for auth in [None, Some("Basic invalid"), Some(wrong.as_str())] {
        let response = send(&b, auth, "message/rfc822").await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(
            response.headers()[header::WWW_AUTHENTICATE],
            "Basic realm=\"Action Mailbox\""
        );
        assert_eq!(
            axum::body::to_bytes(response.into_body(), 1024)
                .await
                .unwrap(),
            "HTTP Basic: Access denied.\n"
        );
    }
    let auth = format!(
        "Basic {}",
        base64::engine::general_purpose::STANDARD.encode("actionmailbox:fixture-mail-password")
    );
    assert_eq!(
        send(&b, Some(&auth), "text/plain").await.status(),
        StatusCode::UNSUPPORTED_MEDIA_TYPE
    );
    assert_eq!(
        send(&b, Some(&auth), "message/rfc822; charset=utf-8")
            .await
            .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        send(&b, Some(&auth), "message/rfc822").await.status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        b.app
            .db
            .read(|c| Ok(c.query_row(
                "SELECT COUNT(*) FROM action_mailbox_inbound_emails",
                [],
                |r| r.get::<_, i64>(0)
            )?))
            .await
            .unwrap(),
        1
    );
    b.jobs.shutdown(crate::test_support::WAIT).await;
}
#[tokio::test]
async fn ws10_relay_http_missing_password_refuses() {
    let (b, _dir) = boot(true, false).await;
    assert_eq!(
        send(&b, None, "message/rfc822").await.status(),
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert_eq!(
        b.app
            .db
            .read(|c| Ok(c.query_row(
                "SELECT COUNT(*) FROM action_mailbox_inbound_emails",
                [],
                |r| r.get::<_, i64>(0)
            )?))
            .await
            .unwrap(),
        0
    );
    b.jobs.shutdown(crate::test_support::WAIT).await;
}
#[tokio::test]
async fn ws10_relay_http_disabled_domain_refuses() {
    let (b, _dir) = boot(false, true).await;
    assert_eq!(
        send(&b, None, "message/rfc822").await.status(),
        StatusCode::NOT_FOUND
    );
    b.jobs.shutdown(crate::test_support::WAIT).await;
}
#[tokio::test]
async fn ws10_relay_http_enqueue_failure_rolls_back_acceptance() {
    let (b, _dir) = boot(true, true).await;
    b.app.db.write(|tx| {
        tx.conn().execute_batch("CREATE TRIGGER ws10_reject_routing BEFORE INSERT ON background_jobs BEGIN SELECT RAISE(ABORT, 'ws10 routing enqueue rejected'); END")?;
        Ok(())
    }).await.unwrap();
    use base64::Engine as _;
    let auth = format!(
        "Basic {}",
        base64::engine::general_purpose::STANDARD.encode("actionmailbox:fixture-mail-password")
    );
    assert_eq!(
        send(&b, Some(&auth), "message/rfc822").await.status(),
        StatusCode::INTERNAL_SERVER_ERROR
    );
    let counts = b
        .app
        .db
        .read(|conn| {
            [
                "action_mailbox_inbound_emails",
                "active_storage_blobs",
                "active_storage_attachments",
                "background_jobs",
            ]
            .map(|table| {
                conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get::<_, i64>(0)
                })
            })
            .into_iter()
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
        })
        .await
        .unwrap();
    assert_eq!(counts, [0, 0, 0, 0]);
    b.app
        .db
        .write(|tx| {
            tx.conn()
                .execute_batch("DROP TRIGGER ws10_reject_routing")?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(
        send(&b, Some(&auth), "message/rfc822").await.status(),
        StatusCode::NO_CONTENT
    );
    b.jobs.shutdown(crate::test_support::WAIT).await;
}
