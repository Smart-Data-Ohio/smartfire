//! `MessagesController` (reference/app/controllers/messages_controller.rb), including the
//! multipart attachment upload the composer's `FileUploader` posts here, plus what
//! `Messages::ByBotsController` reuses: creating a message (with `process_attachment`),
//! delivering webhooks and the broadcasts.

pub mod boosts;
#[cfg(test)]
pub(crate) mod boosts_tests;
#[cfg(test)]
mod upload_tests;
#[cfg(test)]
mod review_tests;
pub mod by_bots;
pub(crate) mod payload;
mod freshness;
pub(crate) mod rendered;
#[cfg(test)]
mod root_tests;
#[cfg(test)]
mod paging_tests;
#[cfg(test)]
mod collection_tests;
#[cfg(test)]
mod csrf_tests;
#[cfg(test)]
mod room_list_tests;
#[cfg(test)]
mod github_integration_tests;
#[cfg(test)]
pub(crate) mod provider_tests;
#[cfg(test)]
pub(crate) mod drive_tests;
#[cfg(test)]
pub(crate) mod state_tests;

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
use crate::controllers::presenters::{DbResolver, Presenter, room_kind, storage_error};
use crate::jobs::{WEBHOOK_HOLD, WebhookJob};

// --- Actions ------------------------------------------------------------------------------------

