//! `reference/app/models/message.rb` and `message/*.rb` (Attachment, Mentionee, Pagination,
//! Searchable; Broadcasts belong to the app, except the thread indicator and quote cards, which
//! are emitted as [`Event::Broadcast`]).

use rusqlite::{Connection, Row, params};

use crate::database::Tx;
use crate::error::{Errors, OptionalExt, Result};
use crate::events::Event;
use crate::models::{
    Attachment, Blob, Boost, ChannelThread, RichTextRecord, Room, RoomType, Sound, User,
};
use crate::rich_text::{RichText, mention_token_names};
use crate::sql::{self, CachedStatements, placeholders, query_all, query_one};
use crate::time::Timestamp;

/// `Message::Pagination::PAGE_SIZE`
pub const PAGE_SIZE: i64 = 40;

/// `Message::Markdown::SOURCE_LIMIT`: the longest Markdown source (and forward note).
pub const SOURCE_LIMIT: usize = 50_000;

/// `DriveAttachment::MAX_PER_MESSAGE`
pub const DRIVE_ATTACHMENTS_PER_MESSAGE: usize = 10;

const RECORD_TYPE: &str = "Message";

#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    pub id: i64,
    pub room_id: i64,
    pub creator_id: i64,
    pub client_message_id: String,
    /// The `ChannelThread` the message was posted in; nil on the room's root timeline.
    pub thread_id: Option<i64>,
    /// A quiet timeline note: rendered, but never unread, pushed, delivered or indexed.
    pub system_note: bool,
    /// Still being written by an agent: every noisy side effect waits for the finalize.
    pub streaming: bool,
    pub streaming_updated_at: Option<Timestamp>,
    pub stream_broadcast_at: Option<Timestamp>,
    pub markdown_source: Option<String>,
    pub action: bool,
    pub board_post_opener: bool,
    pub edited_at: Option<Timestamp>,
    pub embeds_suppressed: bool,
    pub reply_to_message_id: Option<i64>,
    pub reply_target_deleted_at: Option<Timestamp>,
    pub reply_notify_author: bool,
    pub forwarded_from_message_id: Option<i64>,
    pub forwarded_at: Option<Timestamp>,
    pub forward_note: Option<String>,
    pub forwarded_markdown: bool,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// Attributes for `room.messages.create!` / `create_with_attachment!`.
#[derive(Debug, Clone, Default)]
pub struct NewMessage {
    pub room_id: i64,
    pub creator_id: i64,
    /// Defaults to a random UUID (`before_create`; "Bots don't care").
    pub client_message_id: Option<String>,
    /// The stored Action Text body, if one was assigned.
    pub body: Option<String>,
    /// An already-saved blob to attach as `attachment`.
    pub attachment_blob_id: Option<i64>,
    /// Posts into a thread instead of the room's root timeline.
    pub thread_id: Option<i64>,
    pub system_note: bool,
    pub streaming: bool,
    /// Markdown source: rendered into the body before validation (`render_markdown_body`).
    pub markdown_source: Option<String>,
    /// A `/me` action.
    pub action: bool,
    pub board_post_opener: bool,
    pub embeds_suppressed: bool,
    pub reply_to_message_id: Option<i64>,
    /// Whether the replied-to author is notified; the column defaults to true.
    pub reply_notify_author: Option<bool>,
    pub forwarded_from_message_id: Option<i64>,
    pub forwarded_at: Option<Timestamp>,
    pub forward_note: Option<String>,
    pub forwarded_markdown: bool,
    /// Google Drive file ids (`drive_attachments.build(file_id:)`).
    pub drive_file_ids: Vec<String>,
}

/// Attributes assigned to a saved message (`message.update!(...)`); `None` leaves one alone.
#[derive(Debug, Clone, Default)]
pub struct MessageChanges {
    /// Rendered into the body before validation, like a new message's.
    pub markdown_source: Option<String>,
    /// The human edit endpoint assigns nil when switching back to a legacy body.
    pub clear_markdown_source: bool,
    /// Rendered non-mention attachments retained during a legacy-to-Markdown edit.
    pub legacy_attachment_snapshot: Option<String>,
    pub client_message_id: Option<Option<String>>,
    pub reply_to_message_id: Option<Option<i64>>,
    pub reply_notify_author: Option<bool>,
    /// A legacy (Action Text) body.
    pub body: Option<String>,
    pub forward_note: Option<Option<String>>,
    pub embeds_suppressed: Option<bool>,
    /// The whole Drive set (`apply_drive_file_ids!`): ids missing from it are destroyed, new
    /// ones built, the rest kept.
    pub drive_file_ids: Option<Vec<String>>,
}

/// The message list a page is cut from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Timeline {
    /// `room.root_messages`: the room's messages outside threads, which every room timeline
    /// pages through (`MessagesController`, `RoomsController`, `Rooms::RefreshesController`).
    Room(i64),
    /// `thread.messages` (`ChannelThread`).
    Thread(i64),
}

impl Timeline {
    /// The `WHERE` condition and its one bound value.
    fn condition(self) -> (&'static str, i64) {
        match self {
            Timeline::Room(room_id) => (
                r#""messages"."room_id" = ? AND "messages"."thread_id" IS NULL"#,
                room_id,
            ),
            Timeline::Thread(thread_id) => (r#""messages"."thread_id" = ?"#, thread_id),
        }
    }
}

/// `scope :ordered, -> { order(:created_at, :id) }`: the id breaks ties, so page windows agree
/// with the `(created_at, id)` cursors.
const ORDERED: &str = r#"ORDER BY "messages"."created_at" ASC, "messages"."id" ASC"#;
/// `ordered.last(n)` reverses `ordered` and takes the first n.
const ORDERED_REVERSE: &str = r#"ORDER BY "messages"."created_at" DESC, "messages"."id" DESC"#;
/// `Message::Pagination`'s `before(message)` and `after(message)`: tuple comparisons, so
/// messages sharing a timestamp at a page edge are neither skipped nor repeated.
const BEFORE: &str = "(messages.created_at, messages.id) < (?, ?)";
const AFTER: &str = "(messages.created_at, messages.id) > (?, ?)";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentType {
    Attachment,
    Sound,
    Text,
}

impl ContentType {
    pub fn name(self) -> &'static str {
        match self {
            ContentType::Attachment => "attachment",
            ContentType::Sound => "sound",
            ContentType::Text => "text",
        }
    }
}

const SELECT_IN_ROOM: &str =
    r#"SELECT "messages".* FROM "messages" WHERE "messages"."room_id" = ?"#;

/// `user.reachable_messages`: through `user.rooms`, so alive rooms only (`app/models/user.rb`).
const SELECT_REACHABLE: &str = r#"SELECT "messages".* FROM "messages" INNER JOIN "rooms" ON "messages"."room_id" = "rooms"."id" INNER JOIN "memberships" ON "rooms"."id" = "memberships"."room_id""#;

impl Message {
    pub(crate) fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            room_id: row.get("room_id")?,
            creator_id: row.get("creator_id")?,
            client_message_id: row.get("client_message_id")?,
            thread_id: row.get("thread_id")?,
            system_note: row.get("system_note")?,
            streaming: row.get("streaming")?,
            streaming_updated_at: row.get("streaming_updated_at")?,
            stream_broadcast_at: row.get("stream_broadcast_at")?,
            markdown_source: row.get("markdown_source")?,
            action: row.get("action")?,
            board_post_opener: row.get("board_post_opener")?,
            edited_at: row.get("edited_at")?,
            embeds_suppressed: row.get("embeds_suppressed")?,
            reply_to_message_id: row.get("reply_to_message_id")?,
            reply_target_deleted_at: row.get("reply_target_deleted_at")?,
            reply_notify_author: row.get("reply_notify_author")?,
            forwarded_from_message_id: row.get("forwarded_from_message_id")?,
            forwarded_at: row.get("forwarded_at")?,
            forward_note: row.get("forward_note")?,
            forwarded_markdown: row.get("forwarded_markdown")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    pub fn find(conn: &Connection, id: i64) -> Result<Self> {
        Self::find_by_id(conn, id)?.or_not_found("Message")
    }

