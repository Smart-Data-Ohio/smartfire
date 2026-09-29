//! Active Mailbox ingress storage, status callbacks, RoomMailbox and BounceMailbox.
use crate::{
    config::Config,
    jobs::{IncinerationJob, MessageCreated, RoutingJob},
    parse::{Email, Verdict, authenticated_sender},
};
use campfire_db::{
    Attachment, Connection, Database, Event, Membership, Message, NewMessage, Room, Timestamp, Tx,
    User,
};
use campfire_storage::{Filename, Staged, Storage};
use rusqlite::{OptionalExtension, params};
use sha1::{Digest, Sha1};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

pub const INCINERATE_AFTER: Duration = Duration::from_secs(30 * 24 * 3600);
pub const RECORD_TYPE: &str = "ActionMailbox::InboundEmail";
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i64)]
pub enum Status {
    Pending = 0,
    Processing = 1,
    Delivered = 2,
    Failed = 3,
    Bounced = 4,
}
impl Status {
    pub fn processed(self) -> bool {
        matches!(self, Self::Delivered | Self::Failed | Self::Bounced)
    }
}
#[derive(Clone, Default)]
pub struct Throttle(Arc<Mutex<HashMap<(i64, i64), u32>>>);
impl Throttle {
    pub fn rate_limited(&self, room_id: i64, now: Timestamp) -> bool {
        let hour = now.as_second().div_euclid(3600);
        let mut counters = self.0.lock().unwrap_or_else(|e| e.into_inner());
        counters.retain(|(_, bucket), _| *bucket >= hour - 1);
        let count = counters.entry((room_id, hour)).or_default();
        *count = count.saturating_add(1);
        *count > 30
    }
}
fn storage_error(error: campfire_storage::Error) -> campfire_db::Error {
    campfire_db::Error::Other(error.to_string())
}
/// Accepting the email, blob and RoutingJob is one SQLite transaction.
/// Files are staged outside it, and discarded automatically on duplicate acceptance or rollback.
pub async fn accept(
    db: &Database,
    storage: Arc<Storage>,
    raw: Vec<u8>,
) -> anyhow::Result<Option<i64>> {
    let checksum = hex::encode(Sha1::digest(&raw));
    use mailparse::MailHeaderMap as _;
    let message_id = mailparse::parse_headers(&raw)
        .ok()
        .and_then(|(headers, _)| headers.get_first_value("Message-ID"))
        .map(|value| {
            value
                .trim()
                .trim_start_matches('<')
                .trim_end_matches('>')
                .to_owned()
        })
        .unwrap_or_else(|| {
            format!(
                "{checksum}@{}.mail",
                std::env::var("HOSTNAME").unwrap_or_else(|_| "localhost".into())
            )
        });
    let staged = tokio::task::spawn_blocking(move || {
        storage.stage_bytes(&raw, Filename::new("message.eml"), Some("message/rfc822"))
    })
    .await??;
    let (id, staged) = db.write(move |tx| {
        let id = tx.conn().query_row("INSERT INTO action_mailbox_inbound_emails (message_id, message_checksum, status, created_at, updated_at) VALUES (?, ?, 0, ?, ?) ON CONFLICT(message_id, message_checksum) DO NOTHING RETURNING id", params![message_id, checksum, tx.now(), tx.now()], |r| r.get::<_, i64>(0)).optional()?;
        if let Some(id) = id {
            let blob = staged.insert(tx.conn(), tx.now().jiff()).map_err(storage_error)?;
            Attachment::create(tx, RECORD_TYPE, id, "raw_email", blob.id)?;
            tx.emit_after_commit(Event::job(&RoutingJob {inbound_email_id: id}));
        }
        Ok((id, staged))
    }).await?;
    if id.is_some() {
        staged.keep();
    }
    Ok(id)
}
pub fn set_status(tx: &mut Tx<'_>, id: i64, status: Status) -> campfire_db::Result<()> {
    let before = tx
        .conn()
        .query_row(
            "SELECT status FROM action_mailbox_inbound_emails WHERE id = ?",
            [id],
            |r| r.get::<_, i64>(0),
        )
        .optional()?
        .ok_or(campfire_db::Error::RecordNotFound(RECORD_TYPE))?;
    if before != status as i64 {
        tx.conn().execute(
            "UPDATE action_mailbox_inbound_emails SET status = ?, updated_at = ? WHERE id = ?",
            params![status as i64, tx.now(), id],
        )?;
        if status.processed() {
            tx.emit_after_commit(Event::job_in(
                INCINERATE_AFTER,
                &IncinerationJob {
                    inbound_email_id: id,
                },
            ));
        }
    }
    Ok(())
}
pub fn incinerate(tx: &mut Tx<'_>, id: i64) -> campfire_db::Result<bool> {
    let row = tx
        .conn()
        .query_row(
            "SELECT status, updated_at FROM action_mailbox_inbound_emails WHERE id = ?",
            [id],
            |r| Ok((r.get::<_, i64>(0)?, r.get::<_, Timestamp>(1)?)),
        )
        .optional()?;
    let Some((status, updated_at)) = row else {
        return Ok(false);
    };
    // Rails: updated_at < incinerate_after.ago.end_of_day (not a rolling exact timestamp).
    let cutoff = tx
        .now()
        .as_second()
        .saturating_sub(INCINERATE_AFTER.as_secs() as i64)
        .div_euclid(86400)
        * 86400
        + 86400;
    if !matches!(status, 2..=4) || updated_at.as_microsecond() >= cutoff * 1_000_000 - 1 {
        return Ok(false);
    }
    if let Some(attachment) = Attachment::find_for(tx.conn(), RECORD_TYPE, id, "raw_email")? {
        attachment.delete(tx)?;
        tx.emit_after_commit(Event::PurgeBlob {
            blob_id: attachment.blob_id,
        });
    }
    tx.conn().execute(
        "DELETE FROM action_mailbox_inbound_emails WHERE id = ?",
        [id],
    )?;
    Ok(true)
}
/// Markdown and mention rendering belongs to WS5/WS8; the app injects its renderer here.
pub trait Renderer: Send + Sync {
    fn render(&self, conn: &Connection, room: &Room, source: &str) -> campfire_db::Result<String>;
}
impl<F> Renderer for F
where
    F: Fn(&Connection, &Room, &str) -> campfire_db::Result<String> + Send + Sync,
{
    fn render(&self, conn: &Connection, room: &Room, source: &str) -> campfire_db::Result<String> {
        self(conn, room, source)
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Routed {
    Dropped,
    Bounced,
    Posted(i64),
    WaitingForRenderer,
}
fn find_room(conn: &Connection, token: &str) -> campfire_db::Result<Option<Room>> {
    let id = conn.query_row("SELECT id FROM rooms WHERE inbound_email_token = ? AND deleted_at IS NULL AND type NOT IN ('Rooms::Direct', 'Rooms::Board') LIMIT 1", [token], |r| r.get::<_, i64>(0)).optional()?;
    id.map(|id| Room::find(conn, id)).transpose()
}
fn creator(
    tx: &mut Tx<'_>,
    room: &Room,
    email: &Email,
    cfg: &Config,
) -> campfire_db::Result<(User, bool)> {
    if let Some(address) = &email.from {
        let user = tx.conn().query_row("SELECT users.id FROM users JOIN memberships ON memberships.user_id = users.id WHERE users.status = 0 AND users.role != 2 AND memberships.room_id = ? AND LOWER(users.email_address) = ? LIMIT 1", params![room.id, address.to_lowercase()], |r| r.get::<_, i64>(0)).optional()?;
        if let Some(id) = user
            && authenticated_sender(&email.auth_headers, cfg.authserv_id.as_deref(), address)
        {
            return Ok((User::find(tx.conn(), id)?, true));
        }
    }
    let id = tx
        .conn()
        .query_row(
            "SELECT id FROM users WHERE status = 0 AND role = 2 AND name = 'Email' LIMIT 1",
            [],
            |r| r.get::<_, i64>(0),
        )
        .optional()?;
    let bot = if let Some(id) = id {
        User::find(tx.conn(), id)?
    } else {
        User::create_email_bot(tx)?
    };
    if Membership::find_by_room_and_user(tx.conn(), room.id, bot.id)?.is_none() {
        // Membership.find_or_create_by!(user:) uses column defaults and the create-time
        // default_stage_role callback. Room.grant_to uses insert_all and skips that callback.
        tx.conn().execute(
            "INSERT INTO memberships (room_id, user_id, stage_role, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
            params![room.id, bot.id, room.stage().then_some("listener"), tx.now(), tx.now()],
        )?;
    }
    Ok((bot, false))
}
/// Transactional posting through WS2 Message::create_markdown, with Event-based side effects.
pub fn post(
    tx: &mut Tx<'_>,
    email: &Email,
    config: &Config,
    throttle: &Throttle,
    renderer: Option<&dyn Renderer>,
    attachment: Option<&Staged>,
) -> campfire_db::Result<Routed> {
    let Some(token) = email.room_token() else {
        return Ok(Routed::Bounced);
    };
    if config.domain.is_none() {
        return Ok(Routed::Dropped);
    }
    let Some(room) = find_room(tx.conn(), &token)? else {
        return Ok(Routed::Dropped);
    };
    if renderer.is_none() && email.source(false).is_some() {
        return Ok(Routed::WaitingForRenderer);
    }
    if throttle.rate_limited(room.id, tx.now()) {
        return Ok(Routed::Dropped);
    }
    let (creator, member) = creator(tx, &room, email, config)?;
    let Some(source) = email.source(member) else {
        return Ok(Routed::Dropped);
    };
    let renderer = renderer
        .ok_or_else(|| campfire_db::Error::Other("mail Markdown renderer is unavailable".into()))?;
    let body = renderer.render(tx.conn(), &room, &source)?;
    let blob_id = attachment
        .map(|staged| {
            staged
                .insert(tx.conn(), tx.now().jiff())
                .map(|b| b.id)
                .map_err(storage_error)
        })
        .transpose()?;
    let message = Message::create_markdown(
        tx,
        NewMessage {
            room_id: room.id,
            creator_id: creator.id,
            body: Some(body),
            attachment_blob_id: blob_id,
            ..Default::default()
        },
        &source,
    )?;
    tx.emit_after_commit(Event::job(&MessageCreated {
        message_id: message.id,
    }));
    Ok(Routed::Posted(message.id))
}
pub async fn route(
    db: &Database,
    storage: Arc<Storage>,
    config: Config,
    throttle: Throttle,
    renderer: Option<Arc<dyn Renderer>>,
    id: i64,
) -> anyhow::Result<Routed> {
    let key = db
        .write(move |tx| {
            set_status(tx, id, Status::Processing)?;
            let attachment = Attachment::find_for(tx.conn(), RECORD_TYPE, id, "raw_email")?
                .ok_or(campfire_db::Error::RecordNotFound("raw_email"))?;
            Ok(attachment.blob(tx.conn())?.key)
        })
        .await?;
    let raw_storage = storage.clone();
    let parsed = tokio::task::spawn_blocking(move || {
        Email::parse(&std::fs::read(raw_storage.service.path_for(&key))?)
    })
    .await?;
    let result = async {
        let email = parsed?;
        let staged = if renderer.is_some()
            && let Some(file) = email
                .files
                .iter()
                .find(|f| !f.filename.trim().is_empty() && f.verdict == Verdict::Ok)
                .cloned()
        {
            Some(
                tokio::task::spawn_blocking(move || {
                    storage.stage_bytes(
                        &file.bytes,
                        Filename::new(file.filename),
                        Some(&file.content_type),
                    )
                })
                .await??,
            )
        } else {
            None
        };
        let routed = db
            .write(move |tx| {
                let routed = post(
                    tx,
                    &email,
                    &config,
                    &throttle,
                    renderer.as_deref(),
                    staged.as_ref(),
                )?;
                if matches!(routed, Routed::Posted(_))
                    && let Some(staged) = staged
                {
                    // Keep the committed file even when an earlier after-commit callback fails.
                    // On rollback the queued closure drops its staged file instead.
                    tx.after_commit(move |_| {
                        staged.keep();
                        Ok(())
                    });
                }
                set_status(
                    tx,
                    id,
                    if routed == Routed::WaitingForRenderer {
                        Status::Pending
                    } else if routed == Routed::Bounced {
                        Status::Bounced
                    } else {
                        Status::Delivered
                    },
                )?;
                Ok(routed)
            })
            .await?;
        Ok::<_, anyhow::Error>(routed)
    }
    .await;
    if result.is_err() {
        db.write(move |tx| set_status(tx, id, Status::Failed))
            .await?;
    }
    result
}
