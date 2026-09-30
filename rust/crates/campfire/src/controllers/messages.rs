//! `MessagesController` (reference/app/controllers/messages_controller.rb), including the
//! multipart attachment upload the composer's `FileUploader` posts here, plus what
//! `Messages::ByBotsController` reuses: creating a message (with `process_attachment`),
//! delivering webhooks and the broadcasts.

pub mod boosts;
pub mod by_bots;

use askama::Template;
use campfire_db::{Job as _, Message, NewMessage, Room, Timeline};
use campfire_kit::format;
use campfire_kit::{Ctx, Error, Freshness, Param, Result, StatusCode, halt, permit_keys};
use campfire_richtext::Content;
use campfire_storage::{Blob, Staged, Variation};
use campfire_views::messages as views;

use crate::active_storage::{self, keep_after_commit};
use crate::app::{App, AppCtx};
use crate::concerns::{self, Before, before_actions, cast_integer, require_current_user};
use crate::controllers::presenters::attachments::{self, Assignment};
use crate::controllers::presenters::page::{self, Rendered, db_error};
use crate::controllers::presenters::{DbResolver, Presenter, cache_key_with_version, room_kind, storage_error};
use crate::jobs::{WEBHOOK_HOLD, WebhookJob};

// --- Actions ------------------------------------------------------------------------------------

/// `index` (`layout false`): the page before/after a message, or the last page; 204 when empty.
pub async fn index(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let (_, room) = concerns::set_room(c).await?;
    let messages = find_paged_messages(c, &room).await?;
    if messages.is_empty() {
        return Ok(c.head(StatusCode::NO_CONTENT));
    }
    // fresh_when @messages: the records' cache keys, their latest updated_at, and the template.
    let etag = messages.iter().map(|m| cache_key_with_version("messages", m.id, m.updated_at.jiff())).collect::<Vec<_>>().join("/");
    let freshness = Freshness {
        etag: Some(etag),
        last_modified: messages.iter().map(|m| m.updated_at.jiff()).max(),
        template: Some(TEMPLATE_DIGEST_INDEX.into()),
        ..Freshness::default()
    };
    if let Some(not_modified) = c.fresh_when(freshness) {
        return Ok(not_modified);
    }
    c.respond_to(&[&format::HTML])?;
    let views = present(c, move |presenter| presenter.messages(&messages)).await?;
    let response = page::bare(c, StatusCode::OK, &format::HTML, |ctx| views::Index { ctx, messages: &views }.render()).await?;
    let fragments = campfire_views::messages::MessageItem::cached_fragments(&c.app().fragment_cache, &views, &c.url_for(""));
    Ok(response.with_cached_fragments(fragments))
}

/// Stands in for the digest `ETagWithTemplateDigest` adds for `messages/index` (only the ETag's
/// shape has to match the reference).
const TEMPLATE_DIGEST_INDEX: &str = "messages/index";

/// `create`: `set_room` runs inside the action, and a room that's gone renders `room_not_found`.
pub async fn create(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let room = match concerns::set_room(c).await {
        Ok((_, room)) => room,
        Err(Error::NotFound) => return render_room_not_found(c).await,
        Err(error) => return Err(error),
    };
    let attributes = message_params(c)?;
    let message = create_message(c, &room, attributes).await?;
    broadcast_create(c, &room, &message).await?;
    release_webhooks(c, &message).await;

    // The message partial comes out of the fragment cache `broadcast_create` just filled
    // (`cache [ message, "presentation-v3" ]`), so it's the request-less rendering: no CSRF
    // tokens in its forms.
    c.respond_to(&[&format::TURBO_STREAM])?;
    let kind = room_kind(room.room_type);
    let app = c.app().clone();
    let base_url = c.url_for("");
    let html = c
        .app()
        .db
        .read(move |conn| {
            let presenter = Presenter::new(conn, &app, None);
            let item = campfire_views::fragment_cache::with(&app.fragment_cache, || presenter.message_item(&message))?;
            let account = campfire_db::Account::first(conn)?;
            page::render_detached_at(&app, account.as_ref(), &base_url, |ctx| views::CreateStream { ctx, message: &item, room_kind: kind }.render())
                .map_err(|e| campfire_db::Error::Other(e.to_string()))
        })
        .await
        .map_err(db_error)?;
    Ok(c.render(StatusCode::OK, &format::TURBO_STREAM, html))
}

