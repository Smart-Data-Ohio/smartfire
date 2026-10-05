//! App adapter for campfire_mail. WS9 emits its typed notification jobs; WS5/WS8 install the
//! shared Markdown renderer; WS11 installs webhook fanout. Mail owns neither trigger.
use crate::{
    app::{App, AppCtx},
    controllers::presenters::{
        Presenter,
        page::{self, Rendered},
    },
};
use campfire_db::{Account, Attachment, Message, Room, Timestamp, User};
use campfire_jobs::{JobError, JobResult, Outcome};
use campfire_kit::{Ctx, Error, Response, Result, StatusCode};
use campfire_mail::{
    config::{Config, RelayAuth},
    inbound::{self, Renderer, Throttle},
    jobs::{DeliveryJob, IncinerationJob, MessageCreated, Notification, RoutingJob},
    outbound::{self, SignIn},
};
use rusqlite::OptionalExtension;
use std::sync::{Arc, RwLock};

type Fanout =
    Arc<dyn Fn(&mut campfire_db::Tx<'_>, &Message) -> campfire_db::Result<()> + Send + Sync>;
pub struct State {
    pub config: Config,
    throttle: Throttle,
    renderer: RwLock<Option<Arc<dyn Renderer>>>,
    fanout: RwLock<Option<Fanout>>,
}
impl State {
    #[cfg(test)]
    pub(crate) fn fixture_snapshot(&self) -> Self {
        Self {
            config: self.config.clone(),
            throttle: self.throttle.clone(),
            renderer: RwLock::new(self.renderer.read().unwrap_or_else(|p| p.into_inner()).clone()),
            fanout: RwLock::new(self.fanout.read().unwrap_or_else(|p| p.into_inner()).clone()),
        }
    }

    pub fn new(config: Config) -> Self {
        Self {
            config,
            throttle: Throttle::default(),
            renderer: RwLock::new(None),
            fanout: RwLock::new(None),
        }
    }
    /// Called at boot by WS5/WS8 after the shared Markdown/mention renderer has landed.
    pub fn install_renderer(&self, renderer: Arc<dyn Renderer>) {
        *self.renderer.write().unwrap_or_else(|e| e.into_inner()) = Some(renderer);
    }
    /// WS11's Message::BotWebhookFanout hook, called inside a write after the broadcast.
    #[allow(dead_code)]
    pub fn install_fanout(&self, fanout: Fanout) {
        *self.fanout.write().unwrap_or_else(|e| e.into_inner()) = Some(fanout);
    }
}
pub fn register(registry: &mut crate::jobs::Registry) {
    registry.register(delivery);
    registry.register(routing);
    registry.register(incineration);
    registry.register(message_created);
}
pub async fn relay(c: &mut Ctx) -> Result {
    match campfire_mail::config::relay_auth(&c.app().mail.config, c.request.header("authorization"))
    {
        RelayAuth::Disabled => return Ok(c.head(StatusCode::NOT_FOUND)),
        RelayAuth::MissingPassword => {
            return Err(Error::internal(anyhow::anyhow!(
                "Missing required ingress credentials"
            )));
        }
        RelayAuth::Unauthorized => {
            return Ok(Response::with_body(
                StatusCode::UNAUTHORIZED,
                "text/html; charset=utf-8",
                "HTTP Basic: Access denied.\n",
            )
            .header("www-authenticate", "Basic realm=\"Action Mailbox\""));
        }
        RelayAuth::Accepted => {}
    }
    if c.request
        .header("content-type")
        .and_then(|s| s.split(';').next())
        .is_none_or(|s| s.trim() != "message/rfc822")
    {
        return Ok(c.head(StatusCode::UNSUPPORTED_MEDIA_TYPE));
    }
    inbound::accept(
        &c.app().db,
        c.app().storage.clone(),
        c.request.raw_post().to_vec(),
    )
    .await
    .map_err(Error::internal)?;
    Ok(c.head(StatusCode::NO_CONTENT))
}
async fn delivery(app: App, job: DeliveryJob, _: campfire_jobs::Execution) -> JobResult {
    let Some(settings) = &app.mail.config.smtp else {
        return Ok(Outcome::Done);
    };
    let message = match job.notification {
        Notification::Lockout { user_id } => {
            let user = app
                .db
                .read(move |conn| User::find(conn, user_id))
                .await
                .map_err(crate::jobs::discard_missing)?;
            outbound::lockout_notice(&outbound::User {
                name: user.name,
                email: user.email_address.unwrap_or_default(),
            })
        }
        Notification::NewSignIn { activity_item_id } => {
            let item = app.db.read(move |conn| {
                let row = conn.query_row("SELECT user_id, source_id, source_type, created_at FROM activity_items WHERE id = ?", [activity_item_id], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?, r.get::<_, String>(2)?, r.get::<_, Timestamp>(3)?))).optional()?.ok_or(campfire_db::Error::RecordNotFound("ActivityItem"))?;
                let user = User::find(conn, row.0)?;
                if row.2 != "Session" {return Ok(None);}
                let ua = conn.query_row("SELECT user_agent FROM sessions WHERE id = ?", [row.1], |r| r.get::<_, Option<String>>(0)).optional()?;
                let Some(ua) = ua else {return Ok(None);};
                let platform = crate::concerns::platform::ApplicationPlatform::new(ua.as_deref());
                let browser = if ua.as_deref().is_some_and(|s| s.contains("Edg/")) {"Edge"} else if platform.chrome() {"Chrome"} else if platform.firefox() {"Firefox"} else if platform.safari() {"Safari"} else {"Unknown browser"};
                Ok(Some(SignIn {user: outbound::User {name: user.name, email: user.email_address.unwrap_or_default()}, device: format!("{browser} on {}", platform.operating_system().filter(|s| !s.is_empty()).unwrap_or_else(|| "Unknown device".into())), created_at: row.3.jiff()}))
            }).await.map_err(crate::jobs::discard_missing)?;
            let Some(message) = outbound::new_sign_in_alert(&app.mail.config, item.as_ref()) else {
                return Ok(Outcome::Done);
            };
            message
        }
    };
    outbound::deliver(settings, &message, app.db.env().now().jiff()).await?;
    Ok(Outcome::Done)
}
async fn routing(app: App, job: RoutingJob, _: campfire_jobs::Execution) -> JobResult {
    let renderer = app
        .mail
        .renderer
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    let routed = inbound::route(
        &app.db,
        app.storage.clone(),
        app.mail.config.clone(),
        app.mail.throttle.clone(),
        renderer,
        job.inbound_email_id,
    )
    .await
    .map_err(|error| {
        if error
            .downcast_ref::<campfire_db::Error>()
            .is_some_and(|e| matches!(e, campfire_db::Error::RecordNotFound(_)))
        {
            JobError::discard(error)
        } else {
            JobError::from(error)
        }
    })?;
    if routed == inbound::Routed::WaitingForRenderer {
        tracing::warn!("mail routing waits for WS5/WS8 Markdown renderer installation");
        return Ok(Outcome::Again(std::time::Duration::from_secs(60)));
    }
    Ok(Outcome::Done)
}
async fn incineration(app: App, job: IncinerationJob, _: campfire_jobs::Execution) -> JobResult {
    app.db
        .write(move |tx| inbound::incinerate(tx, job.inbound_email_id))
        .await?;
    Ok(Outcome::Done)
}
async fn message_created(app: App, job: MessageCreated, _: campfire_jobs::Execution) -> JobResult {
    let message = app
        .db
        .read(move |conn| Message::find(conn, job.message_id))
        .await
        .map_err(crate::jobs::discard_missing)?;
    if let Some(attachment) = app
        .db
        .read({
            let id = message.id;
            move |conn| Attachment::find_for(conn, "Message", id, "attachment")
        })
        .await?
    {
        let blob = app
            .db
            .read(move |conn| {
                campfire_storage::Blob::find(conn, attachment.blob_id)
                    .map_err(crate::controllers::presenters::storage_error)
            })
            .await?
            .ok_or_else(|| JobError::discard(anyhow::anyhow!("attachment blob was removed")))?;
        crate::controllers::messages::process_attachment(&app, blob)
            .await
            .map_err(|e| JobError::from(anyhow::anyhow!(e.to_string())))?;
    }
    let refreshes = app.db
        .read({
            let app = app.clone();
            let message = message.clone();
            move |conn| {
                let room = Room::find(conn, message.room_id)?;
                let presenter = Presenter::new(conn, &app, None);
                let view = presenter.message(&message)?;
                let account = Account::first(conn)?;
                let html = page::render_detached(&app, account.as_ref(), |ctx| {
                    campfire_views::messages::message(ctx, &view)
                });
                app.broadcasts.message_create(
                    conn,
                    &room,
                    &message,
                    &Rendered {
                        message: Some(html),
                        ..Rendered::default()
                    },
                    &*app.db.env().rich_text,
                )?;
                Ok(presenter.take_render_refreshes())
            }
        })
        .await?;
    crate::controllers::presenters::refresh_after_render(&app.db, refreshes).await;
    let fanout = app
        .mail
        .fanout
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    if let Some(fanout) = fanout {
        app.db.write(move |tx| fanout(tx, &message)).await?;
    }
    Ok(Outcome::Done)
}

#[cfg(test)]
mod tests {
    use super::*;
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
    async fn boot(domain: bool, password: bool) -> (crate::app::Booted, tempfile::TempDir) {
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
        (crate::app::boot(cfg).await.unwrap(), dir)
    }
    async fn send(
        b: &crate::app::Booted,
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
        let g: serde_json::Value = serde_json::from_str(include_str!("../../db/src/tests/ws8_mail_merge_vectors.json")).unwrap();
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
}