/// `index` (`layout false`): the page before/after a message, or the last page; 204 when empty.
pub async fn index(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let room = set_root_room(c).await?;
    let messages = find_paged_messages(c, &room).await?;
    let json = c.format()? == Some(&format::JSON);
    if json { c.no_store(); }
    if messages.is_empty() {
        if !json { c.expires_now(); }
        return Ok(c.head(StatusCode::NO_CONTENT));
    }
    let records = messages.clone();
    let etag = c.app().db.read(move |conn| freshness::etag(conn, &records)).await.map_err(db_error)?;
    let freshness = Freshness {
        etag: Some(etag),
        template: Some(freshness::INDEX_TEMPLATE_DIGEST.trim().into()),
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

/// `create`: `set_room` runs inside the action, and a room that's gone renders `room_not_found`.
pub async fn create(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    match create_action(c).await {
        Err(Error::NotFound) => render_room_not_found(c).await,
        Err(error) => render_record_invalid(c, error),
        response => response,
    }
}

async fn create_action(c: &mut Ctx) -> Result {
    let room = set_root_room(c).await?;
    if c.params.get("message").is_some_and(|value| !matches!(value, Param::Null | Param::Hash(_))) {
        return Err(Error::internal(anyhow::anyhow!("message parameters do not support dig")));
    }
    // Rails checks retries before validating or staging the new payload.
    let raw_client_id = c.params.get("message").and_then(|message| message.get("client_message_id"));
    let client_id = raw_client_id.and_then(string_column);
    let lookup_id = raw_client_id.filter(|value| value.is_present()).and(client_id.clone());
    let (room_id, creator_id) = (room.id, require_current_user(c)?.id);
    let duplicate = match lookup_id {
        Some(id) => c.app().db.read(move |conn| Message::find_duplicate(conn, room_id, creator_id, &id)).await.map_err(db_error)?,
        None => None,
    };
    let message = if let Some(duplicate) = duplicate {
        duplicate
    } else {
        let attributes = human_message_params_with_client_id(c, Some(&room), client_id).await?;
        let message = create_message(c, &room, attributes).await?;
        broadcast_create(c, &room, &message).await?;
        release_webhooks(c, &message).await;
        message
    };

    // Rails renders the individual message without collection caching, in a request-less
    // context: no CSRF tokens in its forms.
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
    let room = set_root_room(c).await?;
    let message = set_message(c, &room).await?;
    c.respond_to(&[&format::HTML])?;
    let view = present(c, move |presenter| presenter.message(&message)).await?;
    page::content_in_application_layout(c, StatusCode::OK, |ctx| views::Show { ctx, message: &view }.render()).await
}

pub async fn edit(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let room = set_root_room(c).await?;
    let message = set_message(c, &room).await?;
    ensure_can_edit(c, &message)?;
    c.respond_to(&[&format::HTML])?;
    let edit = present(c, move |presenter| {
        Ok(views::EditView { editable_body_html: presenter.editable_markdown_source(&message)?, message: presenter.message(&message)? })
    })
    .await?;
    page::content_in_application_layout(c, StatusCode::OK, |ctx| views::Edit { ctx, edit: &edit }.render()).await
}

pub async fn update(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let room = set_root_room(c).await?;
    let message = set_message(c, &room).await?;
    ensure_can_edit(c, &message)?;
    let message = match update_root_message(c, &room, message).await {
        Ok(message) => message,
        Err(error) => return render_record_invalid(c, error),
    };
    let drive_given = c.params.get("message").and_then(|params| params.get("drive_file_ids")).is_some();
    rendered::broadcast_edit(c, &room, &message, drive_given).await?;

    match c.respond_to(&[&format::HTML, &format::JSON])? {
        f if *f == format::JSON => {
            let viewer = require_current_user(c)?.clone();
            let base = c.url_for("");
            let data = present(c, move |presenter| payload::message(presenter, &message, &viewer, &base)).await?;
            // This human `render json:` response leaves HTML entities raw (root Rails oracle).
            Ok(c.render(StatusCode::OK, &format::JSON, serde_json::to_string(&data).map_err(Error::internal)?))
        }
        _ => {
            let url = c.url_for(&campfire_routes::room_message(room.id, message.id));
            c.redirect_to(&url)
        }
    }
}

/// The actions menu reads viewer capabilities and saved/reaction state on every request.
pub async fn actions(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let room = set_root_room(c).await?;
    let message = set_message(c, &room).await?;
    let viewer = require_current_user(c)?.clone();
    let base = c.url_for("");
    c.set_header("cache-control", "no-store");
    let data = present(c, move |presenter| payload::actions(presenter, &message, &viewer, &base)).await?;
    Ok(c.render(StatusCode::OK, &format::JSON, serde_json::to_string(&serde_json::json!({"actions": data})).map_err(Error::internal)?))
}

pub async fn destroy(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let room = set_root_room(c).await?;
    let message = set_message(c, &room).await?;
    ensure_can_delete(c, &message)?;
    destroy_message(c, &room, &message).await?;

    c.respond_to(&[&format::TURBO_STREAM])?;
    let view = present(c, move |presenter| presenter.message(&message)).await?;
    page::bare(c, StatusCode::OK, &format::TURBO_STREAM, |_| views::DestroyStream { message: &view }.render()).await
}

// --- Before-actions and params --------------------------------------------------------------------

/// `RoomScoped`: a membership in an alive room, including during deferred deletion.
async fn set_root_room(c: &mut Ctx) -> Result<Room> {
    let (_, room) = concerns::set_room(c).await?;
    if room.deleted_at.is_some() {
        return Err(Error::NotFound);
    }
    Ok(room)
}

/// Preview renders the normal Markdown presentation without persisting any rows.
pub async fn preview(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let room = set_root_room(c).await?;
    let message = c.params.require("message")?;
    let permitted = message.permit(&permit_keys(&["markdown_source"]));
    let source = permitted.get("markdown_source").ok_or_else(|| Error::ParameterMissing("markdown_source".into()))?;
    let source = source.as_str().ok_or_else(|| Error::internal(anyhow::anyhow!("markdown_source has no length")))?.to_owned();
    if source.chars().count() > campfire_db::message::SOURCE_LIMIT {
        let body = campfire_views::helpers::to_rails_json(&serde_json::json!({"error": "Markdown is limited to 50,000 characters"}));
        return Ok(c.render(StatusCode::UNPROCESSABLE_ENTITY, &format::JSON, body));
    }
    let (app, request_host) = (c.app().clone(), Some(c.request.host()));
    let html = c.app().db.read(move |conn| {
        let body = app.db.env().rich_text.render_markdown(conn, &source, room.id).map_err(campfire_db::Error::Other)?;
        let resolver = DbResolver::new(conn, &app.secrets, app.clock.now());
        crate::rich_text::markdown_presentation(conn, &body, &resolver.render_context(request_host)).map_err(campfire_db::Error::Other)
    }).await.map_err(db_error)?;
    let body = serde_json::to_string(&serde_json::json!({"html": html})).map_err(Error::internal)?;
    Ok(c.render(StatusCode::OK, &format::JSON, body))
}

pub(crate) fn ensure_can_edit(c: &mut Ctx, message: &Message) -> Result<()> {
    if message.system_note || require_current_user(c)?.id != message.creator_id {
        return halt(concerns::head(StatusCode::FORBIDDEN));
    }
    Ok(())
}

pub(crate) fn ensure_can_delete(c: &mut Ctx, message: &Message) -> Result<()> {
    let user = require_current_user(c)?;
    if message.system_note || (user.id != message.creator_id && !user.is_administrator()) {
        return halt(concerns::head(StatusCode::FORBIDDEN));
    }
    Ok(())
}

/// `@room.messages.find(params[:id])`
pub(crate) async fn set_message(c: &mut Ctx, room: &Room) -> Result<Message> {
    let Some(id) = c.param_str("id").and_then(cast_integer) else { return Err(Error::NotFound) };
    let room_id = room.id;
    c.app().db.read(move |conn| Message::find_in(conn, Timeline::Room(room_id), id)).await.map_err(db_error)
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
    pub reply_to_message_id: Option<i64>,
    pub reply_notify_author: Option<bool>,
    pub drive_file_ids: Vec<String>,
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
        ..Default::default()
    })
}

