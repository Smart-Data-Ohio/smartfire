//! App adapter for campfire_mail. WS9 emits its typed notification jobs; WS5/WS8 install the
//! shared Markdown renderer; WS11 installs webhook fanout. Mail owns neither trigger.
use crate::app::{App, AppCtx};
use crate::authentication::{can_reset_password, normalize_password_reset_address};
use campfire_db::user::email_addresses::EmailAddress;
use campfire_db::{Attachment, Message, Room, Timestamp, User};
use campfire_jobs::{JobError, JobResult, Outcome};
use campfire_kit::{Ctx, Error, Response, Result, StatusCode};
use campfire_mail::{
    config::RelayAuth,
    inbound,
    jobs::{DeliveryJob, IncinerationJob, MessageCreated, Notification, RoutingJob},
    outbound::{self, SignIn},
};
use rusqlite::OptionalExtension;

pub use crate::state::mail::State;
pub fn register(registry: &mut crate::queue::Registry) {
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
async fn delivery(app: App, job: DeliveryJob, execution: campfire_jobs::Execution) -> JobResult {
    let Some(settings) = &app.mail.config.smtp else {
        return Ok(Outcome::Done);
    };
    let message = match job.notification {
        Notification::OAuthRefreshReuse { user_id } => {
            let user = app
                .db
                .read(move |conn| User::find(conn, user_id))
                .await
                .map_err(crate::queue::discard_missing)?;
            outbound::oauth_refresh_reuse(
                &app.mail.config,
                &outbound::User {
                    name: user.name,
                    email: user.email_address.unwrap_or_default(),
                },
            )
        }
        Notification::Lockout { user_id } => {
            let user = app
                .db
                .read(move |conn| User::find(conn, user_id))
                .await
                .map_err(crate::queue::discard_missing)?;
            outbound::lockout_notice(&outbound::User {
                name: user.name,
                email: user.email_address.unwrap_or_default(),
            })
        }
        Notification::PasswordReset { email_address } => {
            match password_reset_message(&app, email_address, execution.enqueued_at).await? {
                Some(message) => message,
                None => return Ok(Outcome::Done),
            }
        }
        Notification::EmailAddressConfirmation {
            user_id,
            email_address,
        } => match email_address_confirmation_message(&app, user_id, email_address).await? {
            Some(message) => message,
            None => return Ok(Outcome::Done),
        },
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
            }).await.map_err(crate::queue::discard_missing)?;
            let Some(message) = outbound::new_sign_in_alert(&app.mail.config, item.as_ref()) else {
                return Ok(Outcome::Done);
            };
            message
        }
    };
    outbound::deliver(settings, &message, app.db.env().now().jiff()).await?;
    Ok(Outcome::Done)
}
/// The reset email for the account `email_address` signs in with, its link minted now from the
/// current password and expiring 15 minutes after the request (`requested_at`, when the job was
/// queued). `None` (nothing is sent) when reset isn't available, the address has no account that
/// can reset, fewer than two minutes remain in which to use the link, or the password has changed
/// since the request.
pub async fn password_reset_message(
    app: &App,
    email_address: String,
    requested_at: Timestamp,
) -> campfire_db::Result<Option<outbound::Message>> {
    let config = &app.mail.config;
    let Some(origin) = config
        .validated_app_url()
        .filter(|_| config.password_reset_available())
    else {
        return Ok(None);
    };
    let expires_at = requested_at.jiff() + rails_compat::password_reset::EXPIRES_IN;
    let recipient = app
        .db
        .read(move |conn| {
            let Some(recipient) = reset_recipient(conn, &email_address)? else {
                return Ok(None);
            };
            let changed = password_changed_since(conn, recipient.user.id, requested_at)?;
            Ok((!changed).then_some(recipient))
        })
        .await?;
    let Some(ResetRecipient { user, email, secondary_id }) = recipient else {
        return Ok(None);
    };
    if app.db.env().now().jiff() > expires_at - jiff::SignedDuration::from_mins(2) {
        return Ok(None);
    }
    let digest = user.password_digest.as_deref().unwrap_or_default();
    let token = match secondary_id {
        Some(id) => rails_compat::password_reset::token_for_secondary(
            &app.secrets,
            user.id,
            digest,
            id,
            &email,
            requested_at.jiff(),
        ),
        None => rails_compat::password_reset::token(
            &app.secrets,
            user.id,
            digest,
            &email,
            requested_at.jiff(),
        ),
    };
    Ok(Some(outbound::password_reset(
        config,
        &outbound::User {
            name: user.name,
            email: campfire_richtext::ruby::strip(&email).to_owned(),
        },
        &format!("{origin}/app/passwords/{token}"),
    )))
}