    pub fn find_by_id(conn: &Connection, id: i64) -> Result<Option<Self>> {
        query_one(
            conn,
            r#"SELECT * FROM "messages" WHERE "messages"."id" = ? LIMIT 1"#,
            [id],
            Self::from_row,
        )
    }

    /// `Message.last`
    pub fn last(conn: &Connection) -> Result<Option<Self>> {
        query_one(
            conn,
            r#"SELECT * FROM "messages" ORDER BY "messages"."id" DESC LIMIT 1"#,
            [],
            Self::from_row,
        )
    }

    pub fn count(conn: &Connection) -> Result<i64> {
        sql::count(conn, r#"SELECT COUNT(*) FROM "messages""#, [])
    }

    /// `user.messages`
    pub fn by_creator(conn: &Connection, creator_id: i64) -> Result<Vec<Self>> {
        query_all(
            conn,
            r#"SELECT "messages".* FROM "messages" WHERE "messages"."creator_id" = ?"#,
            [creator_id],
            Self::from_row,
        )
    }

    /// `ordered`
    pub fn ordered(conn: &Connection) -> Result<Vec<Self>> {
        query_all(
            conn,
            &format!(r#"SELECT "messages".* FROM "messages" {ORDERED}"#),
            [],
            Self::from_row,
        )
    }

    /// `room.messages`
    pub fn for_room(conn: &Connection, room_id: i64) -> Result<Vec<Self>> {
        query_all(conn, SELECT_IN_ROOM, [room_id], Self::from_row)
    }

    /// `room.messages.find(id)`
    pub fn find_in_room(conn: &Connection, room_id: i64, id: i64) -> Result<Self> {
        let sql = format!(r#"{SELECT_IN_ROOM} AND "messages"."id" = ? LIMIT 1"#);
        query_one(conn, &sql, [room_id, id], Self::from_row)?.or_not_found("Message")
    }

    /// `room.root_messages.count` (bot API pagination excludes replies).
    pub fn count_roots_in_room(conn: &Connection, room_id: i64) -> Result<i64> {
        sql::count(
            conn,
            r#"SELECT COUNT(*) FROM "messages" WHERE "room_id" = ? AND "thread_id" IS NULL"#,
            [room_id],
        )
    }

    /// `room.messages.count`
    pub fn count_in_room(conn: &Connection, room_id: i64) -> Result<i64> {
        sql::count(
            conn,
            r#"SELECT COUNT(*) FROM "messages" WHERE "messages"."room_id" = ?"#,
            [room_id],
        )
    }

    /// `Current.user.reachable_messages.find(id)`
    pub fn find_reachable(conn: &Connection, user_id: i64, id: i64) -> Result<Self> {
        let sql = format!(
            r#"{SELECT_REACHABLE} WHERE "rooms"."deleted_at" IS NULL AND "memberships"."user_id" = ? AND "messages"."id" = ? LIMIT 1"#
        );
        query_one(conn, &sql, [user_id, id], Self::from_row)?.or_not_found("Message")
    }

    // Message::Pagination (`app/models/message/pagination.rb`)

    /// `root_messages.find(id)` / `thread.messages.find(id)`
    pub fn find_in(conn: &Connection, timeline: Timeline, id: i64) -> Result<Self> {
        let (condition, value) = timeline.condition();
        let sql = format!(
            r#"SELECT "messages".* FROM "messages" WHERE {condition} AND "messages"."id" = ? LIMIT 1"#
        );
        query_one(conn, &sql, [value, id], Self::from_row)?.or_not_found("Message")
    }

    /// The page of `timeline` matching `filter` (bound to `values`), `ordered` and cut to
    /// `PAGE_SIZE`: the first page, or the last (still oldest first).
    fn page(
        conn: &Connection,
        timeline: Timeline,
        filter: &str,
        values: Vec<rusqlite::types::Value>,
        last: bool,
    ) -> Result<Vec<Self>> {
        let (condition, value) = timeline.condition();
        let order = if last { ORDERED_REVERSE } else { ORDERED };
        let sql = format!(
            r#"SELECT "messages".* FROM "messages" WHERE {condition}{filter} {order} LIMIT {PAGE_SIZE}"#
        );
        let values = std::iter::once(rusqlite::types::Value::from(value)).chain(values);
        let rows = query_all(
            conn,
            &sql,
            rusqlite::params_from_iter(values),
            Self::from_row,
        )?;
        Ok(if last { reversed(rows) } else { rows })
    }

    fn cursor(message: &Message) -> Vec<rusqlite::types::Value> {
        vec![message.created_at.to_db().into(), message.id.into()]
    }

    /// `last_page`: `ordered.last(40)`, the newest 40, oldest first.
    pub fn last_page(conn: &Connection, timeline: Timeline) -> Result<Vec<Self>> {
        Self::page(conn, timeline, "", Vec::new(), true)
    }

    /// `first_page`: `ordered.first(40)`
    pub fn first_page(conn: &Connection, timeline: Timeline) -> Result<Vec<Self>> {
        Self::page(conn, timeline, "", Vec::new(), false)
    }

    /// `page_before(message)`: `before(message).last_page`
    pub fn page_before(
        conn: &Connection,
        timeline: Timeline,
        message: &Message,
    ) -> Result<Vec<Self>> {
        Self::page(
            conn,
            timeline,
            &format!(" AND ({BEFORE})"),
            Self::cursor(message),
            true,
        )
    }

    /// `page_after(message)`: `after(message).first_page`
    pub fn page_after(
        conn: &Connection,
        timeline: Timeline,
        message: &Message,
    ) -> Result<Vec<Self>> {
        Self::page(
            conn,
            timeline,
            &format!(" AND ({AFTER})"),
            Self::cursor(message),
            false,
        )
    }

    /// `page_around(message)`: up to 40 before, the message, up to 40 after.
    pub fn page_around(
        conn: &Connection,
        timeline: Timeline,
        message: &Message,
    ) -> Result<Vec<Self>> {
        let mut page = Self::page_before(conn, timeline, message)?;
        page.push(message.clone());
        page.extend(Self::page_after(conn, timeline, message)?);
        Ok(page)
    }

    /// `page_created_since(time)`: `where("created_at > ?", time).first_page`
    pub fn page_created_since(
        conn: &Connection,
        timeline: Timeline,
        time: Timestamp,
    ) -> Result<Vec<Self>> {
        Self::page(
            conn,
            timeline,
            " AND (created_at > ?)",
            vec![time.to_db().into()],
            false,
        )
    }

    /// `without(excluding).page_updated_since(time)`: `where("updated_at > ?", time).last_page`
    pub fn page_updated_since(
        conn: &Connection,
        timeline: Timeline,
        time: Timestamp,
        excluding: &[i64],
    ) -> Result<Vec<Self>> {
        let mut filter = String::new();
        let mut values: Vec<rusqlite::types::Value> = Vec::new();
        if !excluding.is_empty() {
            filter = format!(
                r#" AND "messages"."id" NOT IN ({})"#,
                placeholders(excluding.len())
            );
            values.extend(excluding.iter().map(|id| rusqlite::types::Value::from(*id)));
        }
        filter.push_str(" AND (updated_at > ?)");
        values.push(time.to_db().into());
        Self::page(conn, timeline, &filter, values, true)
    }

    /// `before(message).exists?`
    pub fn exists_before(conn: &Connection, timeline: Timeline, message: &Message) -> Result<bool> {
        Self::exists_in(conn, timeline, BEFORE, message)
    }

    /// `after(message).exists?`
    pub fn exists_after(conn: &Connection, timeline: Timeline, message: &Message) -> Result<bool> {
        Self::exists_in(conn, timeline, AFTER, message)
    }

    fn exists_in(
        conn: &Connection,
        timeline: Timeline,
        comparison: &str,
        message: &Message,
    ) -> Result<bool> {
        let (condition, value) = timeline.condition();
        let sql = format!(
            r#"SELECT 1 AS one FROM "messages" WHERE {condition} AND ({comparison}) LIMIT 1"#
        );
        sql::exists(conn, &sql, params![value, message.created_at, message.id])
    }

    /// `paged?`: more than a page (`count > PAGE_SIZE`). Asks whether a row exists past the
    /// first page rather than counting the whole timeline, which grows without bound.
    pub fn paged(conn: &Connection, timeline: Timeline) -> Result<bool> {
        let (condition, value) = timeline.condition();
        let sql =
            format!(r#"SELECT 1 FROM "messages" WHERE {condition} LIMIT 1 OFFSET {PAGE_SIZE}"#);
        sql::exists(conn, &sql, [value])
    }

    // Message::Searchable

    /// Our `SearchQuery` grammar applied to a live room, returned oldest first.
    pub fn search_in_room(conn: &Connection, room_id: i64, query: &str) -> Result<Vec<Self>> {
        super::search_query::SearchQuery::parse(query).messages_in_room(conn, room_id)
    }

    /// The first bounded reachable search window. Subsequent windows use
    /// `SearchQuery::messages_for_user` with an accessible message cursor.
    pub fn search_reachable(conn: &Connection, user_id: i64, query: &str) -> Result<Vec<Self>> {
        Ok(super::search_query::SearchQuery::parse(query)
            .messages_for_user(conn, user_id, jiff::tz::TimeZone::UTC, None)?
            .messages)
    }

    // Creating, updating, destroying

    /// `room.messages.create!` (or `create_with_attachment!` given a blob). Before validation a
    /// Markdown source is rendered into the body (`render_markdown_body`); then the validations,
    /// and inside the transaction: the message, its body (which touches the message), its
    /// attachment and Drive attachments, the room touch, and a thread reply's counter refresh
    /// (`after_create :refresh_thread_messages_count`).
    ///
    /// Rails' index, unread, reference and stale-thread bookkeeping runs here in the same
    /// SQLite transaction as the write, so failure rolls it all back. Broadcasts and job wakes
    /// run after commit in Rails' order: unread, push, then the final thread indicator.
    /// Not ported here, for their owners: agent deliveries
    /// (WS11), activity items (WS12), and the Slack importer's `importing` flag (WS16).
    /// App-owned reference domains register in Env; their failures roll back this write.
    pub fn create(tx: &mut Tx<'_>, attributes: NewMessage) -> Result<Self> {
        Self::create_with_import_time(tx, attributes, None)
    }

    /// Slack's `importing: true` plus `ActiveRecord::Base.no_touching`: keep validation,
    /// rendering, mention resolution, search and DB references; suppress delivery and touches.
    pub fn create_imported(
        tx: &mut Tx<'_>,
        attributes: NewMessage,
        created_at: Timestamp,
        edited_at: Option<Timestamp>,
    ) -> Result<Self> {
        Self::create_with_import_time(tx, attributes, Some((created_at, edited_at)))
    }

    fn create_with_import_time(
        tx: &mut Tx<'_>,
        attributes: NewMessage,
        imported_time: Option<(Timestamp, Option<Timestamp>)>,
    ) -> Result<Self> {
        let importing = imported_time.is_some();
        let body = Self::rendered_body(tx, &attributes)?;
        Self::validate(tx.conn(), &attributes)?.into_result()?;
        let now = tx.now();
        let client_message_id = attributes
            .client_message_id
            .clone()
            .unwrap_or_else(|| tx.env().message_uuid());
        // `before_save :touch_streaming_activity, if: :streaming?`
        let streaming_updated_at = attributes.streaming.then_some(now);
        let id: i64 = tx.conn().query_row_cached(
            r#"INSERT INTO "messages" ("action", "board_post_opener", "client_message_id", "created_at", "creator_id", "embeds_suppressed", "forward_note", "forwarded_at", "forwarded_from_message_id", "forwarded_markdown", "markdown_source", "reply_notify_author", "reply_to_message_id", "room_id", "streaming", "streaming_updated_at", "system_note", "thread_id", "updated_at") VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING "id""#,
            params![
                attributes.action,
                attributes.board_post_opener,
                client_message_id,
                imported_time.map_or(now, |(created, _)| created),
                attributes.creator_id,
                attributes.embeds_suppressed,
                attributes.forward_note,
                attributes.forwarded_at,
                attributes.forwarded_from_message_id,
                attributes.forwarded_markdown,
                attributes.markdown_source,
                attributes.reply_notify_author.unwrap_or(true),
                attributes.reply_to_message_id,
                attributes.room_id,
                attributes.streaming,
                streaming_updated_at,
                attributes.system_note,
                attributes.thread_id,
                now
            ],
            |r| r.get(0),
        )?;
        let mut message = Self::find(tx.conn(), id)?;
        if let Some((_, edited_at)) = imported_time {
            tx.conn().execute_cached(
                "UPDATE messages SET edited_at = ? WHERE id = ?",
                params![edited_at, id],
            )?;
            message.edited_at = edited_at;
        }
        let mut indicator_threads = Vec::new();
        if let Some(thread_id) = message
            .thread_id
            .filter(|_| message.thread_reply() && !importing)
        {
            ChannelThread::refresh_messages_count(tx, thread_id)?;
            indicator_threads.push(thread_id);
        }

        let mut touched = false;
        if let Some(body) = &body {
            RichTextRecord::create(tx, RECORD_TYPE, id, "body", body)?;
            touched = true;
        }
        if let Some(blob_id) = attributes.attachment_blob_id {
            Attachment::create(tx, RECORD_TYPE, id, "attachment", blob_id)?;
            touched = true;
        }
        for file_id in &attributes.drive_file_ids {
            tx.conn().execute_cached(
                r#"INSERT INTO "drive_attachments" ("created_at", "file_id", "message_id") VALUES (?, ?, ?)"#,
                params![tx.now(), file_id, id],
            )?;
            touched = true;
        }
        if touched {
            message.updated_at = Self::touch_row(tx, id)?;
        }
        if !importing {
            Room::touch(tx, message.room_id)?;
        }

        if !message.streaming {
            // Bookkeeping commits with the message (lead decision, round 2). Receive only
            // queues its broadcasts here; push persistence stays in this same transaction.
            message.create_in_index(tx)?;
            if !importing {
                message.receive_in_conversation(tx)?;
                if !message.system_note {
                    crate::ActivityItem::record_message(tx, &message)?;
                    tx.model_callback(crate::callbacks::Phase::MessageActivity, message.id)?;
                }
            }
            if importing {
                crate::models::message_reference::sync(tx, &message)?;
                message.sync_external_references(tx, false)?;
            } else {
                message.sync_all_references(tx)?;
            }
            if !importing {
                message.push_later_in_conversation(tx);
            }
        }
        if message.thread_id.is_some() && !importing {
            // `close_stale_sibling_threads`: with no scheduled sweep, thread writes persist the
            // room's archive state.
            ChannelThread::close_stale_in(tx, Some(message.room_id))?;
        }
        if !importing {
            crate::models::agent_delivery::enqueue_for_message(tx, &message)?;
            // Read the final counter after commit. Rails sends unread, push, then indicator.
            tx.after_commit(move |tx| {
                ChannelThread::broadcast_thread_indicators(tx, &indicator_threads)
            });
        }
        Ok(message)
    }

    /// Network-card owners plug into the real create/edit callbacks through the app sink.
    /// WS11 calls this only after deciding a finalized stream may fan out; a quiet finalize
    /// must not warm previews. Import callers pass false to retain DB references without fetches.
    pub fn sync_external_references(&self, tx: &mut Tx<'_>, enqueue: bool) -> Result<()> {
        use crate::callbacks::Phase;
        for sync in tx.env().message_reference_syncs.clone() { sync(tx,self,enqueue)?; }
        let sink=tx.env().sink.clone();
        for phase in [Phase::MessageFizzyReferences,Phase::MessageTwitterReferences,Phase::MessageEventReferences,Phase::MessageLinkReferences] {
            if phase == Phase::MessageEventReferences { crate::models::calendar_event::references::sync(tx, self)?; }
            sink.sync_message_reference_phase(tx,self,phase,enqueue)?;

        }
        Ok(())
    }

    /// RoomMailbox's Markdown entry point; all validation, rendering and callbacks use `create`.
    pub fn create_markdown(
        tx: &mut Tx<'_>,
        mut attributes: NewMessage,
        source: &str,
    ) -> Result<Self> {
        attributes.markdown_source = Some(source.to_owned());
        Self::create(tx, attributes)
    }

    /// `render_markdown_body`: the body a Markdown source renders to, when it's within the
    /// limit (a longer one fails validation instead); otherwise the body given.
    fn rendered_body(tx: &Tx<'_>, attributes: &NewMessage) -> Result<Option<String>> {
        match &attributes.markdown_source {
            Some(source) if source.chars().count() <= SOURCE_LIMIT => {
                let rendered = tx
                    .rich_text()
                    .render_markdown(tx.conn(), source, attributes.room_id)
                    .map_err(crate::error::Error::Other)?;
                // Assigning Markdown to Rails' Action Text body canonicalizes before save.
                Ok(Some(Self::prepare_body(tx, &rendered, true)?))
            }
            Some(_) => Ok(None),
            None => attributes
                .body
                .as_deref()
                .map(|body| Self::prepare_body(tx, body, false))
                .transpose(),
        }
    }

    fn prepare_body(tx: &Tx<'_>, body: &str, markdown: bool) -> Result<String> {
        let text = tx.rich_text();
        let body = text
            .try_canonicalize_html(tx.conn(), body)
            .map_err(crate::Error::Other)?;
        // Resolve before committing: index/receive callbacks must not hide renderer failures
        // after the originating row has already committed.
        let names = |id| {
            User::find_by_id(tx.conn(), id)
                .ok()
                .flatten()
                .map(|u| u.name)
        };
        if markdown {
            text.try_markdown_plain_text(tx.conn(), &body, &names)
                .map_err(crate::Error::Other)?;
        } else {
            text.try_to_plain_text(tx.conn(), &body, &names)
                .map_err(crate::Error::Other)?;
        }
        text.try_mentioned_user_ids(tx.conn(), &body)
            .map_err(crate::Error::Other)?;
        Ok(body)
    }

    /// The validations of `app/models/message.rb` for a new message.
    pub fn validate(conn: &Connection, attributes: &NewMessage) -> Result<Errors> {
        Self::validate_attributes(conn, attributes, true)
    }

    /// The validations, for a new record or (`new_record` false) a saved one being updated to
    /// `attributes`.
    fn validate_attributes(
        conn: &Connection,
        attributes: &NewMessage,
        new_record: bool,
    ) -> Result<Errors> {
        let mut errors = Errors::default();
        // `belongs_to :room` and `:creator` (required)
        if Room::find_by_id(conn, attributes.room_id)?.is_none() {
            errors.add("room", "must exist");
        }
        if User::find_by_id(conn, attributes.creator_id)?.is_none() {
            errors.add("creator", "must exist");
        }
        // `DriveAttachment`'s own validations, through the autosaved association (declared, so
        // run, before the message's). Duplicates within one message are left to the unique index,
        // as Rails' uniqueness check only sees saved rows.
        for file_id in &attributes.drive_file_ids {
            if file_id.trim().is_empty() {
                errors.add("drive_attachments.file_id", "can't be blank");
            }
            if !valid_drive_file_id(file_id) {
                errors.add("drive_attachments.file_id", "is invalid");
            }
        }
        // `validates :markdown_source, length: { maximum: SOURCE_LIMIT }`
        if let Some(source) = &attributes.markdown_source
            && source.chars().count() > SOURCE_LIMIT
        {
            errors.add(
                "markdown_source",
                format!("is too long (maximum is {SOURCE_LIMIT} characters)"),
            );
        }
        // `markdown_source_or_attachment`, if `requires_body?` (Markdown, not streaming)
        if let Some(source) = &attributes.markdown_source
            && !attributes.streaming
            && source.trim().is_empty()
            && attributes.attachment_blob_id.is_none()
            && attributes.drive_file_ids.is_empty()
        {
            errors.add("markdown_source", "can't be blank");
        }
        // `drive_attachments_within_limit`
        if attributes.drive_file_ids.len() > DRIVE_ATTACHMENTS_PER_MESSAGE {
            errors.add(
                "drive_attachments",
                format!("are limited to {DRIVE_ATTACHMENTS_PER_MESSAGE} per message"),
            );
        }
        // `validate_conversation_links`
        let thread = match attributes.thread_id {
            Some(thread_id) => ChannelThread::find_by_id(conn, thread_id)?,
            None => None,
        };
        if let Some(thread) = &thread {
            let direct = Room::find_by_id(conn, thread.room_id)?.is_some_and(|room| room.direct());
            if thread.room_id != attributes.room_id || direct {
                errors.add(
                    "thread",
                    "must belong to the message room and cannot be a direct room thread",
                );
            }
        }
        if let Some(source) = attributes
            .reply_to_message_id
            .map(|id| Self::find_by_id(conn, id))
            .transpose()?
            .flatten()
        {
            let same_stream = match attributes.thread_id {
                None => source.thread_id.is_none(),
                Some(thread_id) => {
                    source.thread_id == Some(thread_id)
                        || thread.as_ref().is_some_and(|thread| {
                            thread.parent_message_id == Some(source.id)
                                && source.thread_id.is_none()
                        })
                }
            };
            if source.room_id != attributes.room_id || !same_stream {
                errors.add("reply_to_message", "must be in the same conversation");
            }
        }
        // `validate_forward_metadata` (new records only: deleting the source nullifies the link
        // and leaves `forwarded_at`)
        match (
            attributes.forwarded_from_message_id,
            attributes.forwarded_at,
        ) {
            (Some(_), None) if new_record => {
                errors.add("forwarded_at", "must be present for a forwarded message")
            }
            (None, Some(_)) if new_record => errors.add(
                "forwarded_from_message",
                "must be present for a forwarded message",
            ),
            _ => {}
        }
        // `no_root_messages_in_boards`: quiet system notes (the stale-work digest) may sit at a
        // board's root; chat may not.
        if attributes.thread_id.is_none() && !attributes.system_note {
            let room_type = query_one(
                conn,
                r#"SELECT "rooms"."type" FROM "rooms" WHERE "rooms"."id" = ?"#,
                [attributes.room_id],
                |r| r.get::<_, RoomType>(0),
            )?;
            if room_type == Some(RoomType::Board) {
                errors.add("thread", "must be present in a board");
            }
        }
        // `validates :forward_note, length: { maximum: SOURCE_LIMIT }`
        if let Some(note) = &attributes.forward_note
            && note.chars().count() > SOURCE_LIMIT
        {
            errors.add(
                "forward_note",
                format!("is too long (maximum is {SOURCE_LIMIT} characters)"),
            );
        }
        Ok(errors)
    }

    /// `thread_reply?`: what `ChannelThread#messages_count` counts.
    pub fn thread_reply(&self) -> bool {
        self.thread_id.is_some() && !self.system_note && !self.streaming
    }

    /// Rails validates the saved false -> true transition on every update.
    /// Compare the persisted row, so a stale or manually edited model cannot
    /// bypass the stream's irreversible finalization claim.
    fn validate_streaming_state(&self, conn: &Connection) -> Result<()> {
        if self.streaming && !Self::find(conn, self.id)?.streaming {
            let mut errors = Errors::default();
            errors.add("streaming", "cannot resume once finalized");
            return errors.into_result();
        }
        Ok(())
    }

    /// `before_save :touch_streaming_activity, if: :streaming?`: every save while streaming
    /// restarts the finalize sweep's inactivity clock. That's a saved change, so it touches the
    /// room too (`belongs_to :room, touch: true`), even when nothing else changed. (`touch` isn't
    /// a save, so boosts don't restart the clock.) `save_touches_test` holds each save path to
    /// Rails' answer.
    fn touch_streaming_activity(&mut self, tx: &mut Tx<'_>) -> Result<()> {
        self.validate_streaming_state(tx.conn())?;
        if !self.streaming {
            return Ok(());
        }
        let now = tx.now();
        tx.conn().execute_cached(
            r#"UPDATE "messages" SET "streaming_updated_at" = ?, "updated_at" = ? WHERE "messages"."id" = ?"#,
            params![now, now, self.id],
        )?;
        self.streaming_updated_at = Some(now);
        self.updated_at = now;
        Room::touch(tx, self.room_id)
    }

    /// `message.update!(body:)`: the body changes, which touches the message and its room;
    /// the search index follows after commit. A streaming message is saved even when the body
    /// is unchanged, which restarts its activity clock and touches the room.
    pub fn update_body(&mut self, tx: &mut Tx<'_>, body: &str) -> Result<()> {
        let body = Self::prepare_body(tx, body, self.markdown())?;
        let body = body.as_str();
        self.touch_streaming_activity(tx)?;
        match RichTextRecord::find_for(tx.conn(), RECORD_TYPE, self.id, "body")? {
            Some(record) if record.body.as_deref() == Some(body) => return Ok(()),
            Some(mut record) => record.update_body(tx, body)?,
            None => {
                RichTextRecord::create(tx, RECORD_TYPE, self.id, "body", body)?;
            }
        }
        self.touch(tx)?;
        if !self.streaming {
            self.sync_external_references(tx, true)?;
        }
        Ok(())
    }

    /// The edit endpoints' save (`MessagesController#update`,
    /// `ChannelThreadMessagesController#update`): assign `changes`, stamp `edited_at` only when
    /// the text itself changes (`body_content_will_change?`), then save. Reactions, tombstones,
    /// card fetches, attachment-only and identical saves never mark a message "(edited)". The
    /// controllers' presentation and card broadcasts are WS8b's.
    pub fn edit(&mut self, tx: &mut Tx<'_>, changes: MessageChanges) -> Result<()> {
        self.save_changes(tx, changes, true)
    }

    /// `update!(...)`: a save that never stamps `edited_at`.
    pub fn update(&mut self, tx: &mut Tx<'_>, changes: MessageChanges) -> Result<()> {
        self.save_changes(tx, changes, false)
    }

    /// `MessageEmbedSuppressionsController#create`: `update!(embeds_suppressed: true) unless
    /// embeds_suppressed?`.
    pub fn suppress_embeds(&mut self, tx: &mut Tx<'_>) -> Result<()> {
        if self.embeds_suppressed {
            return Ok(());
        }
        self.update(
            tx,
            MessageChanges {
                embeds_suppressed: Some(true),
                ..Default::default()
            },
        )
    }

    /// `body_content_will_change?`: a new Markdown source, or a body whose HTML changes when
    /// either side has any text (a blank body assigned to an attachment-only message isn't a
    /// text change).
    pub fn body_content_will_change(
        &self,
        conn: &Connection,
        rich_text: &dyn RichText,
        changes: &MessageChanges,
    ) -> Result<bool> {
        if self.markdown_source_will_change(changes) {
            return Ok(true);
        }
        let Some(body) = &changes.body else {
            return Ok(false);
        };
        let previous = self.body_html(conn)?.unwrap_or_default();
        if *body == previous {
            return Ok(false);
        }
        let names = |id| User::find_by_id(conn, id).ok().flatten().map(|u| u.name);
        let has_text = |html: &str| {
            rich_text
                .try_to_plain_text(conn, html, &names)
                .map(|s| !s.trim().is_empty())
                .map_err(crate::Error::Other)
        };
        Ok(has_text(body)? || has_text(&previous)?)
    }

    fn markdown_source_will_change(&self, changes: &MessageChanges) -> bool {
        (changes.clear_markdown_source && self.markdown_source.is_some())
            || changes
                .markdown_source
                .as_ref()
                .is_some_and(|source| self.markdown_source.as_ref() != Some(source))
    }

    /// `save!` of assigned changes. A changed Markdown source re-renders the body
    /// (`render_markdown_body`); the validations run as for an update; the message and its room
    /// are touched when anything changed (or always while streaming); the search index follows
    /// in this transaction (Rails' `after_update_commit :update_in_index` is moved here so a
    /// failed renderer cannot leave a partially processed write).
    fn save_changes(
        &mut self,
        tx: &mut Tx<'_>,
        changes: MessageChanges,
        stamp_edited: bool,
    ) -> Result<()> {
        self.validate_streaming_state(tx.conn())?;
        let conn = tx.conn();
        let content_changes = stamp_edited && self.body_content_will_change(conn, tx.rich_text(), &changes)?;
        let markdown_source = if changes.clear_markdown_source { None }
            else if self.markdown_source_will_change(&changes) { changes.markdown_source.clone() }
            else { self.markdown_source.clone() };
        let body = match &changes.markdown_source {
            Some(source) if self.markdown_source_will_change(&changes) => {
                if source.chars().count() <= SOURCE_LIMIT {
                    let rendered = tx.rich_text().render_markdown(conn, source, self.room_id).map_err(crate::error::Error::Other)?;
                    Some([Some(rendered), changes.legacy_attachment_snapshot.clone()].into_iter().flatten()
                        .filter(|body| !body.chars().all(char::is_whitespace)).collect::<Vec<_>>().join("\n"))
                } else {
                    None
                }
            }
            _ => changes.body.clone(),
        };
        let body = body
            .as_deref()
            .map(|body| Self::prepare_body(tx, body, markdown_source.is_some()))
            .transpose()?;
        let current_body = self.body_html(conn)?;
        let body = body.filter(|body| current_body.as_deref() != Some(body.as_str()));
        let references_changed =
            !self.streaming && (markdown_source != self.markdown_source || body.is_some());
        let current_drive_ids = drive_file_ids(conn, self.id)?;
        let drive_file_ids = changes
            .drive_file_ids
            .clone()
            .unwrap_or_else(|| current_drive_ids.clone());
        let forward_note = changes
            .forward_note
            .clone()
            .unwrap_or_else(|| self.forward_note.clone());
        let embeds_suppressed = changes.embeds_suppressed.unwrap_or(self.embeds_suppressed);
        let client_message_id = changes.client_message_id.clone().unwrap_or_else(|| Some(self.client_message_id.clone()));
        let reply_to_message_id = changes.reply_to_message_id.unwrap_or(self.reply_to_message_id);
        let reply_notify_author = changes.reply_notify_author.unwrap_or(self.reply_notify_author);

        let attributes = NewMessage {
            room_id: self.room_id,
            creator_id: self.creator_id,
            client_message_id: client_message_id.clone(),
            body: body.clone(),
            attachment_blob_id: Attachment::find_for(conn, RECORD_TYPE, self.id, "attachment")?
                .map(|a| a.blob_id),
            thread_id: self.thread_id,
            system_note: self.system_note,
            streaming: self.streaming,
            markdown_source: markdown_source.clone(),
            action: self.action,
            board_post_opener: self.board_post_opener,
            embeds_suppressed,
            reply_to_message_id,
            reply_notify_author: Some(reply_notify_author),
            forwarded_from_message_id: self.forwarded_from_message_id,
            forwarded_at: self.forwarded_at,
            forward_note: forward_note.clone(),
            forwarded_markdown: self.forwarded_markdown,
            drive_file_ids: drive_file_ids.clone(),
        };
        Self::validate_attributes(conn, &attributes, false)?.into_result()?;

        let now = tx.now();
        let edited_at = if content_changes {
            Some(now)
        } else {
            self.edited_at
        };
        let columns_changed = markdown_source != self.markdown_source
            || client_message_id.as_deref() != Some(self.client_message_id.as_str())
            || reply_to_message_id != self.reply_to_message_id
            || reply_notify_author != self.reply_notify_author
            || forward_note != self.forward_note
            || embeds_suppressed != self.embeds_suppressed
            || edited_at != self.edited_at;
        let drive_changed = drive_file_ids != current_drive_ids;
        if !(columns_changed || body.is_some() || drive_changed || self.streaming) {
            return Ok(());
        }
        let streaming_updated_at = if self.streaming {
            Some(now)
        } else {
            self.streaming_updated_at
        };
        tx.conn().execute_cached(
            r#"UPDATE "messages" SET "markdown_source" = ?, "client_message_id" = ?, "reply_to_message_id" = ?, "reply_notify_author" = ?, "forward_note" = ?, "embeds_suppressed" = ?, "edited_at" = ?, "streaming_updated_at" = ?, "updated_at" = ? WHERE "messages"."id" = ?"#,
            params![markdown_source, client_message_id, reply_to_message_id, reply_notify_author, forward_note, embeds_suppressed, edited_at, streaming_updated_at, now, self.id],
        )?;
        if let Some(body) = &body {
            match RichTextRecord::find_for(tx.conn(), RECORD_TYPE, self.id, "body")? {
                Some(mut record) => record.update_body(tx, body)?,
                None => {
                    RichTextRecord::create(tx, RECORD_TYPE, self.id, "body", body)?;
                }
            }
        }
        if drive_changed {
            for file_id in current_drive_ids
                .iter()
                .filter(|id| !drive_file_ids.contains(id))
            {
                tx.conn().execute_cached(
                    r#"DELETE FROM "drive_attachments" WHERE "drive_attachments"."message_id" = ? AND "drive_attachments"."file_id" = ?"#,
                    params![self.id, file_id],
                )?;
            }
            for file_id in drive_file_ids
                .iter()
                .filter(|id| !current_drive_ids.contains(id))
            {
                tx.conn().execute_cached(
                    r#"INSERT INTO "drive_attachments" ("created_at", "file_id", "message_id") VALUES (?, ?, ?)"#,
                    params![now, file_id, self.id],
                )?;
            }
        }
        Room::touch(tx, self.room_id)?;
        self.reload(tx.conn())?;
        if references_changed {
            tx.emit_after_commit(Event::job(
                &crate::models::message_reference::QuoteCardsRefreshJob {
                    source_message_id: self.id,
                },
            ));
        }
        if !self.streaming {
            self.update_in_index(tx)?;
            if references_changed { self.sync_all_references(tx)?; }
        }
        Ok(())
    }

    /// Compatibility entry point for import callers; both owners use the real save hook.
    pub fn sync_integration_references(
        &self,
        tx: &mut Tx<'_>,
        enqueue_fetches: bool,
    ) -> Result<()> {
        self.sync_external_references(tx, enqueue_fetches)
    }

    /// `drive_attachments.map(&:file_id)`, in id order.
    pub fn drive_file_ids(&self, conn: &Connection) -> Result<Vec<String>> {
        drive_file_ids(conn, self.id)
    }

    /// `touch` (from a boost, or the body): the message, its room and its index atomically.
    pub fn touch(&mut self, tx: &mut Tx<'_>) -> Result<()> {
        self.updated_at = Self::touch_row(tx, self.id)?;
        Room::touch(tx, self.room_id)?;
        if !self.streaming {
            self.update_in_index(tx)?;
        }
        Ok(())
    }

    fn touch_row(tx: &Tx<'_>, id: i64) -> Result<Timestamp> {
        let now = tx.now();
        tx.conn().execute_cached(
            r#"UPDATE "messages" SET "updated_at" = ? WHERE "messages"."id" = ?"#,
            params![now, id],
        )?;
        Ok(now)
    }

    /// `update!(attachment: blob or nil)`: `has_one_attached` replaces the attachment. The old
    /// one is destroyed (its blob purged after commit, `dependent: :purge_later`) and each
    /// attachment change touches the message (`belongs_to :record, touch: true`), and so its room.
    /// A streaming message saves even with no attachment change (`touch_streaming_activity`).
    pub fn replace_attachment(&mut self, tx: &mut Tx<'_>, blob_id: Option<i64>) -> Result<()> {
        self.touch_streaming_activity(tx)?;
        if let Some(attachment) =
            Attachment::find_for(tx.conn(), RECORD_TYPE, self.id, "attachment")?
        {
            if Some(attachment.blob_id) == blob_id {
                return Ok(());
            }
            attachment.delete(tx)?;
            tx.emit_after_commit(Event::PurgeBlob {
                blob_id: attachment.blob_id,
            });
            self.touch(tx)?;
        }
        if let Some(blob_id) = blob_id {
            Attachment::create(tx, RECORD_TYPE, self.id, "attachment", blob_id)?;
            self.touch(tx)?;
        }
        Ok(())
    }

    /// `destroy`: `capture_quote_referencing_ids` and `preserve_reply_tombstones` (both
    /// prepended), then the dependents in declaration order: its attachment (blob purged later),
    /// boosts, poll, pins, saved items, board stale digests (nullified), activity items, the
    /// reference rows of every kind, Drive attachments, agent steps and the body; then the
    /// message, and the room is touched. Its replies, forwards and thread lose their link through
    /// the foreign keys (`ON DELETE SET NULL`), as `dependent: :nullify` would. A thread reply
    /// refreshes its thread's counter unless the room is being torn down
    /// (`destroyed_with_conversation?`).
    ///
    /// The index entry and quoting-message timestamps change in this transaction. After
    /// commit their quote cards are replaced and the thread indicator is re-broadcast.
    pub fn destroy(&self, tx: &mut Tx<'_>) -> Result<()> {
        let with_conversation = Room::find(tx.conn(), self.room_id)?.deleted();
        self.destroy_inner(tx, with_conversation, false)
    }

    /// `destroy` from the thread's `dependent: :destroy`: the thread's counter row goes with it,
    /// so no recount.
    pub(crate) fn destroy_with_conversation(&self, tx: &mut Tx<'_>) -> Result<()> {
        self.destroy_inner(tx, true, false)
    }

    /// `message.importing = true; message.destroy!`: retain dependency cleanup,
    /// tombstones, the counter and search removal, without quote or thread broadcasts.
    pub fn destroy_imported(&self, tx: &mut Tx<'_>) -> Result<()> {
        let with_conversation = Room::find(tx.conn(), self.room_id)?.deleted();
        self.destroy_inner(tx, with_conversation, true)
    }

    pub(crate) fn destroy_imported_with_conversation(&self, tx: &mut Tx<'_>) -> Result<()> {
        self.destroy_inner(tx, true, true)
    }

    fn destroy_inner(
        &self,
        tx: &mut Tx<'_>,
        with_conversation: bool,
        importing: bool,
    ) -> Result<()> {
        let quoting_ids = crate::models::message_reference::incoming_ids(tx.conn(), self.id)?;
        let now = tx.now();
        tx.conn().execute_cached(
            r#"UPDATE "messages" SET "reply_to_message_id" = NULL, "reply_target_deleted_at" = ?, "updated_at" = ? WHERE "messages"."reply_to_message_id" = ?"#,
            params![now, now, self.id],
        )?;
        if let Some(attachment) =
            Attachment::find_for(tx.conn(), RECORD_TYPE, self.id, "attachment")?
        {
            attachment.delete(tx)?;
            tx.emit_after_commit(Event::PurgeBlob {
                blob_id: attachment.blob_id,
            });
        }
        for boost in Boost::for_message(tx.conn(), self.id)? {
            boost.delete_row(tx)?;
        }
        crate::models::Poll::destroy_for_message(tx, self.id)?;
        crate::models::MessagePin::destroy_for_message(tx, self)?;
        crate::models::SavedItem::destroy_for_message(tx, self.id)?;
        tx.conn().execute_cached(
            r#"UPDATE "board_stale_digests" SET "message_id" = NULL WHERE "board_stale_digests"."message_id" = ?"#,
            [self.id],
        )?;
        tx.conn().execute_cached(
            r#"DELETE FROM "activity_items" WHERE "activity_items"."source_type" = 'Message' AND "activity_items"."source_id" = ?"#,
            [self.id],
        )?;
        // The reference rows (none has destroy callbacks of its own), Drive attachments and
        // agent steps, in declaration order.
        for sql in [
            r#"DELETE FROM "github_pull_request_references" WHERE "message_id" = ?"#,
            r#"DELETE FROM "fizzy_card_references" WHERE "message_id" = ?"#,
            r#"DELETE FROM "twitter_post_references" WHERE "message_id" = ?"#,
            r#"DELETE FROM "event_references" WHERE "message_id" = ?"#,
            r#"DELETE FROM "message_references" WHERE "message_id" = ?"#,
            r#"DELETE FROM "message_references" WHERE "referenced_message_id" = ?"#,
            r#"DELETE FROM "link_embed_references" WHERE "message_id" = ?"#,
            r#"DELETE FROM "drive_attachments" WHERE "message_id" = ?"#,
            r#"DELETE FROM "agent_steps" WHERE "message_id" = ?"#,
        ] {
            tx.conn().execute_cached(sql, [self.id])?;
        }
        if let Some(body) = RichTextRecord::find_for(tx.conn(), RECORD_TYPE, self.id, "body")? {
            body.delete(tx)?;
        }
        tx.conn().execute_cached(
            r#"DELETE FROM "messages" WHERE "messages"."id" = ?"#,
            [self.id],
        )?;
        let mut indicator_threads = Vec::new();
        if let Some(thread_id) = self
            .thread_id
            .filter(|_| self.thread_reply() && !with_conversation)
        {
            ChannelThread::refresh_messages_count(tx, thread_id)?;
            indicator_threads.push(thread_id);
        }
        Room::touch(tx, self.room_id)?;
        remove_from_index(tx, self.id)?;
        if !importing {
            crate::models::message_reference::removed_source(tx, quoting_ids)?;
            tx.after_commit(move |tx| {
                ChannelThread::broadcast_thread_indicators(tx, &indicator_threads)
            });
        }
        Ok(())
    }

    /// `receive_in_conversation`: the thread's, or the room's.
    fn receive_in_conversation(&self, tx: &mut Tx<'_>) -> Result<()> {
        match self.thread_id {
            Some(thread_id) => ChannelThread::receive(tx, thread_id, self),
            None => Room::receive(tx, self.room_id, self),
        }
    }

    /// The push job `receive_in_conversation` enqueues, written in the message's transaction
    /// (see [`Room::push_later`]).
    fn push_later_in_conversation(&self, tx: &mut Tx<'_>) {
        match self.thread_id {
            Some(thread_id) => ChannelThread::push_later(tx, thread_id, self),
            None => Room::push_later(tx, self.room_id, self),
        }
    }

    /// `create_in_index`: never for a system note.
    fn create_in_index(&self, tx: &Tx<'_>) -> Result<()> {
        if self.system_note {
            return Ok(());
        }
        let body = self.plain_text_body(tx.conn(), tx.rich_text())?;
        tx.conn().execute_cached(
            "insert into message_search_index(rowid, body) values (?, ?)",
            params![self.id, body],
        )?;
        Ok(())
    }

    fn update_in_index(&self, tx: &Tx<'_>) -> Result<()> {
        let body = self.plain_text_body(tx.conn(), tx.rich_text())?;
        tx.conn().execute_cached(
            "update message_search_index set body = ? where rowid = ?",
            params![body, self.id],
        )?;
        Ok(())
    }

    // Attributes and associations

    pub fn room(&self, conn: &Connection) -> Result<Room> {
        Room::find(conn, self.room_id)
    }

    pub fn creator(&self, conn: &Connection) -> Result<User> {
        User::find(conn, self.creator_id)
    }

    pub fn boosts(&self, conn: &Connection) -> Result<Vec<Boost>> {
        Boost::for_message_ordered(conn, self.id)
    }

    pub fn body(&self, conn: &Connection) -> Result<Option<RichTextRecord>> {
        RichTextRecord::find_for(conn, RECORD_TYPE, self.id, "body")
    }

    /// `message.body.body` as stored HTML, if any.
    pub fn body_html(&self, conn: &Connection) -> Result<Option<String>> {
        Ok(self.body(conn)?.and_then(|b| b.body))
    }

    /// The attachment and its blob, if attached.
    pub fn attachment(&self, conn: &Connection) -> Result<Option<(Attachment, Blob)>> {
        match Attachment::find_for(conn, RECORD_TYPE, self.id, "attachment")? {
            Some(attachment) => {
                let blob = attachment.blob(conn)?;
                Ok(Some((attachment, blob)))
            }
            None => Ok(None),
        }
    }

    /// `plain_text_body`: the body's plain text (`Markdown.plain_text` for a Markdown message),
    /// else the attachment's filename, else ""; a forward note goes first, a blank line between.
    pub fn plain_text_body(&self, conn: &Connection, rich_text: &dyn RichText) -> Result<String> {
        let mut text = String::new();
        if let Some(html) = self.body_html(conn)? {
            let names = |id| User::find_by_id(conn, id).ok().flatten().map(|u| u.name);
            text = if self.markdown() {
                rich_text
                    .try_markdown_plain_text(conn, &html, &names)
                    .map_err(crate::Error::Other)?
            } else {
                rich_text
                    .try_to_plain_text(conn, &html, &names)
                    .map_err(crate::Error::Other)?
            };
        }
        if text.trim().is_empty() {
            text = self
                .attachment(conn)?
                .map(|(_, blob)| blob.filename)
                .unwrap_or_default();
        }
        Ok(
            match self
                .forward_note
                .as_deref()
                .filter(|note| !note.trim().is_empty())
            {
                Some(note) if text.trim().is_empty() => note.to_string(),
                Some(note) => format!("{note}\n\n{text}"),
                None => text,
            },
        )
    }

    /// `markdown?`
    pub fn markdown(&self) -> bool {
        self.markdown_source.is_some()
    }

    /// `thread_message?`
    pub fn thread_message(&self) -> bool {
        self.thread_id.is_some()
    }

    /// `reply?`: a reply, or the tombstone of one whose source was deleted.
    pub fn reply(&self) -> bool {
        self.reply_to_message_id.is_some() || self.reply_target_deleted_at.is_some()
    }

    /// `forwarded?`
    pub fn forwarded(&self) -> bool {
        self.forwarded_at.is_some()
    }

    /// `content_type`
    pub fn content_type(&self, conn: &Connection, rich_text: &dyn RichText) -> Result<ContentType> {
        if self.attachment(conn)?.is_some() {
            Ok(ContentType::Attachment)
        } else if self.sound(conn, rich_text)?.is_some() {
            Ok(ContentType::Sound)
        } else {
            Ok(ContentType::Text)
        }
    }

    /// `sound`: a body of exactly `/play <name>` naming a built-in sound.
    pub fn sound(
        &self,
        conn: &Connection,
        rich_text: &dyn RichText,
    ) -> Result<Option<&'static Sound>> {
        Ok(sound_in(&self.plain_text_body(conn, rich_text)?))
    }

    /// `mentionees`: mentioned users who are members of the room. A forward's body is a
    /// snapshot whose mentions never notify anyone; only its note's `@[Name]` tokens do, each
    /// naming exactly one active member of the destination room (`forward_note_mentionees`).
    pub fn mentionees(&self, conn: &Connection, rich_text: &dyn RichText) -> Result<Vec<User>> {
        if self.forwarded() {
            return forward_note_mentionees(
                conn,
                self.room_id,
                self.forward_note.as_deref().unwrap_or(""),
            );
        }
        let ids = match self.body_html(conn)? {
            Some(html) => rich_text
                .try_mentioned_user_ids(conn, &html)
                .map_err(crate::Error::Other)?,
            None => Vec::new(),
        };
        mentionees_in_room(conn, self.room_id, &ids)
    }

    /// `Message.find_duplicate(room:, creator:, client_message_id:)`: the message this user
    /// already posted in the room with that client id, so a retried create returns it rather
    /// than posting twice. (No unique index backs it: production holds duplicates.)
    pub fn find_duplicate(
        conn: &Connection,
        room_id: i64,
        creator_id: i64,
        client_message_id: &str,
    ) -> Result<Option<Self>> {
        if client_message_id.trim().is_empty() {
            return Ok(None);
        }
        query_one(
            conn,
            r#"SELECT "messages".* FROM "messages" WHERE "messages"."room_id" = ? AND "messages"."creator_id" = ? AND "messages"."client_message_id" = ? LIMIT 1"#,
            params![room_id, creator_id, client_message_id],
            Self::from_row,
        )
    }

    /// `thread.messages`: the thread's messages, `ordered`.
    pub fn in_thread(conn: &Connection, thread_id: i64) -> Result<Vec<Self>> {
        query_all(
            conn,
            &format!(
                r#"SELECT "messages".* FROM "messages" WHERE "messages"."thread_id" = ? {ORDERED}"#
            ),
            [thread_id],
            Self::from_row,
        )
    }

    /// `claim_stream_finalized!`: flips `streaming` off with a conditional claim, so a finalize
    /// racing another (the overdue sweep, a retry) wins once; the loser gets false. A finished
    /// thread stream joins its thread's reply count, and the indicator is broadcast after commit.
    /// The rest of `finalize_stream!` (the deferred side effects) is WS11's.
    pub fn claim_stream_finalized(&mut self, tx: &mut Tx<'_>) -> Result<bool> {
        let claimed = self.claim_stream_finalized_without_indicator(tx)?;
        if claimed
            && self.thread_reply()
            && let Some(thread_id) = self.thread_id
        {
            tx.after_commit(move |tx| ChannelThread::broadcast_thread_indicators(tx, &[thread_id]));
        }
        Ok(claimed)
    }

    pub(crate) fn claim_stream_finalized_without_indicator(
        &mut self,
        tx: &mut Tx<'_>,
    ) -> Result<bool> {
        let now = tx.now();
        let claimed = tx.conn().execute_cached(
            r#"UPDATE "messages" SET "streaming" = 0, "updated_at" = ? WHERE "messages"."id" = ? AND "messages"."streaming" = 1"#,
            params![now, self.id],
        )? == 1;
        if claimed {
            self.reload(tx.conn())?;
            if let Some(thread_id) = self.thread_id.filter(|_| self.thread_reply()) {
                ChannelThread::refresh_messages_count(tx, thread_id)?;
            }
        }
        Ok(claimed)
    }

    /// `sync_all_references`, in the Rails declaration order. Peer adapters
    /// implement their import/fetch policy; no network I/O runs here.
    pub fn sync_all_references(&self, tx: &mut Tx<'_>) -> Result<()> {
        use crate::callbacks::Phase;
        for phase in [Phase::MessageGithubReferences, Phase::MessageFizzyReferences,
            Phase::MessageTwitterReferences, Phase::MessageEventReferences] {
            self.sync_reference_phase(tx, phase, true)?;
        }
        crate::models::message_reference::sync(tx,self)?;
        self.sync_reference_phase(tx, Phase::MessageLinkReferences, true)
    }

    fn sync_reference_phase(&self, tx: &mut Tx<'_>, phase: crate::callbacks::Phase, enqueue: bool) -> Result<()> {
        tx.model_callback(phase, self.id)?;
        if phase == crate::callbacks::Phase::MessageGithubReferences {
            for sync in tx.env().message_reference_syncs.clone() { sync(tx, self, enqueue)?; }
        }
        if phase == crate::callbacks::Phase::MessageEventReferences {
            crate::models::calendar_event::references::sync(tx, self)?;
        }
        let sink = tx.env().sink.clone();
        sink.sync_message_reference_phase(tx, self, phase, enqueue)
    }

    /// Claim first, like Rails' `update_all`, then run the deferred callbacks.
    /// A callback exception preserves the claim and earlier effects. The writer
    /// returns it after commit; a durable enqueue error still rolls the entire
    /// write back through Tx::persist_error (the fixed queue-atomicity decision).
    pub fn finalize_stream(&mut self, tx: &mut Tx<'_>) -> Result<bool> {
        if !self.claim_stream_finalized_without_indicator(tx)? { return Ok(false); }
        let effects = (|| {
            let agent = crate::Agent::for_user(tx.conn(), self.creator_id)?;
            if !agent.as_ref().map(|a| a.active(tx.conn())).transpose()?.unwrap_or(false) {
                return self.finalize_claimed_stream_quietly(tx);
            }
            self.create_in_index(tx)?;
            self.receive_in_conversation(tx)?;
            self.push_later_in_conversation(tx);
            if !self.system_note { crate::ActivityItem::record_message(tx, self)?; tx.model_callback(crate::callbacks::Phase::MessageActivity, self.id)?; }
            crate::models::agent_delivery::enqueue_for_message(tx,self)?;
            self.sync_all_references(tx)?;
            crate::models::bot_webhook_fanout::deliver(tx,self)?;
            if self.thread_id.is_none() && !self.system_note {crate::models::agent_posting::broadcast_unread_room(tx,self)?;}
            if let Some(mut agent) = crate::Agent::for_user(tx.conn(),self.creator_id)? {agent.clear_working_presence(tx)?;}
            self.broadcast_finalized_thread_indicator(tx);
            crate::models::agent_streaming::broadcast_final(tx,self)
        })();
        if let Err(error) = effects { tx.after_commit(move |_| Err(error)); }
        Ok(true)
    }

    /// Quiet finalize is used by suspension even in locked threads. It claims
    /// once and broadcasts the final draft without indexing, receive or fanout.
    pub fn finalize_stream_quietly(&mut self, tx: &mut Tx<'_>) -> Result<bool> {
        if !self.claim_stream_finalized_without_indicator(tx)? { return Ok(false); }
        if let Err(error) = self.finalize_claimed_stream_quietly(tx) { tx.after_commit(move |_| Err(error)); }
        Ok(true)
    }
    fn finalize_claimed_stream_quietly(&self, tx: &mut Tx<'_>) -> Result<()> {
        if let Some(mut agent) = crate::Agent::for_user(tx.conn(),self.creator_id)? {agent.clear_working_presence(tx)?;}
        self.broadcast_finalized_thread_indicator(tx);
        crate::models::agent_streaming::broadcast_final(tx,self)
    }
    fn broadcast_finalized_thread_indicator(&self, tx: &mut Tx<'_>) {
        if self.thread_reply() && let Some(thread_id)=self.thread_id {
            tx.after_commit(move |tx| ChannelThread::broadcast_thread_indicators(tx, &[thread_id]));
        }
    }

    pub fn reload(&mut self, conn: &Connection) -> Result<()> {
        *self = Self::find(conn, self.id)?;
        Ok(())
    }
}

fn drive_file_ids(conn: &Connection, message_id: i64) -> Result<Vec<String>> {
    query_all(
        conn,
        r#"SELECT "drive_attachments"."file_id" FROM "drive_attachments" WHERE "drive_attachments"."message_id" = ? ORDER BY "drive_attachments"."id" ASC"#,
        [message_id],
        |r| r.get(0),
    )
}

/// `forward_note_mentionees`: the note's `@[Name]` names that identify exactly one active member
/// of the room, as those members.
pub fn forward_note_mentionees(conn: &Connection, room_id: i64, note: &str) -> Result<Vec<User>> {
    let mut unique = Vec::new();
    for name in mention_token_names(note) {
        if !unique.contains(&name) {
            unique.push(name);
        }
    }
    if unique.is_empty() {
        return Ok(Vec::new());
    }
    let sql = format!(
        r#"SELECT "users".* FROM "users" INNER JOIN "memberships" ON "users"."id" = "memberships"."user_id" WHERE "memberships"."room_id" = ? AND "users"."status" = 0 AND "users"."name" IN (SELECT "users"."name" FROM "users" INNER JOIN "memberships" ON "users"."id" = "memberships"."user_id" WHERE "memberships"."room_id" = ? AND "users"."status" = 0 AND "users"."name" IN ({}) GROUP BY "users"."name" HAVING (COUNT(*) = 1))"#,
        placeholders(unique.len())
    );
    let mut values: Vec<rusqlite::types::Value> = vec![room_id.into(), room_id.into()];
    values.extend(unique.into_iter().map(rusqlite::types::Value::from));
    query_all(
        conn,
        &sql,
        rusqlite::params_from_iter(values),
        User::from_row,
    )
}

/// `room.users.where(id: ids)`
pub fn mentionees_in_room(conn: &Connection, room_id: i64, user_ids: &[i64]) -> Result<Vec<User>> {
    if user_ids.is_empty() {
        return Ok(Vec::new());
    }
    let sql = format!(
        r#"SELECT "users".* FROM "users" INNER JOIN "memberships" ON "users"."id" = "memberships"."user_id" WHERE "memberships"."room_id" = ? AND "users"."id" IN ({})"#,
        placeholders(user_ids.len())
    );
    let values: Vec<i64> = std::iter::once(room_id)
        .chain(user_ids.iter().copied())
        .collect();
    query_all(
        conn,
        &sql,
        rusqlite::params_from_iter(values),
        User::from_row,
    )
}

/// `Google::DriveLink.valid_id?`: `/\A[A-Za-z0-9_-]{10,}\z/`
pub fn valid_drive_file_id(id: &str) -> bool {
    id.len() >= 10
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

/// `plain_text_body.match(/\A\/play (?<name>\w+)\z/)` then `Sound.find_by_name`.
pub fn sound_in(plain_text: &str) -> Option<&'static Sound> {
    let name = plain_text.strip_prefix("/play ")?;
    // `\z` allows no trailing newline; `\w` is ASCII word characters.
    if name.is_empty() || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
        return None;
    }
    Sound::find_by_name(name)
}

fn remove_from_index(tx: &Tx<'_>, id: i64) -> Result<()> {
    tx.conn()
        .execute_cached("delete from message_search_index where rowid = ?", [id])?;
    Ok(())
}

fn reversed<T>(mut rows: Vec<T>) -> Vec<T> {
    rows.reverse();
    rows
}