/// Threads let the model validate their reply target; roots first scope it to root_messages.
pub(crate) async fn human_message_params(c: &Ctx, root_room: Option<&Room>) -> Result<MessageParams> {
    let client_id = c.params.get("message").and_then(|message| message.get("client_message_id")).and_then(string_column);
    human_message_params_with_client_id(c, root_room, client_id).await
}

/// Create callers cast once before the retry lookup and pass that same column value here.
/// Raw `blank?` still controls lookup: false persists as "f" but never deduplicates.
pub(crate) async fn human_message_params_with_client_id(c: &Ctx, root_room: Option<&Room>, client_id: Option<String>) -> Result<MessageParams> {
    let message = c.params.require("message")?;
    if message.as_hash().is_none() {
        return Err(Error::internal(anyhow::anyhow!("message parameters do not support permit")));
    }
    let mut attributes = message_params(c)?;
    let permitted = message.permit(&permit_keys(&["markdown_source", "client_message_id", "reply_to_message_id", "reply_notify_author"]));
    attributes.markdown_source = permitted.get("markdown_source").and_then(string_column);
    attributes.client_message_id = client_id;
    if attributes.markdown_source.is_some() {
        attributes.body = None;
    }
    if let Some(value) = permitted.get("reply_to_message_id").filter(|value| value.is_present()) {
        attributes.reply_to_message_id = if let Some(room) = root_room {
            let id = value.to_s().as_deref().and_then(cast_integer).ok_or(Error::NotFound)?;
            let room_id = room.id;
            Some(c.app().db.read(move |conn| Message::find_in(conn, Timeline::Room(room_id), id)).await.map_err(db_error)?.id)
        } else { Some(value.to_s().as_deref().and_then(cast_integer).unwrap_or(0)) };
    }
    if let Some(value) = permitted.get("reply_notify_author") {
        if value.is_null() || value.as_str() == Some("") {
            return Err(Error::internal(anyhow::anyhow!("reply_notify_author violates NOT NULL")));
        }
        attributes.reply_notify_author = Some(!matches!(value.to_s().as_deref(), Some("false" | "FALSE" | "f" | "F" | "0" | "off" | "OFF")));
    }
    if let Some(raw) = message.get("drive_file_ids") {
        let Param::Array(ids) = raw else { return Err(invalid_drive_file_ids()) };
        for id in ids {
            let Some(id) = id.to_s() else { return Err(invalid_drive_file_ids()) };
            // Ruby String#strip removes ASCII whitespace and NUL, not Unicode spaces.
            let id = id.trim_matches([' ', '\t', '\r', '\n', '\x0b', '\x0c', '\0']);
            if id.chars().all(char::is_whitespace) {
                continue;
            }
            if !campfire_db::message::valid_drive_file_id(id) {
                return Err(invalid_drive_file_ids());
            }
            if !attributes.drive_file_ids.iter().any(|stored| stored == id) {
                attributes.drive_file_ids.push(id.to_owned());
            }
        }
    }
    Ok(attributes)
}

