//! The jobs integrations perform for the app's job runner: `Room::PushMessageJob`
//! (reference/app/jobs/room/push_message_job.rb) and `Bot::WebhookJob`
//! (reference/app/jobs/bot/webhook_job.rb), including what `Webhook#deliver` does with a reply
//! (create the bot's message, process an attachment, `broadcast_create`).

use anyhow::{Context as _, anyhow};
use campfire_db::{Database, Message, NewMessage, PushSubscription, Room, User, Webhook};
use campfire_jobs::{Execution, JobResult, Outcome};
use campfire_views::messages as views;

use super::net::Network;
use super::web_push::{self, VapidConfig, VapidError};
use super::webhook::{self, WebhookReply};
use crate::app::App;
use crate::config::Config;
use crate::controllers::presenters::page::{self, Rendered};
use crate::controllers::messages::{canonicalize_body, process_attachment, save_staged};
use crate::controllers::presenters::Presenter;
use crate::jobs::{PushMessageJob, Registry, WebhookJob, discard_missing};

/// Registers `Room::PushMessageJob` and `Bot::WebhookJob`.
pub fn register_jobs(registry: &mut Registry) {
    super::agent_jobs::register(registry);
    registry.register(push_message);
    registry.register(deliver_webhook);
    super::github::jobs::register(registry);
}

/// `Room::PushMessageJob#perform(room, message)`: `Room::MessagePusher.new(room:, message:).push`,
/// unless Web Push is off. A room or message that's gone discards the job.
async fn push_message(app: App, job: PushMessageJob, _: Execution) -> JobResult {
    let Some(pool) = app.web_push.clone() else { return Ok(Outcome::Done) };
    let PushMessageJob { room_id, message_id } = job;
    let message = app
        .db
        .read(move |conn| {
            Room::find(conn, room_id)?;
            Message::find(conn, message_id)
        })
        .await
        .map_err(discard_missing)?;
    let db = app.db.clone();
    app.db.read(move |conn| web_push::push_message(&pool, conn, &*db.env().rich_text, &message, db.env().now()).map(|_| ())).await?;
    Ok(Outcome::Done)
}

/// config/initializers/web_push.rb (`config.x.web_push_pool`): the pool, whose invalid
/// subscription handler destroys the subscription (`Push::Subscription.find_by(id:)&.destroy`).
/// `None`, and Web Push is off, when the VAPID keys are missing or invalid. Call from inside the
/// runtime.
pub fn web_push_pool(config: &Config, db: &Database) -> Option<web_push::Pool> {
    let vapid = match VapidConfig::from_config(config) {
        Ok(vapid) => vapid,
        Err(error @ VapidError::Missing) => {
            tracing::warn!("Web Push is off: {error}");
            return None;
        }
        Err(error) => {
            tracing::error!("Web Push is off: {error}");
            return None;
        }
    };
    let db = db.clone();
    Some(web_push::Pool::new(Network::system(), vapid, move |id| {
        db.write_blocking(move |tx| match PushSubscription::find(tx.conn(), id) {
            Ok(subscription) => subscription.destroy(tx),
            Err(campfire_db::Error::RecordNotFound(_)) => Ok(()),
            Err(error) => Err(error),
        })
    }))
}

/// `Bot::WebhookJob#perform(bot, message)`: `bot.deliver_webhook(message)`, i.e.
/// `webhook.deliver(message)`, then the reply.
async fn deliver_webhook(app: App, job: WebhookJob, _: Execution) -> JobResult {
    let WebhookJob { bot_id, message_id } = job;
    // A bot or message that's gone discards the job.
    let (bot, message) = app.db.read(move |conn| Ok((User::find(conn, bot_id)?, Message::find(conn, message_id)?))).await.map_err(discard_missing)?;
    let trigger = message.clone();
    let now = app.clock.now();
    let secrets = app.secrets.clone();
    let encryption = app.ar_encryption.clone();
    let db = app.db.clone();
    let (bot, room, url, payload, secret) = app
        .db
        .read(move |conn| {
            let room = Room::find(conn, message.room_id)?;
            let Some(webhook) = Webhook::find_by_user(conn, bot_id)? else { return Ok(None) };
            let agent_backed = conn.query_row("SELECT EXISTS(SELECT 1 FROM agents WHERE user_id = ?)", [bot.id], |row| row.get::<_, bool>(0))?;
            let room_path = if agent_backed { campfire_routes::room(room.id) } else {
                let token = rails_compat::verifiers::bot_reply::token_for(&secrets, bot.id, room.id, now);
                campfire_routes::room_bot_messages(room.id, &token)
            };
            let secret = webhook.signing_secret(&encryption)?;
            let payload = webhook.payload(
                conn,
                &*db.env().rich_text,
                &message,
                &room_path,
                &campfire_routes::room_at_message(room.id, message.id),
            )?;
            Ok(Some((bot, room, webhook.url, payload, secret)))
        })
        .await?
        .context("undefined method 'deliver' for nil (the bot has no webhook)")?;

    let delivery = webhook::deliver_signed(&Network::system(), url.as_deref().unwrap_or(""), payload, secret.as_deref(), || app.clock.now(), false).await?;
    let message = match delivery.reply {
        WebhookReply::None => return Ok(Outcome::Done),
        WebhookReply::Text(text) => create_text_reply(&app, &room, &bot, delivery.status.map(|_| trigger.clone()), text).await?,
        WebhookReply::Attachment(attachment) => create_attachment_reply(&app, &room, &bot, trigger, attachment).await?,
    };
    broadcast_create(&app, &room, &message).await?;
    Ok(Outcome::Done)
}