pub async fn show(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let (_, room) = concerns::set_room(c).await?;
    let message = set_message(c, &room).await?;
    c.respond_to(&[&format::HTML])?;
    let view = present(c, move |presenter| presenter.message(&message)).await?;
    page::content_in_application_layout(c, StatusCode::OK, |ctx| views::Show { ctx, message: &view }.render()).await
}

pub async fn edit(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let (_, room) = concerns::set_room(c).await?;
    let message = set_message(c, &room).await?;
    ensure_can_administer(c, &message)?;
    c.respond_to(&[&format::HTML])?;
    let edit = present(c, move |presenter| {
        Ok(views::EditView { editable_body_html: presenter.editable_body(&message)?, message: presenter.message(&message)? })
    })
    .await?;
    page::content_in_application_layout(c, StatusCode::OK, |ctx| views::Edit { ctx, edit: &edit }.render()).await
}

pub async fn update(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let (_, room) = concerns::set_room(c).await?;
    let message = set_message(c, &room).await?;
    ensure_can_administer(c, &message)?;
    let attributes = message_params(c)?;
    let message = update_message(c, message, attributes).await?;
    broadcast_replace(c, &room, &message).await?;

    // respond_to html: redirect; json: `render :show`, which has no JSON template here.
    match c.respond_to(&[&format::HTML, &format::JSON])? {
        f if *f == format::JSON => Err(Error::internal(anyhow::anyhow!("Missing template messages/show"))),
        _ => {
            let url = c.url_for(&campfire_routes::room_message(room.id, message.id));
            c.redirect_to(&url)
        }
    }
}

pub async fn destroy(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let (_, room) = concerns::set_room(c).await?;
    let message = set_message(c, &room).await?;
    ensure_can_administer(c, &message)?;
    destroy_message(c, &room, &message).await?;

    c.respond_to(&[&format::TURBO_STREAM])?;
    let view = present(c, move |presenter| presenter.message(&message)).await?;
    page::bare(c, StatusCode::OK, &format::TURBO_STREAM, |_| views::DestroyStream { message: &view }.render()).await
}

// --- Before-actions and params --------------------------------------------------------------------

/// `@room.messages.find(params[:id])`
pub(crate) async fn set_message(c: &mut Ctx, room: &Room) -> Result<Message> {
    let Some(id) = c.param_str("id").and_then(cast_integer) else { return Err(Error::NotFound) };
    let room_id = room.id;
    c.app().db.read(move |conn| Message::find_in_room(conn, room_id, id)).await.map_err(db_error)
}

/// `head :forbidden unless Current.user.can_administer?(@message)`
pub(crate) fn ensure_can_administer(c: &mut Ctx, message: &Message) -> Result<()> {
    if !require_current_user(c)?.can_administer(Some(message.creator_id), false) {
        return halt(concerns::head(StatusCode::FORBIDDEN));
    }
    Ok(())
}

/// What `create_with_attachment!`/`update!` receive.
#[derive(Debug, Default, Clone)]
pub(crate) struct MessageParams {
    pub clear_markdown_source: bool,
    pub client_message_lookup: campfire_db::models::agent_posting::client_ids::Lookup,
    pub body: Option<String>,
    pub markdown_source: Option<String>,
    /// `attachment=`: `None` when the key wasn't given.
    pub attachment: Option<Assignment>,
    pub client_message_id: Option<String>,
}

/// `params.require(:message).permit(:body, :attachment, :client_message_id)`
fn message_params(c: &Ctx) -> Result<MessageParams> {
    let message = c.params.require("message")?;
    let permitted = message.permit(&permit_keys(&["body", "attachment", "client_message_id", "markdown_source"]));
    let text = |key: &str| permitted.get(key).and_then(Param::as_str).map(str::to_string);
    Ok(MessageParams {
        body: text("body"),
        markdown_source: text("markdown_source"),
        attachment: attachment_assignment(&permitted)?,
        client_message_id: text("client_message_id"),
        clear_markdown_source: false,
        client_message_lookup: Default::default(),
    })
}