/// `assign_attributes` + `save!` on the human edit endpoint. Bot updates keep their own seam.
async fn update_root_message(c: &Ctx, room: &Room, message: Message) -> Result<Message> {
    update_human_message(c, Some(room), None, message).await
}

pub(crate) async fn update_human_message(c: &Ctx, root_room: Option<&Room>, thread_id: Option<i64>, message: Message) -> Result<Message> {
    let attributes = human_message_params(c, root_room).await?;
    let params = c.params.require("message")?;
    let scalar = params.permit(&permit_keys(&["client_message_id", "reply_to_message_id", "reply_notify_author"]));
    let attachment_given = attributes.attachment.is_some();
    let attachment = attributes.attachment.unwrap_or(Assignment::Unchanged).stage(c.app()).await?;
    if matches!(attachment, Assignment::Invalid) { return Err(invalid_attachment()); }
    let changes = campfire_db::MessageChanges {
        clear_markdown_source: attributes.markdown_source.is_none(),
        markdown_source: attributes.markdown_source,
        body: attributes.body,
        client_message_id: scalar.get("client_message_id").map(string_column),
        reply_to_message_id: scalar.get("reply_to_message_id").map(|_| attributes.reply_to_message_id),
        reply_notify_author: attributes.reply_notify_author,
        drive_file_ids: params.get("drive_file_ids").map(|_| attributes.drive_file_ids),
        ..Default::default()
    };
    let preserve = !message.markdown() && changes.markdown_source.as_ref().is_some_and(|source| !source.chars().all(char::is_whitespace));
    let id = message.id;
    let app = c.app().clone();
    let host = Some(c.request.host());
    let id = c.app().db.write(move |tx| {
        if let Some(thread) = thread_id
            && campfire_db::ChannelThread::find(tx.conn(), thread)?.locked_at.is_some() {
            return Err(campfire_db::Error::Other(campfire_db::channel_thread::LOCKED_MESSAGE.into()));
        }
        let mut message = message;
        let mut changes = changes;
        if preserve {
            let body = Presenter::new(tx.conn(), &app, host).rendered_body_html(&message)?;
            changes.legacy_attachment_snapshot = Some(campfire_richtext::legacy_markdown::non_mention_attachments(&body)
                .map_err(|error| campfire_db::Error::Other(error.to_string()))?);
        }
        let blob = attachment_blob(tx, attachment)?;
        if attachment_given { message.replace_attachment(tx, blob.as_ref().map(|blob| blob.id))?; }
        message.edit(tx, changes)?;
        if let Some(blob) = &blob { attachments::enqueue_analysis(tx, blob); }
        Ok(id)
    }).await.map_err(db_error)?;
    c.app().db.read(move |conn| Message::find(conn, id)).await.map_err(db_error)
}

fn invalid_drive_file_ids() -> Error {
    let mut errors = campfire_db::Errors::default();
    errors.add("drive_attachments", "includes an invalid file id");
    db_error(campfire_db::Error::RecordInvalid(errors))
}

