//! `MessagesController` (reference/app/controllers/messages_controller.rb), including the
//! multipart attachment upload the composer's `FileUploader` posts here, plus what
//! `Messages::ByBotsController` reuses: creating a message (with `process_attachment`),
//! delivering webhooks and the broadcasts.

pub mod boosts;
pub mod pins;
pub mod by_bots;
pub mod rendered;
pub use crate::controllers::presenters::message_payload as payload;
pub use crate::controllers::presenters::message_freshness as freshness;

use campfire_db::{Job as _, Message, NewMessage, Room, Timeline};
use campfire_kit::format;
use campfire_kit::{Ctx, Error, Param, Result, StatusCode, halt, permit_keys};
use campfire_storage::{Blob, Staged};

use crate::app::AppCtx;
use crate::concerns::{self, Before, before_actions, cast_integer, require_current_user};
use crate::controllers::presenters::attachments::{self, Assignment};
use crate::controllers::presenters::page::db_error;
use crate::controllers::presenters::{DbResolver, Presenter};
use crate::queue::{WEBHOOK_HOLD, WebhookJob};
pub use crate::messaging::{canonicalize_body, process_attachment, save_staged};

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
    if json { return Err(Error::UnknownFormat); }
    c.redirect_to(&c.url_for(&format!("/app/r/{}", room.id)))
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
    let _message = if let Some(duplicate) = duplicate {
        let app = c.app().clone();
        let message = duplicate.clone();
        let refreshes = c.app().db.read(move |conn| campfire_runtime::presenters::broadcast_refreshes(conn, &app, &message)).await.map_err(db_error)?;
        campfire_runtime::presenters::refresh_after_render(&c.app().db, refreshes).await;
        duplicate
    } else {
        let attributes = human_message_params_with_client_id(c, Some(&room), client_id).await?;
        let message = create_message(c, &room, attributes).await?;
        broadcast_create(c, &room, &message).await?;
        release_webhooks(c, &message).await;
        message
    };

    Ok(c.head(StatusCode::CREATED))
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

    Ok(c.head(StatusCode::NO_CONTENT))
}

// --- Before-actions and params --------------------------------------------------------------------

/// `RoomScoped`: a membership in an alive room, including during deferred deletion.
pub async fn set_root_room(c: &mut Ctx) -> Result<Room> {
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
        let body = campfire_presentation::helpers::to_rails_json(&serde_json::json!({"error": "Markdown is limited to 50,000 characters"}));
        return Ok(c.render(StatusCode::UNPROCESSABLE_ENTITY, &format::JSON, body));
    }
    let html = render_preview(c, room.id, source).await?;
    let body = serde_json::to_string(&serde_json::json!({"html": html})).map_err(Error::internal)?;
    Ok(c.render(StatusCode::OK, &format::JSON, body))
}

/// `messages#preview`'s rendering: `source` as a message in `room_id` would present it.
pub async fn render_preview(c: &Ctx, room_id: i64, source: String) -> Result<String> {
    let (app, request_host) = (c.app().clone(), Some(c.request.host()));
    c.app().db.read(move |conn| {
        let body = app.db.env().rich_text.render_markdown(conn, &source, room_id).map_err(campfire_db::Error::Other)?;
        let resolver = DbResolver::new(conn, &app.secrets, app.clock.now());
        crate::rich_text::markdown_presentation(conn, &body, &resolver.render_context(request_host)).map_err(campfire_db::Error::Other)
    }).await.map_err(db_error)
}

pub fn ensure_can_edit(c: &mut Ctx, message: &Message) -> Result<()> {
    if message.system_note || require_current_user(c)?.id != message.creator_id {
        return halt(concerns::head(StatusCode::FORBIDDEN));
    }
    Ok(())
}

pub fn ensure_can_delete(c: &mut Ctx, message: &Message) -> Result<()> {
    let user = require_current_user(c)?;
    if message.system_note || (user.id != message.creator_id && !user.is_administrator()) {
        return halt(concerns::head(StatusCode::FORBIDDEN));
    }
    Ok(())
}