/// What assigning the permitted `attachment` does: an upload replaces the attachment, nil or ""
/// removes it (`Attached::Changes::DeleteOne`), anything else raises.
pub(crate) fn attachment_assignment(permitted: &campfire_kit::ParamMap) -> Result<Option<Assignment>> {
    match Assignment::from_params(permitted, "attachment")? {
        Assignment::Unchanged => Ok(None),
        assignment => Ok(Some(assignment)),
    }
}

/// `@room.root_messages.find(params[:before])` and friends (`find_paged_messages`).
pub(crate) async fn find_paged_messages(c: &Ctx, room: &Room) -> Result<Vec<Message>> {
    let present = |key: &str| c.params.get(key).filter(|p| p.is_present()).map(|p| p.as_str().and_then(cast_integer));
    let (before, after) = (present("before"), present("after"));
    let room_id = room.id;
    c.app()
        .db
        .read(move |conn| match (before, after) {
            (Some(before), _) => {
                let message = Message::find_in(conn, Timeline::Room(room_id), before.ok_or(campfire_db::Error::RecordNotFound("Message"))?)?;
                Message::page_before(conn, Timeline::Room(room_id), &message)
            }
            (None, Some(after)) => {
                let message = Message::find_in(conn, Timeline::Room(room_id), after.ok_or(campfire_db::Error::RecordNotFound("Message"))?)?;
                Message::page_after(conn, Timeline::Room(room_id), &message)
            }
            (None, None) => Message::last_page(conn, Timeline::Room(room_id)),
        })
        .await
        .map_err(db_error)
}

// --- Creating, updating, destroying ---------------------------------------------------------------

/// `@room.messages.create_with_attachment!(attributes)`: the message (with its uploaded blob, in
/// one transaction), then `process_attachment`. The upload's file is copied into storage and the
/// body canonicalized before the transaction, so the writer only inserts rows.
/// `@room.messages.create!` and, in its transaction, `deliver_webhooks_to_bots`: the webhook
/// jobs are held until the caller has broadcast the message ([`release_webhooks`]).
pub(crate) async fn create_message(c: &Ctx, room: &Room, attributes: MessageParams) -> Result<Message> {
    match create_message_with_agent_policy(c, room, attributes, false).await? {
        campfire_db::models::agent_posting::PostingOutcome::Created(message) => Ok(message),
        _ => unreachable!("human posting does not run agent policy"),
    }
}

pub(crate) async fn create_message_with_agent_policy(c: &Ctx, room: &Room, mut attributes: MessageParams, agent_policy: bool) -> Result<campfire_db::models::agent_posting::PostingOutcome> {
    use campfire_db::models::agent_posting::{PostingOutcome, PostingCheck};
    let creator_id = require_current_user(c)?.id;
    let room_id = room.id;
    let room = room.clone();
    let attachment = attributes.attachment.unwrap_or(Assignment::Unchanged).stage(c.app()).await?;
    if matches!(attachment, Assignment::Invalid) {
        return Err(invalid_attachment());
    }
    let body = match attributes.body {
        Some(body) => Some(canonicalize_body(c.app(), body, Some(c.request.host())).await?),
        None => None,
    };
    let (message, blob) = c
        .app()
        .db
        .write(move |tx| {
            if agent_policy {
                match campfire_db::models::agent_posting::prepare_for_user_with_lookup(tx, creator_id, room_id, attributes.client_message_id.as_deref(), &attributes.client_message_lookup)? {
                    Some(PostingCheck::Replay(message)) => return Ok((PostingOutcome::Replay(*message), None)),
                    Some(PostingCheck::Budget(payload)) => return Ok((PostingOutcome::Budget(payload), None)),
                    Some(PostingCheck::Allowed) => {},
                    None => attributes.client_message_id = None,
                }
            }
            let blob = attachment_blob(tx, attachment)?;

            let message = Message::create(
                tx,
                NewMessage {
                    room_id,
                    creator_id,
                    client_message_id: attributes.client_message_id,
                    body,
                    markdown_source: attributes.markdown_source,
                    attachment_blob_id: blob.as_ref().map(|blob| blob.id),
                    ..Default::default()
                },
            )?;
            deliver_webhooks_to_bots(tx, &room, &message)?;
            if let Some(blob) = &blob {
                attachments::enqueue_analysis(tx, blob);
            }
            Ok((PostingOutcome::Created(message), blob))

        })
        .await
        .map_err(db_error)?;
    if let Some(blob) = blob {
        process_attachment(c.app(), blob).await?;
    }
    match message {
        PostingOutcome::Created(message) => {
            let id = message.id;
            let message = c.app().db.read(move |conn| Message::find(conn, id)).await.map_err(db_error)?;
            Ok(PostingOutcome::Created(message))
        }
        outcome => Ok(outcome),
    }
}