/// Active Record string/text column assignment (verified by the Rails model probes).
pub(crate) fn string_column(value: &Param) -> Option<String> {
    match value {
        Param::Null => None,
        Param::Bool(true) => Some("t".into()),
        Param::Bool(false) => Some("f".into()),
        value => value.to_s(),
    }
}

fn render_record_invalid(c: &mut Ctx, error: Error) -> Result {
    let Error::Internal(error) = error else { return Err(error) };
    let Some(campfire_db::Error::RecordInvalid(errors)) = error.downcast_ref::<campfire_db::Error>() else {
        return Err(Error::Internal(error));
    };
    if c.format()? == Some(&format::JSON) {
        let mut by_attribute = serde_json::Map::new();
        for (attribute, message) in &errors.0 {
            by_attribute.entry((*attribute).to_owned()).or_insert_with(|| serde_json::json!([])).as_array_mut().unwrap().push(message.clone().into());
        }
        let body = campfire_views::helpers::to_rails_json(&serde_json::json!({"errors": by_attribute}));
        Ok(c.render(StatusCode::UNPROCESSABLE_ENTITY, &format::JSON, body))
    } else {
        Ok(c.head(StatusCode::UNPROCESSABLE_ENTITY))
    }
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
    let present = |key: &str| c.params.get(key).filter(|p| p.is_present()).cloned();
    let (before, after) = (present("before"), present("after"));
    let room_id = room.id;
    c.app()
        .db
        .read(move |conn| match (before, after) {
            (Some(before), _) => {
                let message = paging_anchor(conn, Timeline::Room(room_id), &before)?;
                Message::page_before(conn, Timeline::Room(room_id), &message)
            }
            (None, Some(after)) => {
                let message = paging_anchor(conn, Timeline::Room(room_id), &after)?;
                Message::page_after(conn, Timeline::Room(room_id), &message)
            }
            (None, None) => Message::last_page(conn, Timeline::Room(room_id)),
        })
        .await
        .map_err(db_error)
}

pub(crate) fn paging_anchor(conn: &campfire_db::Connection, timeline: Timeline, value: &Param) -> campfire_db::Result<Message> {
    if let Param::Array(values) = value {
        // ActiveRecord find(array) first resolves every id, then pagination raises because the
        // resulting Array has no created_at. Unknown ids still raise RecordNotFound first.
        fn flatten<'a>(values: &'a [Param], ids: &mut Vec<&'a Param>) {
            for value in values {
                match value { Param::Array(values) => flatten(values, ids), Param::Null => {}, value => ids.push(value) }
            }
        }
        let mut ids = Vec::new();
        flatten(values, &mut ids);
        if ids.is_empty() { return Err(campfire_db::Error::RecordNotFound("Message")); }
        for id in ids { paging_anchor(conn, timeline, id)?; }
        return Err(campfire_db::Error::Other("Array has no created_at pagination cursor".into()));
    }
    let id = value.to_s().as_deref().and_then(cast_integer).ok_or(campfire_db::Error::RecordNotFound("Message"))?;
    Message::find_in(conn, timeline, id)
}

// --- Creating, updating, destroying ---------------------------------------------------------------

/// `@room.messages.create_with_attachment!(attributes)`: the message (with its uploaded blob, in
/// one transaction), then `process_attachment`. The upload's file is copied into storage and the
/// body canonicalized before the transaction. Root media processing follows the commit;
/// thread media processing stays inside `post_message!`'s transaction, as in Rails.
/// `@room.messages.create!` and, in its transaction, `deliver_webhooks_to_bots`: the webhook
/// jobs are held until the caller has broadcast the message ([`release_webhooks`]).
pub(crate) async fn create_message(c: &Ctx, room: &Room, attributes: MessageParams) -> Result<Message> {
    created_message(create_message_outcome(c, room, None, attributes, false).await?)
}