struct ResetRecipient {
    user: User,
    email: String,
    secondary_id: Option<i64>,
}

/// An exact primary address takes precedence over a normalized match to a single active account.
fn reset_recipient(
    conn: &rusqlite::Connection,
    address: &str,
) -> campfire_db::Result<Option<ResetRecipient>> {
    let normalized = normalize_password_reset_address(address);
    if normalized.is_empty() {
        return Ok(None);
    }
    if let Some(user) = User::find_active_by_email_address(conn, address)? {
        return Ok(can_reset_password(&user).then(|| ResetRecipient {
            email: user.email_address.clone().unwrap_or_default(),
            user,
            secondary_id: None,
        }));
    }

    let mut query = conn.prepare_cached(
        "SELECT id AS user_id, email_address, NULL AS secondary_id
         FROM users WHERE status = 0 AND email_address IS NOT NULL
         UNION ALL
         SELECT users.id, link.email_address, link.id
         FROM user_email_addresses link JOIN users ON users.id = link.user_id
         WHERE users.status = 0
         ORDER BY secondary_id",
    )?;
    let mut rows = query.query([])?;
    let mut candidate = None;
    while let Some(row) = rows.next()? {
        let email: String = row.get("email_address")?;
        if normalize_password_reset_address(&email) != normalized {
            continue;
        }
        let user_id: i64 = row.get("user_id")?;
        match candidate {
            Some((matched_user, _, _)) if matched_user != user_id => return Ok(None),
            Some(_) => {}
            None => candidate = Some((user_id, email, row.get("secondary_id")?)),
        }
    }
    let Some((user_id, email, secondary_id)) = candidate else {
        return Ok(None);
    };
    Ok(User::find_by_id(conn, user_id)?
        .filter(can_reset_password)
        .map(|user| ResetRecipient { user, email, secondary_id }))
}

/// Whether the user's password was changed or reset at or after `since`, by the audit log.
fn password_changed_since(
    conn: &rusqlite::Connection,
    user_id: i64,
    since: Timestamp,
) -> campfire_db::Result<bool> {
    Ok(conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM audit_logs WHERE target_type='User' AND target_id=?1 \
         AND action IN ('user.password.change','user.password.reset') AND created_at>=?2)",
        rusqlite::params![user_id, since],
        |row| row.get(0),
    )?)
}

pub async fn email_address_confirmation_message(
    app: &App,
    user_id: i64,
    email_address: String,
) -> campfire_db::Result<Option<outbound::Message>> {
    let config = &app.mail.config;
    let Some(origin) = config
        .validated_app_url()
        .filter(|_| config.password_reset_available())
    else {
        return Ok(None);
    };
    let candidate = app
        .db
        .read(move |conn| {
            let Some(user) =
                User::find_by_id(conn, user_id)?.filter(|user| user.is_active() && !user.is_bot())
            else {
                return Ok(None);
            };
            Ok(EmailAddress::is_available(conn, &email_address)?.then_some((user, email_address)))
        })
        .await?;
    let Some((user, email_address)) = candidate else {
        return Ok(None);
    };
    let token = rails_compat::email_address_confirmation::token(
        &app.secrets,
        user.id,
        &email_address,
        app.db.env().now().jiff(),
    );
    Ok(Some(outbound::email_address_confirmation(
        config,
        &outbound::User {
            name: user.name,
            email: user.email_address.unwrap_or_default(),
        },
        &email_address,
        &format!("{origin}/app/email_addresses/confirm/{token}"),
    )))
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
pub async fn message_created(
    app: App,
    job: MessageCreated,
    _: campfire_jobs::Execution,
) -> JobResult {
    let message = app
        .db
        .read(move |conn| Message::find(conn, job.message_id))
        .await
        .map_err(crate::queue::discard_missing)?;
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
        crate::messaging::process_attachment(&app, blob)
            .await
            .map_err(|e| JobError::from(anyhow::anyhow!(e.to_string())))?;
    }
    let refreshes = app
        .db
        .read({
            let app = app.clone();
            let message = message.clone();
            move |conn| publish(conn, &app, &message)
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

fn publish(
    conn: &campfire_db::Connection,
    app: &App,
    message: &Message,
) -> campfire_db::Result<crate::presenters::RenderRefreshes> {
    let room = Room::find(conn, message.room_id)?;
    let refreshes = crate::presenters::broadcast_refreshes(conn, app, message)?;
    app.broadcasts
        .message_create(conn, &room, message, &*app.db.env().rich_text)?;
    Ok(refreshes)
}