/// `@room.messages.find(params[:id])`
pub async fn set_message(c: &mut Ctx, room: &Room) -> Result<Message> {
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

/// The caller's rule for assigning a verified blob.
#[derive(Debug, Default, Clone, Copy)]
pub enum AttachmentPolicy {
    /// A verified signed blob is reusable, as on the legacy endpoints.
    #[default]
    SignedBlob,
    /// SPA uploads belong to the poster and can only be attached once.
    OwnedUpload { uploader_id: i64 },
    /// Editing a scheduled draft may keep uploads already attached to that draft.
    ScheduledUpload { uploader_id: i64, scheduled_id: i64 },
}

/// What `create_with_attachment!`/`update!` receive.
#[derive(Debug, Default, Clone)]
pub struct MessageParams {
    pub clear_markdown_source: bool,
    pub client_message_lookup: campfire_db::models::agent_posting::client_ids::Lookup,
    pub body: Option<String>,
    pub markdown_source: Option<String>,
    /// `attachment=`: `None` when the key wasn't given.
    pub attachment: Option<Assignment>,
    pub attachments: Vec<Assignment>,
    pub attachment_policy: AttachmentPolicy,
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
        attachment_policy: AttachmentPolicy::SignedBlob,
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
pub async fn human_message_params_with_client_id(c: &Ctx, root_room: Option<&Room>, client_id: Option<String>) -> Result<MessageParams> {
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

pub async fn update_human_message(c: &Ctx, root_room: Option<&Room>, thread_id: Option<i64>, message: Message) -> Result<Message> {
    let attributes = human_message_params(c, root_room).await?;
    let params = c.params.require("message")?;
    let scalar = params.permit(&permit_keys(&["client_message_id", "reply_to_message_id", "reply_notify_author"]));
    let attachment_given = attributes.attachment.is_some();
    let attachment = stage_attachment(c, attributes.attachment.unwrap_or(Assignment::Unchanged)).await?;
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
    apply_human_edit(c, thread_id, message, changes, attachment, attachment_given).await
}

/// The SPA's edit (`PATCH /api/v1/messages/:id`): the classic update given
/// `message[markdown_source]`, and `remove_drive_file_ids` dropped from the current set
/// (`message[drive_file_ids][]` on the classic edit form is what remains).
pub async fn update_markdown_source(c: &Ctx, thread_id: Option<i64>, message: Message, markdown_source: String, remove_drive_file_ids: &[String]) -> Result<Message> {
    let drive_file_ids = if remove_drive_file_ids.is_empty() {
        None
    } else {
        let id = message.id;
        let current = c.app().db.read(move |conn| Message::find(conn, id)?.drive_file_ids(conn)).await.map_err(db_error)?;
        Some(current.into_iter().filter(|file_id| !remove_drive_file_ids.iter().any(|removed| removed == file_id)).collect())
    };
    let changes = campfire_db::MessageChanges { markdown_source: Some(markdown_source), drive_file_ids, ..Default::default() };
    let attachment = Assignment::Unchanged.stage(c.app()).await?;
    apply_human_edit(c, thread_id, message, changes, attachment, false).await
}

/// The write half of [`update_human_message`]: a locked thread refuses, a rich-text message
/// edited into Markdown keeps its attachments, then `assign_attributes` + `save!`.
async fn apply_human_edit(c: &Ctx, thread_id: Option<i64>, message: Message, changes: campfire_db::MessageChanges, attachment: Assignment<Staged>, attachment_given: bool) -> Result<Message> {
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
        let blob = attachment_blob(tx, attachment, AttachmentPolicy::SignedBlob, "attachment")?;
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
pub fn string_column(value: &Param) -> Option<String> {
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
        let body = campfire_presentation::helpers::to_rails_json(&serde_json::json!({"errors": by_attribute}));
        Ok(c.render(StatusCode::UNPROCESSABLE_ENTITY, &format::JSON, body))
    } else {
        Ok(c.head(StatusCode::UNPROCESSABLE_ENTITY))
    }
}

/// What assigning the permitted `attachment` does: an upload replaces the attachment, nil or ""
/// removes it (`Attached::Changes::DeleteOne`), anything else raises.
pub fn attachment_assignment(permitted: &campfire_kit::ParamMap) -> Result<Option<Assignment>> {
    match Assignment::from_params(permitted, "attachment")? {
        Assignment::Unchanged => Ok(None),
        assignment => Ok(Some(assignment)),
    }
}

pub async fn stage_attachment(c: &Ctx, assignment: Assignment) -> Result<Assignment<Staged>> {
    if matches!(assignment, Assignment::Unchanged | Assignment::Delete | Assignment::Invalid) {
        return assignment.stage(c.app()).await;
    }
    let limit = campfire_runtime::active_storage::upload_limit_bytes(c.app()).await?;
    assignment.stage_with_limit(c.app(), limit as u64).await
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

pub fn paging_anchor(conn: &campfire_db::Connection, timeline: Timeline, value: &Param) -> campfire_db::Result<Message> {
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
    created_message(create_message_outcome(c, room, None, attributes, false, false).await?)
}

pub async fn create_message_into(c: &Ctx, room: &Room, thread: Option<campfire_db::ChannelThread>, attributes: MessageParams) -> Result<Message> {
    created_message(create_message_outcome(c, room, thread, attributes, false, false).await?)
}

/// [`create_message`], unless the creator already posted the message's `client_message_id` in
/// the room (`Message.find_duplicate`): `Replay` then carries the earlier message. The check runs
/// in the create's write transaction, so retries racing each other can't both create.
pub async fn create_or_find_message(c: &Ctx, room: &Room, attributes: MessageParams) -> Result<campfire_db::models::agent_posting::PostingOutcome> {
    create_message_outcome(c, room, None, attributes, false, true).await
}

/// [`create_or_find_message`] for a reply in `thread`.
pub async fn create_or_find_reply(c: &Ctx, room: &Room, thread: campfire_db::ChannelThread, attributes: MessageParams) -> Result<campfire_db::models::agent_posting::PostingOutcome> {
    create_message_outcome(c, room, Some(thread), attributes, false, true).await
}

/// What [`create_or_find_thread`] did.
pub enum ThreadOutcome {
    /// The thread, with its first reply.
    Created(campfire_db::ChannelThread, Message),
    /// A retry: the thread the reply's `client_message_id` already started.
    Replay(campfire_db::ChannelThread, Message),
    /// The reply's `client_message_id` already names a message outside any thread.
    ClientMessageIdTaken,
}

/// `channel_threads#create` in a channel (not a board): the thread on `parent_message_id`, the
/// creator's thread membership and the first reply, in one write. A retry of the reply's
/// `client_message_id` finds the thread its first attempt started. Posts nothing itself; the
/// thread's frames come from its write, as the classic action's do.
pub async fn create_or_find_thread(c: &Ctx, room: &Room, parent_message_id: i64, name: Option<String>, attributes: MessageParams) -> Result<ThreadOutcome> {
    use campfire_db::{ChannelThread, Membership, NewChannelThread, ThreadMembership};
    let creator_id = require_current_user(c)?.id;
    let room_id = room.id;
    let attachment = stage_attachment(c, attributes.attachment.unwrap_or(Assignment::Unchanged)).await?;
    if matches!(attachment, Assignment::Invalid) {
        return Err(invalid_attachment());
    }
    let files = attachments::stage_many(c.app(), attributes.attachments).await?;
    let storage = c.app().storage.clone();
    let outcome = c
        .app()
        .db
        .write(move |tx| {
            if let Some(client_message_id) = attributes.client_message_id.as_deref()
                && let Some(duplicate) = Message::find_duplicate(tx.conn(), room_id, creator_id, client_message_id)?
            {
                return Ok(match duplicate.thread_id {
                    Some(thread_id) => ThreadOutcome::Replay(ChannelThread::find(tx.conn(), thread_id)?, duplicate),
                    None => ThreadOutcome::ClientMessageIdTaken,
                });
            }
            let room = Room::find(tx.conn(), room_id)?;
            if room.deleted_at.is_some() || Membership::find_by_room_and_user(tx.conn(), room_id, creator_id)?.is_none() {
                return Err(campfire_db::Error::RecordNotFound("Membership"));
            }
            let mut thread = ChannelThread::create(tx, NewChannelThread {
                room_id,
                creator_id,
                parent_message_id: Some(parent_message_id),
                name,
                ..Default::default()
            })?;
            ThreadMembership::join(tx, thread.id, creator_id)?;
            let blob = attachment_blob(tx, attachment, attributes.attachment_policy, "attachment_signed_id")?;
            let files = attachment_blobs(tx, files, attributes.attachment_policy)?;
            let message = thread.post_message(tx, creator_id, NewMessage {
                markdown_source: attributes.markdown_source,
                client_message_id: attributes.client_message_id,
                attachment_blob_id: blob.as_ref().map(|blob| blob.id),
                attachment_blob_ids: files.iter().map(|blob| blob.id).collect(),
                reply_to_message_id: attributes.reply_to_message_id,
                reply_notify_author: attributes.reply_notify_author,
                ..Default::default()
            })?;
            if let Some(blob) = &blob { attachments::enqueue_analysis(tx, blob); }
            for blob in &files { attachments::enqueue_analysis(tx, blob); }
            crate::messaging::process_message_attachment(tx, storage, &message)?;
            Ok(ThreadOutcome::Created(thread, message))
        })
        .await
        .map_err(db_error)?;
    match outcome {
        ThreadOutcome::Created(thread, message) => {
            let (thread_id, message_id) = (thread.id, message.id);
            c.app().db.read(move |conn| Ok(ThreadOutcome::Created(campfire_db::ChannelThread::find(conn, thread_id)?, Message::find(conn, message_id)?)))
                .await
                .map_err(db_error)
        }
        replay => Ok(replay),
    }
}

fn created_message(outcome: campfire_db::models::agent_posting::PostingOutcome) -> Result<Message> {
    match outcome {
        campfire_db::models::agent_posting::PostingOutcome::Created(message) => Ok(message),
        _ => unreachable!("human posting does not run agent policy"),
    }
}

pub(crate) async fn create_message_with_agent_policy(c: &Ctx, room: &Room, attributes: MessageParams, agent_policy: bool) -> Result<campfire_db::models::agent_posting::PostingOutcome> {
    create_message_outcome(c, room, None, attributes, agent_policy, false).await
}

async fn create_message_outcome(c: &Ctx, room: &Room, thread: Option<campfire_db::ChannelThread>, mut attributes: MessageParams, agent_policy: bool, replay_duplicate: bool) -> Result<campfire_db::models::agent_posting::PostingOutcome> {
    use campfire_db::models::agent_posting::{PostingOutcome, PostingCheck};
    let creator_id = require_current_user(c)?.id;
    let room_id = room.id;
    let room = room.clone();
    let attachment = stage_attachment(c, attributes.attachment.unwrap_or(Assignment::Unchanged)).await?;
    if matches!(attachment, Assignment::Invalid) {
        return Err(invalid_attachment());
    }
    let files = attachments::stage_many(c.app(), attributes.attachments).await?;
    let body = match attributes.body {
        Some(body) => Some(canonicalize_body(c.app(), body, Some(c.request.host())).await?),
        None => None,
    };
    let storage = c.app().storage.clone();
    let (message, blobs) = c
        .app()
        .db
        .write(move |tx| {
            if replay_duplicate
                && let Some(client_message_id) = attributes.client_message_id.as_deref()
                && let Some(duplicate) = Message::find_duplicate(tx.conn(), room_id, creator_id, client_message_id)?
            {
                return Ok((PostingOutcome::Replay(duplicate), Vec::new()));
            }
            if agent_policy {
                match campfire_db::models::agent_posting::prepare_for_user_with_lookup(tx, creator_id, room_id, attributes.client_message_id.as_deref(), &attributes.client_message_lookup)? {
                    Some(PostingCheck::Replay(message)) => return Ok((PostingOutcome::Replay(*message), Vec::new())),
                    Some(PostingCheck::Budget(payload)) => return Ok((PostingOutcome::Budget(payload), Vec::new())),
                    Some(PostingCheck::Allowed) => {},
                    None => attributes.client_message_id = None,
                }
            }
            let blob = attachment_blob(tx, attachment, attributes.attachment_policy, "attachment_signed_id")?;
            let mut files = attachment_blobs(tx, files, attributes.attachment_policy)?;
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
                    attachment_blob_ids: files.iter().map(|blob| blob.id).collect(),
                    ..Default::default()
                };
            files.extend(blob);
            let message = if let Some(mut thread) = thread {
                    let message = thread.post_message(tx, creator_id, attributes)?;
                    for blob in &files { attachments::enqueue_analysis(tx, blob); }
                    crate::messaging::process_message_attachment(tx, storage, &message)?;
                    return Ok((PostingOutcome::Created(message), Vec::new()));
                } else {
                    let message = Message::create(tx, attributes)?;
                    deliver_webhooks_to_bots(tx, &room, &message)?;
                    message
                };
            for blob in &files { attachments::enqueue_analysis(tx, blob); }
            Ok((PostingOutcome::Created(message), files))
        })
        .await
        .map_err(db_error)?;
    for blob in blobs {
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

/// Resolve and claim an upload inside the same writer transaction that attaches it.
/// BEGIN IMMEDIATE prevents another writer from claiming it before this save finishes.
pub fn attachment_blob(tx: &mut campfire_db::Tx<'_>, assignment: Assignment<Staged>, policy: AttachmentPolicy, attribute: &'static str) -> campfire_db::Result<Option<Blob>> {
    match assignment {
        Assignment::Create(staged) => save_staged(tx, staged).map(Some),
        Assignment::Existing(blob) => {
            if let AttachmentPolicy::OwnedUpload { uploader_id } | AttachmentPolicy::ScheduledUpload { uploader_id, .. } = policy {
                let current = Blob::find(tx.conn(), blob.id).map_err(attachments::storage_error)?
                    .ok_or(campfire_db::Error::RecordNotFound("ActiveStorage::Blob"))?;
                let owner = current.metadata.get("uploader_id").and_then(campfire_storage::Json::as_i64);
                let scheduled_id = match policy {
                    AttachmentPolicy::ScheduledUpload { scheduled_id, .. } => Some(scheduled_id),
                    _ => None,
                };
                let attached: bool = tx.conn().query_row(
                    "SELECT EXISTS(SELECT 1 FROM active_storage_attachments WHERE blob_id=?1 AND (?2 IS NULL OR record_type!='ScheduledMessage' OR record_id!=?2 OR name!='attachments'))",
                    (blob.id, scheduled_id), |row| row.get(0),
                )?;
                let mut errors = campfire_db::Errors::default();
                if owner != Some(uploader_id) {
                    errors.add(attribute, "includes an upload that isn't yours");
                } else if attached {
                    errors.add(attribute, "includes an upload that is already attached");
                }
                errors.into_result()?;
            }
            attachments::save_existing(tx, blob).map(Some)
        }
        Assignment::Unchanged | Assignment::Delete => Ok(None),
        Assignment::Signed(_) | Assignment::Invalid => Err(campfire_db::Error::Other("invalid attachment".into())),
    }
}

fn attachment_blobs(tx: &mut campfire_db::Tx<'_>, assignments: Vec<Assignment<Staged>>, policy: AttachmentPolicy) -> campfire_db::Result<Vec<Blob>> {
    assignments.into_iter().filter_map(|assignment| attachment_blob(tx, assignment, policy, "attachment_signed_ids").transpose()).collect()
}

/// Assigning something that isn't an upload, a signed blob id, nil or "".
fn invalid_attachment() -> Error {
    Error::internal(anyhow::anyhow!("Could not find or build blob: expected attachable"))
}

/// `@message.update!(message_params)`. A new attachment replaces the old one (whose blob is purged
/// later). `Message::replace_attachment` schedules #226's processing after commit as well
/// as Active Storage's ordinary analysis callback.
pub(crate) async fn update_message(c: &Ctx, message: Message, attributes: MessageParams) -> Result<Message> {
    let attachment_given = attributes.attachment.is_some();
    let attachment = stage_attachment(c, attributes.attachment.unwrap_or(Assignment::Unchanged)).await?;
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
            let blob = attachment_blob(tx, attachment, attributes.attachment_policy, "attachment")?;
            if attachment_given {
                message.replace_attachment(tx, blob.as_ref().map(|blob| blob.id))?;
            }
            message.edit(tx, campfire_db::MessageChanges {
                clear_markdown_source: attributes.clear_markdown_source,
                markdown_source: attributes.markdown_source,
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
pub async fn destroy_message(c: &Ctx, room: &Room, message: &Message) -> Result<()> {
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
pub async fn broadcast_create(c: &Ctx, room: &Room, message: &Message) -> Result<()> {
    let (app, room, message) = (c.app().clone(), room.clone(), message.clone());
    let refreshes = c.app().db.read(move |conn| {
        let refreshes = campfire_runtime::presenters::broadcast_refreshes(conn, &app, &message)?;
        app.broadcasts.message_create(conn, &room, &message, &*app.db.env().rich_text)?;
        Ok(refreshes)
    }).await.map_err(db_error)?;
    campfire_runtime::presenters::refresh_after_render(&c.app().db, refreshes).await;
    Ok(())
}

/// `broadcast_replace_to @room, :messages, target: [ @message, :presentation ], partial:
/// "messages/presentation", attributes: { maintain_scroll: true }`
pub(crate) async fn broadcast_replace(c: &Ctx, room: &Room, message: &Message) -> Result<()> {
    rendered::broadcast_edit(c, room, message, false).await
}

/// `deliver_webhooks_to_bots`, in the message's transaction: every active bot in a direct room,
/// else every mentioned active bot, except the message's creator, gets `bot.deliver_webhook_later
/// (@message)` when it has a webhook. Rails enqueues them after the message is saved, processed
/// and broadcast, in a separate step that a crash (or a failed enqueue) can skip. Here the job
/// rows commit, or roll back, with the message; each is held for [`WEBHOOK_HOLD`] so a bot can't
/// be told (and reply) before the room sees the message, and [`release_webhooks`] makes them due
/// once it has been broadcast.
pub fn deliver_webhooks_to_bots(tx: &mut campfire_db::Tx<'_>, room: &Room, message: &Message) -> campfire_db::Result<()> {
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
pub async fn release_webhooks(c: &Ctx, message: &Message) {
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
pub async fn present<T: Send + 'static>(
    c: &Ctx,
    f: impl FnOnce(&Presenter) -> campfire_db::Result<T> + Send + 'static,
) -> Result<T> {
    let app = c.app().clone();
    let request_host = Some(c.request.host());
    let cache_base_url = c.url_for("");
    let current_user_id = require_current_user(c)?.id;
    let (value, fetches, twitter_fetches, refreshes) = c.app()
        .db
        .read(move |conn| {
            let mut presenter = Presenter::new(conn, &app, request_host);
            presenter.cache_base_url = Some(cache_base_url);
            presenter.current_user_id = Some(current_user_id);
            presenter.use_viewer_zone(current_user_id)?;
            let value = f(&presenter)?;
            Ok((value, presenter.pending_link_fetches(), presenter.pending_twitter_fetches(), presenter.take_render_refreshes()))
        })
        .await
        .map_err(db_error)?;
    super::presenters::link_embeds::enqueue_render_fetches(c.app(), fetches, twitter_fetches)
        .await
        .map_err(db_error)?;
    crate::controllers::presenters::refresh_after_render(&c.app().db, refreshes).await;
    Ok(value)
}

async fn render_room_not_found(c: &mut Ctx) -> Result {
    if c.format()?.is_some_and(|requested| *requested != format::HTML) {
        return Err(Error::internal(anyhow::anyhow!("Unsupported missing-room response format")));
    }
    Ok(c.head(StatusCode::OK))
}