/// `room.messages.create!(body: text, creator: user)`: the text is assigned to the rich text
/// body, which stores it canonicalized (as `MessagesController` does; there's no request host
/// in a job).
pub(super) async fn create_text_reply(app: &App, room: &Room, bot: &User, trigger: Option<Message>, text: String) -> anyhow::Result<Message> {
    let (room_id, creator_id) = (room.id, bot.id);
    let body = canonicalize_body(app, text, None).await.map_err(|e| anyhow!("{e:?}"))?;
    let message = app
        .db
        .write(move |tx| create_reply(tx, trigger.as_ref(), NewMessage { room_id, creator_id, body: Some(body), ..Default::default() }))
        .await?;
    Ok(message)
}

/// `ActiveStorage::Blob.create_and_upload!` (its own save), then
/// `room.messages.create_with_attachment!(attachment:, creator: user)`, which processes the
/// attachment.
pub(super) async fn create_attachment_reply(app: &App, room: &Room, bot: &User, trigger: Message, attachment: webhook::Attachment) -> anyhow::Result<Message> {
    let storage = app.storage.clone();
    let staged = tokio::task::spawn_blocking(move || attachment.stage_blob(&storage)).await??;
    let blob = app.db.write(move |tx| save_staged(tx, staged)).await?;

    let (room_id, creator_id, blob_id) = (room.id, bot.id, blob.id);
    let message = app
        .db
        .write(move |tx| create_reply(tx, Some(&trigger), NewMessage { room_id, creator_id, attachment_blob_id: Some(blob_id), ..Default::default() }))
        .await?;
    process_attachment(app, blob).await.map_err(|e| anyhow!("{e:?}"))?;
    let id = message.id;
    Ok(app.db.read(move |conn| Message::find(conn, id)).await?)
}

/// `Webhook#create_sync_reply`: root replies reference the trigger; thread replies use
/// ChannelThread#post_message! so membership, locking, reopening and counters stay shared.
fn create_reply(tx: &mut campfire_db::Tx<'_>, trigger: Option<&Message>, mut attributes: NewMessage) -> campfire_db::Result<Message> {
    attributes.reply_to_message_id = trigger.map(|message| message.id);
    if let Some(thread_id) = trigger.and_then(|message| message.thread_id) {
        campfire_db::ChannelThread::find(tx.conn(), thread_id)?.post_message(tx, attributes.creator_id, attributes)
    } else {
        Message::create(tx, attributes)
    }
}

