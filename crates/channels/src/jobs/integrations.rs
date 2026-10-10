//! The jobs integrations perform for the app's job runner: `Room::PushMessageJob`
//! (reference/app/jobs/room/push_message_job.rb) and `Bot::WebhookJob`
//! (reference/app/jobs/bot/webhook_job.rb), including what `Webhook#deliver` does with a reply
//! (create the bot's message, process an attachment, `broadcast_create`).

use anyhow::{Context as _, anyhow};
use campfire_db::{Message, NewMessage, PushSubscription, Room, User, Webhook};
use campfire_db::models::activity_item::message_recorder::MentionPushJob;
use campfire_jobs::{Execution, JobResult, Outcome};
use campfire_views::messages as views;

use crate::integrations::webhook::{self, WebhookReply};
use crate::net::Network;
use crate::app::App;
use crate::controllers::presenters::page::{self, Rendered};
use crate::messaging::{canonicalize_body, process_attachment, save_staged};
use crate::controllers::presenters::Presenter;
use crate::queue::{PushMessageJob, Registry, WebhookJob, discard_missing};

/// Registers `Room::PushMessageJob` and `Bot::WebhookJob`.
pub fn register_jobs(registry: &mut Registry) {
    super::agent_jobs::register(registry);
    crate::integrations::agent_streaming::register(registry);
    registry.register(crate::integrations::link_embed::perform);
    registry.register(crate::integrations::twitter::fetcher::perform);
    registry.register(crate::integrations::fizzy::fetch::perform);
    registry.register(crate::integrations::fizzy::agent_job::perform);
    registry.register(push_message);
    registry.register(deliver_webhook);
    crate::integrations::github::jobs::register(registry);
    crate::integrations::slack::jobs::register(registry);
}

/// `Room::PushMessageJob#perform(room, message)`: `Room::MessagePusher.new(room:, message:).push`,
/// unless Web Push is off. A room or message that's gone discards the job.
async fn push_message(app: App, job: PushMessageJob, execution: Execution) -> JobResult {
    let Some(pool) = app.web_push.clone() else { return Ok(Outcome::Done) };
    let PushMessageJob { room_id, message_id } = job;
    let db = app.db.clone();
    app.db
        .write(move |tx| {
            // Serialize recipient selection and pool handoff with mention edits.
            Room::find(tx.conn(), room_id)?;
            let message = Message::find(tx.conn(), message_id)?;
            let excluded = MentionPushJob::original_push_exclusions(tx.conn(), execution.id)?;
            let (payload, mut subscriptions, mentions) = PushSubscription::pushes_for(
                tx.conn(), &*db.env().rich_text, &message, tx.now(),
            )?;
            subscriptions.extend(mentions);
            subscriptions.retain(|sub| !excluded.contains(&sub.user_id));
            pool.queue(tx.conn(), &payload, subscriptions)
        })
        .await
        .map_err(discard_missing)?;
    Ok(Outcome::Done)
}


/// `Bot::WebhookJob#perform(bot, message)`: `bot.deliver_webhook(message)`, i.e.
/// `webhook.deliver(message)`, then the reply.
async fn deliver_webhook(app: App, job: WebhookJob, _: Execution) -> JobResult {
    deliver_webhook_with_network(&app, job, &Network::system()).await
}

pub async fn deliver_webhook_with_network(app: &App, job: WebhookJob, network: &Network) -> JobResult {
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

    let delivery = webhook::deliver_signed(network, url.as_deref().unwrap_or(""), payload, secret.as_deref(), || app.clock.now(), false).await?;
    let message = match delivery.reply {
        WebhookReply::None => return Ok(Outcome::Done),
        WebhookReply::Text(text) => create_text_reply(app, &room, &bot, delivery.status.map(|_| trigger.clone()), text).await?,
        WebhookReply::Attachment(attachment) => create_attachment_reply(app, &room, &bot, trigger, attachment).await?,
    };
    broadcast_create(app, &room, &message).await?;
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
pub async fn create_attachment_reply(app: &App, room: &Room, bot: &User, trigger: Message, attachment: webhook::Attachment) -> anyhow::Result<Message> {
    let storage = app.storage.clone();
    let staged = tokio::task::spawn_blocking(move || attachment.stage_blob(&storage)).await??;
    let blob = app.db.write(move |tx| save_staged(tx, staged)).await?;

    let (room_id, creator_id, blob_id) = (room.id, bot.id, blob.id);
    let message = app
        .db
        .write(move |tx| {
            let message = create_reply(tx, Some(&trigger), NewMessage { room_id, creator_id, attachment_blob_id: Some(blob_id), ..Default::default() })?;
            if message.thread_id.is_some() {
                if let Some(blob) = campfire_storage::Blob::find(tx.conn(), blob_id).map_err(|e| campfire_db::Error::Other(e.to_string()))? {
                    crate::controllers::presenters::attachments::enqueue_analysis(tx, &blob);
                }
                campfire_db::models::message_attachment_processing::schedule(tx, message.id, blob_id);
            }
            Ok(message)
        })
        .await?;
    if message.thread_id.is_none() {
        process_attachment(app, blob).await.map_err(|e| anyhow!("{e:?}"))?;
    }
    let id = message.id;
    Ok(app.db.read(move |conn| Message::find(conn, id)).await?)
}

/// `Webhook#create_sync_reply`: root replies reference the trigger; thread replies use
/// ChannelThread#post_message! so membership, locking, reopening and counters stay shared.
pub fn create_reply(tx: &mut campfire_db::Tx<'_>, trigger: Option<&Message>, mut attributes: NewMessage) -> campfire_db::Result<Message> {
    attributes.reply_to_message_id = trigger.map(|message| message.id);
    if let Some(thread_id) = trigger.and_then(|message| message.thread_id) {
        campfire_db::ChannelThread::find(tx.conn(), thread_id)?.post_message(tx, attributes.creator_id, attributes)
    } else {
        Message::create(tx, attributes)
    }
}

/// `message.broadcast_create`, rendered without a request (`ApplicationController.renderer`).
pub async fn broadcast_create(app: &App, room: &Room, message: &Message) -> anyhow::Result<()> {
    let (app, room, message) = (app.clone(), room.clone(), message.clone());
    let db = app.db.clone();
    let refreshes = db.read(move |conn| {
        let presenter = Presenter::new(conn, &app, None);
        let view = presenter.message(&message)?;
        let account = campfire_db::Account::first(conn)?;
        let html = page::render_detached(&app, account.as_ref(), |ctx| views::message(ctx, &view));
        let partials = Rendered { message: Some(html), ..Rendered::default() };
        app.broadcasts.message_create(conn, &room, &message, &partials, &*app.db.env().rich_text)?;
        Ok(presenter.take_render_refreshes())
    })
    .await?;
    crate::controllers::presenters::refresh_after_render(&db, refreshes).await;
    Ok(())
}
