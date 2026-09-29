//! `reference/app/models/message.rb` and `message/*.rb` (Attachment, Mentionee, Pagination,
//! Searchable; Broadcasts belong to the app).

use rusqlite::{Connection, Row, params};

use crate::database::Tx;
use crate::error::{Errors, OptionalExt, Result};
use crate::events::Event;
use crate::models::{Attachment, Blob, Boost, RichTextRecord, Room, RoomType, Sound, User};
use crate::rich_text::RichText;
use crate::sql::{self, CachedStatements, placeholders, query_all, query_one};
use crate::time::Timestamp;

/// `Message::Pagination::PAGE_SIZE`
pub const PAGE_SIZE: i64 = 40;

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
        query_all(conn, &format!(r#"SELECT "messages".* FROM "messages" {ORDERED}"#), [], Self::from_row)
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
        let sql = format!(r#"SELECT "messages".* FROM "messages" WHERE {condition} AND "messages"."id" = ? LIMIT 1"#);
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
        let rows = query_all(conn, &sql, rusqlite::params_from_iter(values), Self::from_row)?;
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
    pub fn page_before(conn: &Connection, timeline: Timeline, message: &Message) -> Result<Vec<Self>> {
        Self::page(conn, timeline, &format!(" AND ({BEFORE})"), Self::cursor(message), true)
    }

    /// `page_after(message)`: `after(message).first_page`
    pub fn page_after(conn: &Connection, timeline: Timeline, message: &Message) -> Result<Vec<Self>> {
        Self::page(conn, timeline, &format!(" AND ({AFTER})"), Self::cursor(message), false)
    }

    /// `page_around(message)`: up to 40 before, the message, up to 40 after.
    pub fn page_around(conn: &Connection, timeline: Timeline, message: &Message) -> Result<Vec<Self>> {
        let mut page = Self::page_before(conn, timeline, message)?;
        page.push(message.clone());
        page.extend(Self::page_after(conn, timeline, message)?);
        Ok(page)
    }

    /// `page_created_since(time)`: `where("created_at > ?", time).first_page`
    pub fn page_created_since(conn: &Connection, timeline: Timeline, time: Timestamp) -> Result<Vec<Self>> {
        Self::page(conn, timeline, " AND (created_at > ?)", vec![time.to_db().into()], false)
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
            filter = format!(r#" AND "messages"."id" NOT IN ({})"#, placeholders(excluding.len()));
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

    fn exists_in(conn: &Connection, timeline: Timeline, comparison: &str, message: &Message) -> Result<bool> {
        let (condition, value) = timeline.condition();
        let sql = format!(r#"SELECT 1 AS one FROM "messages" WHERE {condition} AND ({comparison}) LIMIT 1"#);
        sql::exists(conn, &sql, params![value, message.created_at, message.id])
    }

    /// `paged?`: more than a page (`count > PAGE_SIZE`). Asks whether a row exists past the
    /// first page rather than counting the whole timeline, which grows without bound.
    pub fn paged(conn: &Connection, timeline: Timeline) -> Result<bool> {
        let (condition, value) = timeline.condition();
        let sql = format!(r#"SELECT 1 FROM "messages" WHERE {condition} LIMIT 1 OFFSET {PAGE_SIZE}"#);
        sql::exists(conn, &sql, [value])
    }

    // Message::Searchable

    /// `room.messages.search(query)`: FTS5 `MATCH`, `ordered`. Upstream's search; ours goes through
    /// `SearchQuery` (a later workstream).
    pub fn search_in_room(conn: &Connection, room_id: i64, query: &str) -> Result<Vec<Self>> {
        let query = match_terms(query);
        if query.is_empty() {
            return Ok(Vec::new());
        }
        query_all(
            conn,
            r#"SELECT "messages".* FROM "messages" join message_search_index idx on messages.id = idx.rowid WHERE "messages"."room_id" = ? AND (idx.body match ?) ORDER BY "messages"."created_at" ASC, "messages"."id" ASC"#,
            params![room_id, query],
            Self::from_row,
        )
    }

    /// `Current.user.reachable_messages.search(query).last(100)`
    pub fn search_reachable(conn: &Connection, user_id: i64, query: &str) -> Result<Vec<Self>> {
        let query = match_terms(query);
        if query.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            r#"{SELECT_REACHABLE} join message_search_index idx on messages.id = idx.rowid WHERE "rooms"."deleted_at" IS NULL AND "memberships"."user_id" = ? AND (idx.body match ?) ORDER BY "messages"."created_at" DESC, "messages"."id" DESC LIMIT 100"#
        );
        Ok(reversed(query_all(
            conn,
            &sql,
            params![user_id, query],
            Self::from_row,
        )?))
    }

    // Creating, updating, destroying

    /// `room.messages.create!` (or `create_with_attachment!` given a blob). Inside the
    /// transaction: the message, its body (which touches the message), its attachment, and
    /// the room touch. After commit, unless the message is still streaming: the search index
    /// (never for a system note), then `receive_in_conversation`.
    ///
    /// Validated like `Message` (see [`Message::validate`]); a thread reply refreshes its
    /// thread's counter (`after_create :refresh_thread_messages_count`).
    ///
    /// Not yet ported (later workstreams): Markdown rendering, the reference syncs, activity
    /// items, agent deliveries, stale sibling threads, the thread indicator broadcast, and
    /// `ChannelThread#receive` for thread messages, which here mark nothing unread and push
    /// nothing.
    pub fn create(tx: &mut Tx<'_>, attributes: NewMessage) -> Result<Self> {
        Self::create_content(tx, attributes, None)
    }

    /// The Markdown renderer is supplied by the owning domain; source and rendered body commit
    /// together, before the normal Message callbacks (RoomMailbox uses this entry point).
    pub fn create_markdown(tx: &mut Tx<'_>, attributes: NewMessage, source: &str) -> Result<Self> {
        let mut errors = Errors::default();
        if source.chars().count() > 50_000 {
            errors.add("markdown_source", "is too long (maximum is 50000 characters)");
        }
        if !attributes.streaming && source.trim().is_empty() && attributes.attachment_blob_id.is_none() {
            errors.add("markdown_source", "can't be blank");
        }
        errors.into_result()?;
        Self::create_content(tx, attributes, Some(source))
    }

    fn create_content(tx: &mut Tx<'_>, attributes: NewMessage, source: Option<&str>) -> Result<Self> {
        Self::validate(tx.conn(), &attributes)?.into_result()?;
        let now = tx.now();
        let client_message_id = attributes.client_message_id.unwrap_or_else(sql::uuid);
        // `before_save :touch_streaming_activity, if: :streaming?`
        let streaming_updated_at = attributes.streaming.then_some(now);
        let id: i64 = tx.conn().query_row_cached(
            r#"INSERT INTO "messages" ("client_message_id", "created_at", "creator_id", "markdown_source", "room_id", "streaming", "streaming_updated_at", "system_note", "thread_id", "updated_at") VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING "id""#,
            params![
                client_message_id,
                now,
                attributes.creator_id,
                source,
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
        if message.thread_reply() {
            refresh_thread_messages_count(tx, message.thread_id)?;
        }

        let mut touched = false;
        if let Some(body) = &attributes.body {
            RichTextRecord::create(tx, RECORD_TYPE, id, "body", body)?;
            touched = true;
        }
        if let Some(blob_id) = attributes.attachment_blob_id {
            Attachment::create(tx, RECORD_TYPE, id, "attachment", blob_id)?;
            touched = true;
        }
        if touched {
            message.updated_at = Self::touch_row(tx, id)?;
        }
        Room::touch(tx, message.room_id)?;

        if !message.streaming {
            let committed = message.clone();
            tx.after_commit(move |tx| {
                committed.create_in_index(tx)?;
                committed.receive_in_conversation(tx)
            });
        }
        Ok(message)
    }

    /// The validations of `app/models/message.rb` that the attributes a [`NewMessage`] can
    /// carry are subject to. The others need attributes no Rust path writes yet (Markdown
    /// source, reply and forward links, Drive attachments).
    pub fn validate(conn: &Connection, attributes: &NewMessage) -> Result<Errors> {
        let mut errors = Errors::default();
        // `validate_conversation_links`
        if let Some(thread_id) = attributes.thread_id {
            let thread_room = query_one(
                conn,
                r#"SELECT "rooms"."id", "rooms"."type" FROM "channel_threads" INNER JOIN "rooms" ON "rooms"."id" = "channel_threads"."room_id" WHERE "channel_threads"."id" = ?"#,
                [thread_id],
                |r| Ok((r.get::<_, i64>(0)?, r.get::<_, RoomType>(1)?)),
            )?;
            if let Some((room_id, room_type)) = thread_room
                && (room_id != attributes.room_id || room_type == RoomType::Direct)
            {
                errors.add("thread", "must belong to the message room and cannot be a direct room thread");
            }
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
        Ok(errors)
    }

    /// `thread_reply?`: what `ChannelThread#messages_count` counts.
    pub fn thread_reply(&self) -> bool {
        self.thread_id.is_some() && !self.system_note && !self.streaming
    }

    /// `before_save :touch_streaming_activity, if: :streaming?`: every save while streaming
    /// restarts the finalize sweep's inactivity clock. That's a saved change, so it touches the
    /// room too (`belongs_to :room, touch: true`), even when nothing else changed. (`touch` isn't
    /// a save, so boosts don't restart the clock.) `save_touches_test` holds each save path to
    /// Rails' answer.
    fn touch_streaming_activity(&mut self, tx: &mut Tx<'_>) -> Result<()> {
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
        self.touch_streaming_activity(tx)?;
        match RichTextRecord::find_for(tx.conn(), RECORD_TYPE, self.id, "body")? {
            Some(record) if record.body.as_deref() == Some(body) => return Ok(()),
            Some(mut record) => record.update_body(tx, body)?,
            None => {
                RichTextRecord::create(tx, RECORD_TYPE, self.id, "body", body)?;
            }
        }
        self.touch(tx)
    }

    /// `touch` (from a boost, or the body): the message, then its room, then a reindex
    /// after commit.
    pub fn touch(&mut self, tx: &mut Tx<'_>) -> Result<()> {
        self.updated_at = Self::touch_row(tx, self.id)?;
        Room::touch(tx, self.room_id)?;
        let id = self.id;
        tx.after_commit(move |tx| {
            let message = Message::find(tx.conn(), id)?;
            if message.streaming {
                return Ok(());
            }
            message.update_in_index(tx)
        });
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

    /// `destroy`: `preserve_reply_tombstones`, then the dependents in declaration order (boosts;
    /// board stale digests nullified; activity items and agent steps, which have no destroy
    /// callbacks), its attachment (blob purged later) and body, then the message, and the room
    /// is touched. Its thread's parent link and its forwards' source link are nulled by their
    /// foreign keys (`ON DELETE SET NULL`), as `dependent: :nullify` would. A thread reply
    /// refreshes its thread's counter unless the room is being torn down
    /// (`destroyed_with_conversation?`). The search index entry goes after commit.
    ///
    /// Not ported (WS8): the dependents with destroy callbacks of their own (poll, pins, saved
    /// items, the reference rows, Drive attachments). Their foreign keys have no `ON DELETE`, so
    /// destroying a message that has any fails with a constraint error rather than orphaning
    /// them. The quote card broadcasts aren't ported either.
    pub fn destroy(&self, tx: &mut Tx<'_>) -> Result<()> {
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
        tx.conn().execute_cached(
            r#"UPDATE "board_stale_digests" SET "message_id" = NULL WHERE "board_stale_digests"."message_id" = ?"#,
            [self.id],
        )?;
        tx.conn().execute_cached(
            r#"DELETE FROM "activity_items" WHERE "activity_items"."source_type" = 'Message' AND "activity_items"."source_id" = ?"#,
            [self.id],
        )?;
        tx.conn().execute_cached(
            r#"DELETE FROM "agent_steps" WHERE "agent_steps"."message_id" = ?"#,
            [self.id],
        )?;
        if let Some(body) = RichTextRecord::find_for(tx.conn(), RECORD_TYPE, self.id, "body")? {
            body.delete(tx)?;
        }
        tx.conn().execute_cached(
            r#"DELETE FROM "messages" WHERE "messages"."id" = ?"#,
            [self.id],
        )?;
        if self.thread_reply() && !Room::find(tx.conn(), self.room_id)?.deleted() {
            refresh_thread_messages_count(tx, self.thread_id)?;
        }
        Room::touch(tx, self.room_id)?;
        let id = self.id;
        tx.after_commit(move |tx| remove_from_index(tx, id));
        Ok(())
    }

    /// `receive_in_conversation`: the thread's, or the room's.
    fn receive_in_conversation(&self, tx: &mut Tx<'_>) -> Result<()> {
        if self.thread_id.is_some() {
            // `thread.receive(self)`: `ChannelThread` isn't ported yet.
            return Ok(());
        }
        Room::receive(tx, self.room_id, self)
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

    /// `plain_text_body`: `body.to_plain_text.presence || attachment&.filename&.to_s || ""`
    pub fn plain_text_body(&self, conn: &Connection, rich_text: &dyn RichText) -> Result<String> {
        if let Some(html) = self.body_html(conn)? {
            let text = rich_text.to_plain_text(conn, &html, &|id| {
                User::find_by_id(conn, id).ok().flatten().map(|u| u.name)
            });
            if !text.trim().is_empty() {
                return Ok(text);
            }
        }
        Ok(self
            .attachment(conn)?
            .map(|(_, blob)| blob.filename)
            .unwrap_or_default())
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

    /// `mentionees`: mentioned users who are members of the room.
    pub fn mentionees(&self, conn: &Connection, rich_text: &dyn RichText) -> Result<Vec<User>> {
        let ids = match self.body_html(conn)? {
            Some(html) => rich_text.mentioned_user_ids(conn, &html),
            None => Vec::new(),
        };
        mentionees_in_room(conn, self.room_id, &ids)
    }

    pub fn reload(&mut self, conn: &Connection) -> Result<()> {
        *self = Self::find(conn, self.id)?;
        Ok(())
    }
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

/// `plain_text_body.match(/\A\/play (?<name>\w+)\z/)` then `Sound.find_by_name`.
pub fn sound_in(plain_text: &str) -> Option<&'static Sound> {
    let name = plain_text.strip_prefix("/play ")?;
    // `\z` allows no trailing newline; `\w` is ASCII word characters.
    if name.is_empty() || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
        return None;
    }
    Sound::find_by_name(name)
}

/// `ChannelThread.refresh_messages_count` (`app/models/channel_thread.rb`): recount with
/// `REPLY_COUNT_SQL`, then bump the parent message so its reply indicator re-renders.
fn refresh_thread_messages_count(tx: &Tx<'_>, thread_id: Option<i64>) -> Result<()> {
    let Some(thread_id) = thread_id else { return Ok(()) };
    tx.conn().execute_cached(
        r#"UPDATE "channel_threads" SET messages_count = ( SELECT COUNT(*) FROM messages WHERE messages.thread_id = channel_threads.id AND messages.system_note = 0 AND messages.streaming = 0 ) WHERE "channel_threads"."id" = ?"#,
        [thread_id],
    )?;
    tx.conn().execute_cached(
        r#"UPDATE "messages" SET "updated_at" = ? WHERE "messages"."id" IN (SELECT "channel_threads"."parent_message_id" FROM "channel_threads" WHERE "channel_threads"."id" = ? AND "channel_threads"."parent_message_id" IS NOT NULL)"#,
        params![tx.now(), thread_id],
    )?;
    Ok(())
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

/// Each word of a search as an FTS5 string, so every word must appear and none is read as query
/// syntax. Rails passes the words straight to `MATCH`, where `NOT`, `AND`, `OR` or `NEAR` in the
/// wrong place is a syntax error (a 500).
/// NULs separate words too: SQLite would end the query string at one.
fn match_terms(query: &str) -> String {
    query
        .split(|c: char| c.is_whitespace() || c == '\0')
        .filter(|word| !word.is_empty())
        .map(|word| format!("\"{}\"", word.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(" ")
}