/// Inserts a staged blob's row, keeping its file once the transaction commits.
pub(crate) fn save_staged(tx: &mut campfire_db::Tx<'_>, staged: Staged) -> campfire_db::Result<Blob> {
    let blob = staged.insert(tx.conn(), tx.now().jiff()).map_err(storage_error)?;
    keep_after_commit(tx, staged);
    Ok(blob)
}

/// Resolve a staged upload or an existing direct-upload blob inside the writer transaction.
fn attachment_blob(tx: &mut campfire_db::Tx<'_>, assignment: Assignment<Staged>) -> campfire_db::Result<Option<Blob>> {
    match assignment {
        Assignment::Create(staged) => save_staged(tx, staged).map(Some),
        Assignment::Existing(blob) => attachments::save_existing(tx, blob).map(Some),
        Assignment::Unchanged | Assignment::Delete => Ok(None),
        Assignment::Signed(_) | Assignment::Invalid => Err(campfire_db::Error::Other("invalid attachment".into())),
    }
}

/// [`canonical_body`] on a reader, ahead of the write that stores it.
pub(crate) async fn canonicalize_body(app: &App, body: String, request_host: Option<String>) -> Result<String> {
    let app2 = app.clone();
    app.db.read(move |conn| Ok(canonical_body(conn, &app2, &body, request_host))).await.map_err(db_error)
}

/// Assigning a String to a rich text attribute stores the canonicalized content
/// (`ActionText::Content.new(body, canonicalize: true).to_html`).
pub(crate) fn canonical_body(conn: &campfire_db::Connection, app: &App, body: &str, request_host: Option<String>) -> String {
    let resolver = DbResolver::new(conn, &app.secrets, app.clock.now());
    let ctx = resolver.render_context(request_host);
    Content::load(body, &ctx).map(|content| content.to_html()).unwrap_or_else(|_| body.to_string())
}

/// Assigning something that isn't an upload, a signed blob id, nil or "".
fn invalid_attachment() -> Error {
    Error::internal(anyhow::anyhow!("Could not find or build blob: expected attachable"))
}

/// `Message#process_attachment`: analyze the blob now (its `after_update` touches the message),
/// then generate the video preview or the `:thumb` representation.
pub(crate) async fn process_attachment(app: &App, blob: Blob) -> Result<()> {
    let blob = analyze_attachment(app, blob).await?;
    if blob.is_video() {
        // attachment.preview(format: :webp).processed
        active_storage::processed_preview(app, blob, Variation::format_only("webp")).await?;
    } else if blob.is_representable() {
        // attachment.representation(:thumb).processed
        let thumb = Variation::resize_to_limit(1200, 800, None);
        active_storage::processed_representation(app, blob, thumb).await?;
    }
    Ok(())
}

/// `blob.analyze`: its `after_update` touches the attached records. The file is analyzed off the
/// writer.
async fn analyze_attachment(app: &App, blob: Blob) -> Result<Blob> {
    active_storage::analyze(app, blob.id).await.map_err(Error::internal)?.ok_or(Error::NotFound)
}