pub(crate) async fn create_message_into(c: &Ctx, room: &Room, thread: Option<campfire_db::ChannelThread>, attributes: MessageParams) -> Result<Message> {
    created_message(create_message_outcome(c, room, thread, attributes, false).await?)
}

fn created_message(outcome: campfire_db::models::agent_posting::PostingOutcome) -> Result<Message> {
    match outcome {
        campfire_db::models::agent_posting::PostingOutcome::Created(message) => Ok(message),
        _ => unreachable!("human posting does not run agent policy"),
    }
}

pub(crate) async fn create_message_with_agent_policy(c: &Ctx, room: &Room, attributes: MessageParams, agent_policy: bool) -> Result<campfire_db::models::agent_posting::PostingOutcome> {
    create_message_outcome(c, room, None, attributes, agent_policy).await
}

async fn create_message_outcome(c: &Ctx, room: &Room, thread: Option<campfire_db::ChannelThread>, mut attributes: MessageParams, agent_policy: bool) -> Result<campfire_db::models::agent_posting::PostingOutcome> {
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
    let storage = c.app().storage.clone();
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
            let attributes = NewMessage {
                    room_id,
                    creator_id,
                    client_message_id: attributes.client_message_id,
                    markdown_source: attributes.markdown_source,
                    reply_to_message_id: attributes.reply_to_message_id,
                    reply_notify_author: attributes.reply_notify_author,
                    drive_file_ids: attributes.drive_file_ids,
                    body,
                    attachment_blob_id: blob.as_ref().map(|blob| blob.id),
                    ..Default::default()
                };
            let message = if let Some(mut thread) = thread {
                    let message = thread.post_message(tx, creator_id, attributes)?;
                    if let Some(blob) = &blob { attachments::enqueue_analysis(tx, blob); }
                    crate::messaging::process_message_attachment(tx, storage, &message)?;
                    return Ok((PostingOutcome::Created(message), None));
                } else {
                    let message = Message::create(tx, attributes)?;
                    deliver_webhooks_to_bots(tx, &room, &message)?;
                    message
                };
            if let Some(blob) = &blob { attachments::enqueue_analysis(tx, blob); }
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
pub(crate) fn attachment_blob(tx: &mut campfire_db::Tx<'_>, assignment: Assignment<Staged>) -> campfire_db::Result<Option<Blob>> {
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
    active_storage::analyze_explicit(app, blob.id).await.map_err(Error::internal)?.ok_or(Error::NotFound)
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
    let (replies, thread) = c.app().db.write(move |tx| {
        let replies = tx.conn().prepare("SELECT id FROM messages WHERE reply_to_message_id = ? ORDER BY id")?
            .query_map([destroyed.id], |row| row.get::<_, i64>(0))?.collect::<std::result::Result<Vec<_>, _>>()?;
        let thread = campfire_db::ChannelThread::find_by_parent_message(tx.conn(), destroyed.id)?.map(|thread| thread.id);
        destroyed.destroy(tx)?;
        Ok((replies, thread))
    }).await.map_err(db_error)?;
    c.app().broadcasts.message_remove(room, message);
    rendered::broadcast_tombstones(c, replies).await?;
    if let Some(thread) = thread { rendered::broadcast_thread_refresh(c.app(), room.id, thread).await?; }
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
            let html = page::render_detached_at(&app, account.as_ref(), &base_url, |ctx| views::uncached_message(ctx, &view));
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
    // Explicit `render action: :room_not_found` looks up the request's format; Rails has
    // only the HTML template. A Turbo Stream/JSON rescue therefore raises MissingTemplate.
    if c.format()?.is_some_and(|requested| *requested != format::HTML) {
        return Err(Error::internal(anyhow::anyhow!("Missing messages/room_not_found template for request format")));
    }
    c.respond_to(&[&format::HTML])?;
    page::content_in_application_layout(c, StatusCode::OK, |_| views::RoomNotFound.render()).await
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod http_tests;
#[cfg(test)]
#[path = "messages/ws17_activity_tests.rs"]
mod ws17_activity_tests;