/// `message.broadcast_create`, rendered without a request (`ApplicationController.renderer`).
pub(super) async fn broadcast_create(app: &App, room: &Room, message: &Message) -> anyhow::Result<()> {
    let (app, room, message) = (app.clone(), room.clone(), message.clone());
    let db = app.db.clone();
    db.read(move |conn| {
        let presenter = Presenter::new(conn, &app, None);
        let view = presenter.message(&message)?;
        let account = campfire_db::Account::first(conn)?;
        let html = page::render_detached(&app, account.as_ref(), |ctx| views::message(ctx, &view));
        let partials = Rendered { message: Some(html), ..Rendered::default() };
        app.broadcasts.message_create(conn, &room, &message, &partials, &*app.db.env().rich_text)
    })
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controllers::presenters::test_support::{TestApp, ALL_TALK, BENDER, DAVID};

    #[test]
    fn ws11_legacy_webhook_retry_policy_keeps_transient_sources() {
        use campfire_jobs::JobKind;
        use crate::integrations::net::http::HttpError;
        use crate::integrations::webhook::WebhookError;
        let policy = WebhookJob::retry_policy();
        let timeout = anyhow::Error::new(WebhookError::Http(HttpError::Io(std::io::Error::new(std::io::ErrorKind::TimedOut, "write timed out"))));
        assert!((policy.retry_on)(&timeout));
        let refused = anyhow::Error::new(WebhookError::Http(HttpError::Io(std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "refused"))));
        assert!(!(policy.retry_on)(&refused));
        assert!(!(policy.retry_on)(&anyhow::Error::new(WebhookError::InvalidUrl("bad URI".into()))));
        let delays: Vec<_> = (1..=5).map(|attempt| policy.retry_delay(attempt, None, 0.0).map(|delay| delay.as_secs())).collect();
        assert_eq!(delays, vec![Some(3), Some(18), Some(83), Some(258), None]);
    }

    #[tokio::test]
    async fn ws11_sync_replies_use_root_links_threads_boards_and_locking() {
        let app = TestApp::boot().await.expect("build the default parity seed");
        app.db().write(|tx| {
            let attrs = || NewMessage { room_id: ALL_TALK, creator_id: DAVID, body: Some("Trigger".into()), ..Default::default() };
            let root = Message::create(tx, attrs())?;
            let root_reply = create_reply(tx, Some(&root), NewMessage { room_id: ALL_TALK, creator_id: BENDER, body: Some("Answer".into()), ..Default::default() })?;
            assert_eq!(root_reply.reply_to_message_id, Some(root.id));
            assert_eq!(root_reply.thread_id, None);
            let timeout = create_reply(tx, None, NewMessage { room_id: ALL_TALK, creator_id: BENDER, body: Some("Failed to respond within 7 seconds".into()), ..Default::default() })?;
            assert_eq!(timeout.reply_to_message_id, None);
            let mut thread = campfire_db::ChannelThread::create(tx, campfire_db::NewChannelThread { room_id: ALL_TALK, creator_id: DAVID, name: Some("Replies".into()), ..Default::default() })?;
            let trigger = thread.post_message(tx, DAVID, attrs())?;
            let reply = create_reply(tx, Some(&trigger), NewMessage { room_id: ALL_TALK, creator_id: BENDER, body: Some("Thread answer".into()), ..Default::default() })?;
            assert_eq!(reply.thread_id, Some(thread.id));
            assert_eq!(reply.reply_to_message_id, Some(trigger.id));
            assert_eq!(campfire_db::ChannelThread::find(tx.conn(), thread.id)?.messages_count, 2);
            tx.conn().execute("UPDATE rooms SET type='Rooms::Board' WHERE id=?", [ALL_TALK])?;
            tx.conn().execute("UPDATE channel_threads SET work_status='in_progress' WHERE id=?", [thread.id])?;
            thread = campfire_db::ChannelThread::find(tx.conn(), thread.id)?;
            let board_reply = create_reply(tx, Some(&trigger), NewMessage { room_id: ALL_TALK, creator_id: BENDER, body: Some("Board answer".into()), ..Default::default() })?;
            assert_eq!(board_reply.thread_id, Some(thread.id));
            assert_eq!(board_reply.reply_to_message_id, Some(trigger.id));
            thread.lock_conversation(tx)?;
            assert!(matches!(create_reply(tx, Some(&trigger), NewMessage { room_id: ALL_TALK, creator_id: BENDER, body: Some("Locked".into()), ..Default::default() }), Err(campfire_db::Error::Other(error)) if error == campfire_db::models::channel_thread::LOCKED_MESSAGE));
            Ok(())
        }).await.unwrap();
    }

    #[tokio::test]
    async fn ws11_sync_attachment_reply_is_stored_in_the_trigger_thread() {
        let test_app = TestApp::boot().await.expect("build the default parity seed");
        let app = &test_app.booted.app;
        let (room, bot, trigger) = app.db.write(|tx| {
            let room = Room::find(tx.conn(), ALL_TALK)?;
            let bot = User::find(tx.conn(), BENDER)?;
            let mut thread = campfire_db::ChannelThread::create(tx, campfire_db::NewChannelThread { room_id: ALL_TALK, creator_id: DAVID, name: Some("Attachment reply".into()), ..Default::default() })?;
            let trigger = thread.post_message(tx, DAVID, NewMessage { body: Some("Attach".into()), ..Default::default() })?;
            Ok((room, bot, trigger))
        }).await.unwrap();
        let reply = create_attachment_reply(app, &room, &bot, trigger.clone(), webhook::Attachment { data: b"a,b\n1,2\n".to_vec(), filename: "attachment.csv".into(), content_type: "text/csv".into() }).await.unwrap();
        assert_eq!(reply.thread_id, trigger.thread_id);
        assert_eq!(reply.reply_to_message_id, Some(trigger.id));
        let (_, blob) = app.db.read(move |conn| reply.attachment(conn)).await.unwrap().unwrap();
        assert_eq!(blob.filename, "attachment.csv");
        assert_eq!(blob.byte_size, 8);
    }
}