/// `@message.update!(message_params)`. A new attachment replaces the old one (whose blob is purged
/// later) without `process_attachment`: the blob is only analyzed, by `ActiveStorage::AnalyzeJob`
/// after commit (verified against the reference with a bot's `PUT` and `attachment`).
pub(crate) async fn update_message(c: &Ctx, message: Message, attributes: MessageParams) -> Result<Message> {
    let attachment_given = attributes.attachment.is_some();
    let attachment = attributes.attachment.unwrap_or(Assignment::Unchanged).stage(c.app()).await?;
    if matches!(attachment, Assignment::Invalid) {
        return Err(invalid_attachment());
    }
    let body = match attributes.body {
        Some(body) => Some(canonicalize_body(c.app(), body, Some(c.request.host())).await?),
        None => None,
    };
    let id = c
        .app()
        .db
        .write(move |tx| {
            let mut message = message;
            let blob = attachment_blob(tx, attachment)?;
            if attachment_given {
                message.replace_attachment(tx, blob.as_ref().map(|blob| blob.id))?;
            }
            message.edit(tx, campfire_db::MessageChanges {
                markdown_source: attributes.markdown_source,
                clear_markdown_source: attributes.clear_markdown_source,
                body,
                ..Default::default()
            })?;
            if let Some(blob) = &blob {
                attachments::enqueue_analysis(tx, blob);
            }
            Ok(message.id)
        })
        .await
        .map_err(db_error)?;
    c.app().db.read(move |conn| Message::find(conn, id)).await.map_err(db_error)
}


/// `@message.destroy` then `@message.broadcast_remove`.
pub(crate) async fn destroy_message(c: &Ctx, room: &Room, message: &Message) -> Result<()> {
    let destroyed = message.clone();
    c.app().db.write(move |tx| destroyed.destroy(tx)).await.map_err(db_error)?;
    c.app().broadcasts.message_remove(room, message);
    Ok(())
}

// --- Broadcasts and webhooks -----------------------------------------------------------------------

/// `@message.broadcast_create`: the message partial appended to the room, then the unread pings.
pub(crate) async fn broadcast_create(c: &Ctx, room: &Room, message: &Message) -> Result<()> {
    let (app, room, message) = (c.app().clone(), room.clone(), message.clone());
    let base_url = page::renderer_base_url(c);
    let refreshes = c.app()
        .db
        .read(move |conn| {
            let presenter = Presenter::new(conn, &app, None);
            let view = presenter.message(&message)?;
            let account = campfire_db::Account::first(conn)?;
            let html = page::render_detached_at(&app, account.as_ref(), &base_url, |ctx| views::message(ctx, &view));
            let partials = Rendered { message: Some(html), ..Rendered::default() };
            app.broadcasts.message_create(conn, &room, &message, &partials, &*app.db.env().rich_text)?;
            Ok(presenter.take_github_refreshes())
        })
        .await
        .map_err(db_error)?;
    crate::integrations::github::pull_requests::refresh_after_render(&c.app().db, refreshes).await;
    Ok(())
}

/// `broadcast_replace_to @room, :messages, target: [ @message, :presentation ], partial:
/// "messages/presentation", attributes: { maintain_scroll: true }`
pub(crate) async fn broadcast_replace(c: &Ctx, room: &Room, message: &Message) -> Result<()> {
    let (app, room, message) = (c.app().clone(), room.clone(), message.clone());
    let base_url = page::renderer_base_url(c);
    let refreshes = c.app()
        .db
        .read(move |conn| {
            let presenter = Presenter::new(conn, &app, None);
            let view = presenter.message(&message)?;
            let account = campfire_db::Account::first(conn)?;
            let html = page::render_detached_at(&app, account.as_ref(), &base_url, |ctx| {
                views::PresentationPartial { ctx, message: &view }.render()
            })
            .map_err(|e| campfire_db::Error::Other(e.to_string()))?;
            let partials = Rendered { message_presentation: Some(html), ..Rendered::default() };
            app.broadcasts.message_replace(&room, &message, &partials);
            let replacements = page::render_detached_at(&app, account.as_ref(), &base_url, |ctx| -> askama::Result<_> {
                Ok([
                    ("meta", views::MetaPartial {ctx, message: &view}.render()?),
                    // Empty containers remove their old cards after an edit.
                    ("github_pr_cards", view.components.github_cards_html.clone().unwrap_or_else(|| views::cards(&view, "github_pr_cards", "github-pr-cards", 0, &view.components.github_cards).0)),
                    ("twitter_cards", campfire_views::twitter::cards(ctx, &view).0),
                    ("message_link_cards", views::cards(&view, "message_link_cards", "message-link-cards", 0, &view.components.message_link_cards).0),
                    ("fizzy_cards", views::cards(&view, "fizzy_cards", "fizzy-cards", 0, &view.components.fizzy_cards).0),
                    ("linkedin_cards", views::cards(&view, "linkedin_cards", "linkedin-post-cards", 2, &view.components.linkedin_cards).0),
                    ("link_embed_cards", views::cards(&view, "link_embed_cards", "link-embed-cards", 2, &view.components.link_embed_cards).0),
                ])
            }).map_err(|e| campfire_db::Error::Other(e.to_string()))?;
            for (part, html) in replacements {
                app.broadcasts.message_part_replace(&room, &message, part, &html);
            }
            Ok(presenter.take_github_refreshes())
        })
        .await
        .map_err(db_error)?;
    crate::integrations::github::pull_requests::refresh_after_render(&c.app().db, refreshes).await;
    Ok(())
}

/// `deliver_webhooks_to_bots`, in the message's transaction: every active bot in a direct room,
/// else every mentioned active bot, except the message's creator, gets `bot.deliver_webhook_later
/// (@message)` when it has a webhook. Rails enqueues them after the message is saved, processed
/// and broadcast, in a separate step that a crash (or a failed enqueue) can skip. Here the job
/// rows commit, or roll back, with the message; each is held for [`WEBHOOK_HOLD`] so a bot can't
/// be told (and reply) before the room sees the message, and [`release_webhooks`] makes them due
/// once it has been broadcast.
pub(crate) fn deliver_webhooks_to_bots(tx: &mut campfire_db::Tx<'_>, room: &Room, message: &Message) -> campfire_db::Result<()> {
    let _ = room;
    for bot in campfire_db::models::bot_webhook_fanout::recipients(tx, message)? {
        if bot.webhook(tx.conn())?.is_some() {
            tx.emit_after_commit(campfire_db::Event::job_in(WEBHOOK_HOLD, &WebhookJob { bot_id: bot.id, message_id: message.id }));
        }
    }
    Ok(())
}

/// Releases the webhooks [`create_message`] held, now that the message has been broadcast. A
/// failure is logged: they're delivered when the hold runs out.
pub(crate) async fn release_webhooks(c: &Ctx, message: &Message) {
    let (queue, message_id) = (c.app().jobs.queue.clone(), message.id);
    let released = c.app().db.write(move |tx| queue.release_held(tx, WebhookJob::CLASS, "message_id", message_id)).await;
    match released {
        Ok(0) => {}
        Ok(_) => c.app().jobs.queue.wake(WebhookJob::CLASS),
        Err(error) => tracing::error!(%error, message_id, "releasing the message's webhooks failed; they're delivered in {WEBHOOK_HOLD:?}"),
    }
}

// --- Rendering ------------------------------------------------------------------------------------

/// Runs `f` with a presenter on a reader connection.
pub(crate) async fn present<T: Send + 'static>(
    c: &Ctx,
    f: impl FnOnce(&Presenter) -> campfire_db::Result<T> + Send + 'static,
) -> Result<T> {
    let app = c.app().clone();
    let request_host = Some(c.request.host());
    let cache_base_url = c.url_for("");
    let (value, fetches, twitter_fetches, refreshes) = c.app()
        .db
        .read(move |conn| {
            let mut presenter = Presenter::new(conn, &app, request_host);
            presenter.cache_base_url = Some(cache_base_url);
            // The Jbuilder partials (`json.cache!`) read the fragment cache on this thread.
            let value = campfire_views::fragment_cache::with(&app.fragment_cache, || f(&presenter))?;
            Ok((value, presenter.pending_link_fetches(), presenter.pending_twitter_fetches(), presenter.take_github_refreshes()))
        })
        .await
        .map_err(db_error)?;
    super::presenters::link_embeds::enqueue_render_fetches(c.app(), fetches, twitter_fetches)
        .await
        .map_err(db_error)?;
    crate::integrations::github::pull_requests::refresh_after_render(&c.app().db, refreshes).await;
    Ok(value)
}

/// `render action: :room_not_found` (inside the layout).
async fn render_room_not_found(c: &mut Ctx) -> Result {
    c.respond_to(&[&format::HTML])?;
    page::content_in_application_layout(c, StatusCode::OK, |_| views::RoomNotFound.render()).await
}

#[cfg(test)]
mod tests;

#[cfg(test)]
#[path = "messages/ws17_activity_tests.rs"]
mod ws17_activity_tests;
