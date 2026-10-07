//! `reference/app/models/channel_thread.rb`, the core thread behaviour: creation and its
//! validations, the lifecycle (stale, closed, locked), posting, `receive`, the reply counter and
//! the parent's thread indicator, and destroy. `channel_thread/message_pusher.rb` is
//! [`ChannelThread::push_recipients`].
//!
//! Boards and work tracking keep their state in this model. Human work/result writes,
//! fresh-owner policy, board creation and agent-assignment ledger integration live in
//! `channel_thread/work`. Agent writes and committed tag assignment live in the
//! adjacent modules; their services reuse the real agent ledger and delivery APIs.
//! Board listings, post/row broadcasts and their commit callbacks live in `channel_thread/board`.

use std::collections::{HashMap, HashSet};

use jiff::SignedDuration;
use rusqlite::{Connection, Row, params};
use serde::{Deserialize, Serialize};

use crate::broadcasts::{Broadcast, Partial, dom_id, room_messages};
use crate::database::Tx;
use crate::error::{Error, Errors, OptionalExt, Result};
use crate::events::{Event, Job};
use crate::models::{
    Membership, Message, NewMessage, PushPayload, PushSubscription, Room, ThreadInvolvement,
    ThreadMembership, ThreadTag, User, thread_tag,
};
use crate::rich_text::RichText;
use crate::sql::{self, CachedStatements, placeholders, query_all, query_one};
use crate::time::Timestamp;

mod agent_work;
mod board;
mod tag_assignment;
mod work;
mod work_listing;
pub use agent_work::{AgentWorkChanges, tag_names_from_value};
pub use board::{BOARD_POSTS_MAX_PAGE, BOARD_POSTS_PER_PAGE, WorkOwners, board_page_number};
pub use work::{WORK_UPDATE_FORBIDDEN, WorkChanges, normalize_owner_id};
pub use work_listing::{WorkReadFacts, WorkReadPermissions};

/// A tracked thread's work facts changed: its status, owner, result or run URL, or a link was
/// added or removed. The classic app has no broadcast for it (only the board rows, which
/// `register_board_update` emits as before); the cable sink publishes the single-page app's
/// `thread.updated` with the new facts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThreadWorkChange {
    pub thread_id: i64,
}
impl crate::events::Broadcast for ThreadWorkChange {
    const KIND: &'static str = "ChannelThread#sync_work";
}
impl ThreadWorkChange {
    pub fn emit(tx: &mut Tx<'_>, thread_id: i64) {
        // Include tag auto-assignment's after-commit write before publishing the final facts.
        tx.broadcast_after_commit_settled_once(&ThreadWorkChange { thread_id });
    }
}

/// `ChannelThread::AUTO_ARCHIVE_OPTIONS`, in minutes.
pub const AUTO_ARCHIVE_OPTIONS: [i64; 4] = [60, 1_440, 4_320, 10_080];
pub const DEFAULT_AUTO_ARCHIVE_AFTER_MINUTES: i64 = 4_320;
pub const NAME_LIMIT: usize = 100;
/// Tags per board post.
pub const TAG_LIMIT: usize = 5;
pub const RESULT_LIMIT: usize = 20_000;
pub const RUN_URL_LIMIT: usize = 500;
pub const WORK_STATUSES: [&str; 4] = ["planned", "in_progress", "blocked", "done"];

/// `REPLY_COUNT_SQL`: what `messages_count` counts, recomputed from the rows.
const REPLY_COUNT_SQL: &str = "messages_count = ( SELECT COUNT(*) FROM messages WHERE messages.thread_id = channel_threads.id AND messages.system_note = 0 AND messages.streaming = 0 )";

/// The staleness test `stale?` makes, in SQL (`close_stale_in`, `effectively_closed`).
const STALE_SQL: &str =
    "datetime(last_activity_at, '+' || auto_archive_after_minutes || ' minutes') <= datetime(?)";

#[derive(Debug, Clone, PartialEq)]
pub struct ChannelThread {
    pub id: i64,
    pub room_id: i64,
    pub creator_id: i64,
    /// The root message the thread was started from; nil for a board post or once it's deleted.
    pub parent_message_id: Option<i64>,
    pub name: String,
    pub auto_archive_after_minutes: i64,
    pub closed_at: Option<Timestamp>,
    pub locked_at: Option<Timestamp>,
    pub last_activity_at: Timestamp,
    /// The thread's finished, non-system messages (`REPLY_COUNT_SQL`).
    pub messages_count: i64,
    pub result_markdown: Option<String>,
    pub result_updated_at: Option<Timestamp>,
    pub result_updated_by_id: Option<i64>,
    pub run_url: Option<String>,
    pub work_owner_id: Option<i64>,
    pub work_status: Option<String>,
    pub work_status_changed_at: Option<Timestamp>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// Attributes for `ChannelThread.create!`.
#[derive(Debug, Clone, Default)]
pub struct NewChannelThread {
    pub room_id: i64,
    pub creator_id: i64,
    pub parent_message_id: Option<i64>,
    /// Defaults from the parent message's first line (`set_default_name`), except in boards.
    pub name: Option<String>,
    /// Defaults to [`DEFAULT_AUTO_ARCHIVE_AFTER_MINUTES`].
    pub auto_archive_after_minutes: Option<i64>,
    /// Defaults to now (`set_default_last_activity_at`).
    pub last_activity_at: Option<Timestamp>,
    /// Required in boards.
    pub work_status: Option<String>,
    /// `tag_names=`: normalised (stripped, downcased, deduplicated, blanks dropped).
    pub tag_names: Option<Vec<String>>,
    pub work_owner_id: Option<i64>,
    pub run_url: Option<String>,
}

/// `ChannelThread::LockedError`, raised by `post_message!` into a locked thread.
pub const LOCKED_MESSAGE: &str = "This thread is locked";

/// `status`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadStatus {
    Active,
    Closed,
    Locked,
}

impl ThreadStatus {
    pub fn name(self) -> &'static str {
        match self {
            ThreadStatus::Active => "active",
            ThreadStatus::Closed => "closed",
            ThreadStatus::Locked => "locked",
        }
    }
}

/// `ChannelThread::PushMessageJob.perform_later(thread, message)`, from `receive`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PushMessageJob {
    pub thread_id: i64,
    pub message_id: i64,
}

impl Job for PushMessageJob {
    const CLASS: &'static str = "ChannelThread::PushMessageJob";
}

/// One thread member the pusher considers, with everything `Notifications::Policy.new(kind:
/// :thread_message, ...)` is given.
#[derive(Debug, Clone)]
pub struct ThreadPushCandidate {
    pub recipient: User,
    pub room_membership: Option<Membership>,
    pub thread_membership: ThreadMembership,
    pub mentioned: bool,
    /// The recipient wrote the message this one replies to, and the reply notifies them.
    pub reply_to_recipient: bool,
}

/// One recipient's push: the reply payload for the replied-to author, the thread payload for
/// anyone else, sent to each of the recipient's subscriptions.
#[derive(Debug, Clone)]
pub struct ThreadPush {
    pub user_id: i64,
    pub payload: PushPayload,
    /// `tag: "room-#{thread.room_id}"`
    pub tag: String,
    pub subscriptions: Vec<PushSubscription>,
}

impl ChannelThread {
    pub(crate) fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            room_id: row.get("room_id")?,
            creator_id: row.get("creator_id")?,
            parent_message_id: row.get("parent_message_id")?,
            name: row.get("name")?,
            auto_archive_after_minutes: row.get("auto_archive_after_minutes")?,
            closed_at: row.get("closed_at")?,
            locked_at: row.get("locked_at")?,
            last_activity_at: row.get("last_activity_at")?,
            messages_count: row.get("messages_count")?,
            result_markdown: row.get("result_markdown")?,
            result_updated_at: row.get("result_updated_at")?,
            result_updated_by_id: row.get("result_updated_by_id")?,
            run_url: row.get("run_url")?,
            work_owner_id: row.get("work_owner_id")?,
            work_status: row.get("work_status")?,
            work_status_changed_at: row.get("work_status_changed_at")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    pub fn find(conn: &Connection, id: i64) -> Result<Self> {
        Self::find_by_id(conn, id)?.or_not_found("ChannelThread")
    }

    pub fn find_by_id(conn: &Connection, id: i64) -> Result<Option<Self>> {
        query_one(
            conn,
            r#"SELECT * FROM "channel_threads" WHERE "channel_threads"."id" = ? LIMIT 1"#,
            [id],
            Self::from_row,
        )
    }

    /// Preload the conversations of a bounded, authorized inbox page.
    pub fn for_ids(conn: &Connection, ids: &[i64]) -> Result<Vec<Self>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        query_all(
            conn,
            "SELECT * FROM channel_threads WHERE id IN (SELECT value FROM json_each(?))",
            [serde_json::json!(ids).to_string()],
            Self::from_row,
        )
    }
    /// `message.channel_thread`: the thread started from a message.
    pub fn find_by_parent_message(conn: &Connection, message_id: i64) -> Result<Option<Self>> {
        query_one(
            conn,
            r#"SELECT * FROM "channel_threads" WHERE "channel_threads"."parent_message_id" = ? LIMIT 1"#,
            [message_id],
            Self::from_row,
        )
    }

    /// `room.channel_threads.ordered`: most recently active first.
    pub fn for_room(conn: &Connection, room_id: i64) -> Result<Vec<Self>> {
        query_all(
            conn,
            r#"SELECT "channel_threads".* FROM "channel_threads" WHERE "channel_threads"."room_id" = ? ORDER BY "channel_threads"."last_activity_at" DESC, "channel_threads"."id" DESC"#,
            [room_id],
            Self::from_row,
        )
    }

    /// Preload the forward picker's threads in one query, retaining each room's ordering.
    pub fn for_rooms(conn: &Connection, room_ids: &[i64]) -> Result<Vec<Self>> {
        if room_ids.is_empty() {
            return Ok(Vec::new());
        }
        query_all(
            conn,
            "SELECT * FROM channel_threads WHERE room_id IN (SELECT value FROM json_each(?)) ORDER BY last_activity_at DESC, id DESC",
            [serde_json::json!(room_ids).to_string()],
            Self::from_row,
        )
    }

    /// `room.channel_threads.active.ordered`: neither closed nor locked (stale ones included:
    /// `status` reads them as closed).
    pub fn active_for_room(conn: &Connection, room_id: i64) -> Result<Vec<Self>> {
        query_all(
            conn,
            r#"SELECT "channel_threads".* FROM "channel_threads" WHERE "channel_threads"."room_id" = ? AND "channel_threads"."closed_at" IS NULL AND "channel_threads"."locked_at" IS NULL ORDER BY "channel_threads"."last_activity_at" DESC, "channel_threads"."id" DESC"#,
            [room_id],
            Self::from_row,
        )
    }

    /// `room.channel_threads.effectively_closed.ordered`: closed (locked ones too), or stale
    /// outside boards.
    pub fn effectively_closed_for_room(
        conn: &Connection,
        room_id: i64,
        now: Timestamp,
    ) -> Result<Vec<Self>> {
        let sql = format!(
            r#"SELECT "channel_threads".* FROM "channel_threads" WHERE "channel_threads"."room_id" = ? AND ("channel_threads"."closed_at" IS NOT NULL OR ("channel_threads"."closed_at" IS NULL AND "channel_threads"."locked_at" IS NULL AND "channel_threads"."room_id" NOT IN (SELECT "rooms"."id" FROM "rooms" WHERE "rooms"."type" = 'Rooms::Board') AND ({STALE_SQL}))) ORDER BY "channel_threads"."last_activity_at" DESC, "channel_threads"."id" DESC"#
        );
        query_all(conn, &sql, params![room_id, now.to_db()], Self::from_row)
    }

    /// `user.followed_threads`: the threads a user has a thread membership in, in live rooms.
    pub fn followed_by(conn: &Connection, user_id: i64) -> Result<Vec<Self>> {
        query_all(
            conn,
            r#"SELECT "channel_threads".* FROM "channel_threads" INNER JOIN "thread_memberships" ON "channel_threads"."id" = "thread_memberships"."thread_id" INNER JOIN "rooms" ON "rooms"."id" = "channel_threads"."room_id" WHERE "thread_memberships"."user_id" = ? AND "rooms"."deleted_at" IS NULL ORDER BY "channel_threads"."last_activity_at" DESC, "channel_threads"."id" DESC"#,
            [user_id],
            Self::from_row,
        )
    }

    pub fn count(conn: &Connection) -> Result<i64> {
        sql::count(conn, r#"SELECT COUNT(*) FROM "channel_threads""#, [])
    }

    // Creating

    /// `ChannelThread.create!(attributes)`: the default name and activity stamp
    /// (`before_validation ... on: :create`), the validations, then the row and its tags
    /// (`apply_pending_tag_names`), with `work_status_changed_at` stamped when a work status is
    /// set (`stamp_work_status_changed_at`).
    ///
    /// `announce_board_post` broadcasts the rows and marks the board unread after commit.
    /// Added tags register the Rails after-commit auto-assignment callback.
    pub fn create(tx: &mut Tx<'_>, attributes: NewChannelThread) -> Result<Self> {
        let now = tx.now();
        let room = Room::find(tx.conn(), attributes.room_id)?;
        let name = match attributes.name.filter(|name| !name.trim().is_empty()) {
            Some(name) => Some(name),
            None if room.board() => None,
            None => Some(default_name(
                tx.conn(),
                tx.rich_text(),
                attributes.parent_message_id,
            )?),
        };
        let tag_names = attributes
            .tag_names
            .map(|names| normalize_tag_names(&names));
        let work_status = attributes.work_status.filter(|status| !status.is_empty());
        let mut thread = ChannelThread {
            id: 0,
            room_id: attributes.room_id,
            creator_id: attributes.creator_id,
            parent_message_id: attributes.parent_message_id,
            name: name.unwrap_or_default(),
            auto_archive_after_minutes: attributes
                .auto_archive_after_minutes
                .unwrap_or(DEFAULT_AUTO_ARCHIVE_AFTER_MINUTES),
            closed_at: None,
            locked_at: None,
            last_activity_at: attributes.last_activity_at.unwrap_or(now),
            messages_count: 0,
            result_markdown: None,
            result_updated_at: None,
            result_updated_by_id: None,
            run_url: attributes.run_url,
            work_owner_id: attributes.work_owner_id,
            work_status_changed_at: work_status.is_some().then_some(now),
            work_status,
            created_at: now,
            updated_at: now,
        };
        thread
            .validate_for_save(tx.conn(), &room, tag_names.as_deref(), true)?
            .into_result()?;

        let id: i64 = tx.conn().query_row_cached(
            r#"INSERT INTO "channel_threads" ("auto_archive_after_minutes", "created_at", "creator_id", "last_activity_at", "name", "parent_message_id", "room_id", "updated_at", "work_status", "work_status_changed_at", "work_owner_id", "run_url") VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING "id""#,
            params![
                thread.auto_archive_after_minutes,
                now,
                thread.creator_id,
                thread.last_activity_at,
                thread.name,
                thread.parent_message_id,
                thread.room_id,
                now,
                thread.work_status,
                thread.work_status_changed_at,
                thread.work_owner_id,
                thread.run_url
            ],
            |r| r.get(0),
        )?;
        tx.register_record("channel_threads", id);
        Self::register_board_creation(tx, id, &room);
        let tag_names = tag_names.unwrap_or_default();
        Self::register_tag_assignment(tx, id, tag_names.clone(), thread.work_owner_id);
        for name in tag_names {
            thread.id = id;
            ThreadTag::create_for_thread(tx, &thread, &name)?;
        }
        thread.id = id;
        Ok(thread)
    }

    /// The validations of `app/models/channel_thread.rb`, including new owner eligibility.
    pub fn validate(
        &self,
        conn: &Connection,
        room: &Room,
        tag_names: Option<&[String]>,
    ) -> Result<Errors> {
        self.validate_for_save(conn, room, tag_names, self.id == 0)
    }

    fn validate_for_save(
        &self,
        conn: &Connection,
        room: &Room,
        tag_names: Option<&[String]>,
        owner_changed: bool,
    ) -> Result<Errors> {
        let mut errors = Errors::default();
        if self.name.trim().is_empty() {
            errors.add("name", "can't be blank");
        }
        if self.name.chars().count() > NAME_LIMIT {
            errors.add(
                "name",
                format!("is too long (maximum is {NAME_LIMIT} characters)"),
            );
        }
        if !AUTO_ARCHIVE_OPTIONS.contains(&self.auto_archive_after_minutes) {
            errors.add("auto_archive_after_minutes", "is not included in the list");
        }
        if let Some(status) = &self.work_status
            && !WORK_STATUSES.contains(&status.as_str())
        {
            errors.add("work_status", "is not included in the list");
        }
        if let Some(result) = &self.result_markdown
            && result.chars().count() > RESULT_LIMIT
        {
            errors.add(
                "result_markdown",
                format!("is too long (maximum is {RESULT_LIMIT} characters)"),
            );
        }
        if let Some(run_url) = &self.run_url
            && run_url.chars().count() > RUN_URL_LIMIT
        {
            errors.add(
                "run_url",
                format!("is too long (maximum is {RUN_URL_LIMIT} characters)"),
            );
        }
        // work_owner_requires_work
        if self.work_owner_id.is_some() && self.work_status.as_deref().is_none_or(str::is_empty) {
            errors.add("work_owner", "requires work tracking");
        }
        if owner_changed {
            errors.0.extend(self.validate_work_owner(conn)?.0);
        }
        // room_cannot_be_direct
        if room.direct() {
            errors.add("room", "can't be a direct room");
        }
        // parent_message_belongs_to_room
        if let Some(parent_id) = self.parent_message_id
            && let Some(parent) = Message::find_by_id(conn, parent_id)?
            && (parent.room_id != self.room_id || parent.thread_id.is_some())
        {
            errors.add(
                "parent_message",
                "must be a root message in the parent room",
            );
        }
        // work_status_required_in_boards
        if room.board() && self.work_status.as_deref().is_none_or(str::is_empty) {
            errors.add("work_status", "must be tracked in a board");
        }
        // run_url_must_be_https
        if let Some(run_url) = self.run_url.as_deref().filter(|url| !url.trim().is_empty())
            && !run_url.starts_with("https://")
        {
            errors.add("run_url", "must be an https URL");
        }
        // tag_names_within_limits: the staged names, or the stored ones.
        let stored;
        let names = match tag_names {
            Some(names) => names,
            None => {
                stored = if self.id == 0 {
                    Vec::new()
                } else {
                    ThreadTag::for_thread(conn, self.id)?
                        .into_iter()
                        .map(|tag| tag.name)
                        .collect()
                };
                &stored
            }
        };
        if names.len() > TAG_LIMIT {
            errors.add("tags", format!("are limited to {TAG_LIMIT} per post"));
        }
        for name in names {
            if name.chars().count() > thread_tag::TAG_NAME_LIMIT {
                errors.add(
                    "tags",
                    format!("must be at most {} characters", thread_tag::TAG_NAME_LIMIT),
                );
            } else if !thread_tag::valid_tag_name(name) {
                errors.add("tags", "use lowercase letters, digits, and hyphens");
            }
        }
        Ok(errors)
    }

    /// `update!` of the given state: validated, then written with a fresh `updated_at` if
    /// anything changed (a save with no changes writes nothing).
    fn save(&mut self, tx: &mut Tx<'_>, changed: ChannelThread) -> Result<()> {
        tx.register_record("channel_threads", self.id);
        if changed == *self {
            return Ok(());
        }
        let room = Room::find(tx.conn(), changed.room_id)?;
        if let Err(error) = changed
            .validate_for_save(
                tx.conn(),
                &room,
                None,
                changed.work_owner_id != self.work_owner_id,
            )?
            .into_result()
        {
            // Like Active Record, the operation instance retains assigned values
            // after a validation failure while its transaction rolls back the rows.
            *self = changed;
            return Err(error);
        }
        self.save_validated(tx, changed, &room)
    }

    // Called only after validation under this same writer lock. Tag writes between
    // validation and this save cannot change room, agent or membership authority.
    fn save_validated(
        &mut self,
        tx: &mut Tx<'_>,
        changed: ChannelThread,
        room: &Room,
    ) -> Result<()> {
        if changed == *self {
            return Ok(());
        }
        let now = tx.now();
        let stored = Self::find(tx.conn(), self.id)?;
        let revision = tx.revision_after(stored.updated_at);
        let mut changed = changed;
        // WS12: `stamp_work_status_changed_at` on an update that changes the work status.
        if changed.work_status != self.work_status {
            changed.work_status_changed_at = Some(now);
        }
        changed.updated_at = revision;
        // Active Record writes dirty columns only. In particular, a stale settings
        // instance must not overwrite an owner, result or status saved by another caller.
        let mut fields: Vec<(&str, &dyn rusqlite::ToSql)> = Vec::new();
        macro_rules! dirty { ($($field:ident),+ $(,)?) => { $(
            if changed.$field != self.$field { fields.push((stringify!($field), &changed.$field)); }
        )+ }; }
        dirty!(
            auto_archive_after_minutes,
            closed_at,
            last_activity_at,
            locked_at,
            name,
            work_status,
            work_status_changed_at,
            work_owner_id,
            result_markdown,
            result_updated_at,
            result_updated_by_id,
            run_url
        );
        fields.push(("updated_at", &revision));
        let assignments = fields
            .iter()
            .map(|(column, _)| format!("\"{column}\"=?"))
            .collect::<Vec<_>>()
            .join(",");
        let mut values = fields.iter().map(|(_, value)| *value).collect::<Vec<_>>();
        values.push(&self.id);
        tx.conn().execute(
            &format!("UPDATE channel_threads SET {assignments} WHERE id=?"),
            values.as_slice(),
        )?;
        let status_changed = changed.work_status != self.work_status;
        let row_changed = status_changed
            || changed.name != self.name
            || changed.work_owner_id != self.work_owner_id
            || changed.last_activity_at != self.last_activity_at;
        if changed.work_changed_from(self) {
            ThreadWorkChange::emit(tx, self.id);
        }
        *self = Self::find(tx.conn(), self.id)?;
        self.register_board_update(tx, room, row_changed, status_changed)?;
        Ok(())
    }

    /// Whether a work column differs from `before`'s: the change [`ThreadWorkChange`] publishes
    /// as `thread.updated`.
    pub fn work_changed_from(&self, before: &ChannelThread) -> bool {
        self.work_status != before.work_status
            || self.work_owner_id != before.work_owner_id
            || self.run_url != before.run_url
            || self.result_markdown != before.result_markdown
            || self.result_updated_at != before.result_updated_at
    }

    /// `update!(name:, auto_archive_after_minutes:)`: the thread settings form.
    pub fn update_settings(
        &mut self,
        tx: &mut Tx<'_>,
        name: Option<&str>,
        auto_archive_after_minutes: Option<i64>,
    ) -> Result<()> {
        let mut changed = self.clone();
        if let Some(name) = name {
            changed.name = name.to_string();
        }
        if let Some(minutes) = auto_archive_after_minutes {
            changed.auto_archive_after_minutes = minutes;
        }
        self.save(tx, changed)
    }

    /// The ordinary thread metadata update with Rails' pending tag set and callbacks.
    pub fn update_metadata(
        &mut self,
        tx: &mut Tx<'_>,
        name: Option<&str>,
        minutes: Option<i64>,
        tags: Option<&[String]>,
    ) -> Result<()> {
        let mut changed = self.clone();
        if let Some(name) = name {
            changed.name = name.into();
        }
        if let Some(minutes) = minutes {
            changed.auto_archive_after_minutes = minutes;
        }
        self.save_with_tags(tx, changed, tags.map(normalize_tag_names))
    }

    // Lifecycle

    /// `status`: locked, else closed (explicitly, or stale), else active. Reads report a stale
    /// thread closed before any write persists it.
    pub fn status(&self, conn: &Connection, now: Timestamp) -> Result<ThreadStatus> {
        Ok(if self.locked_at.is_some() {
            ThreadStatus::Locked
        } else if self.closed_at.is_some() || self.stale(conn, now)? {
            ThreadStatus::Closed
        } else {
            ThreadStatus::Active
        })
    }

    /// Same lifecycle read with the already-preloaded parent room (destination pickers).
    pub fn status_in_room(&self, room: &Room, now: Timestamp) -> ThreadStatus {
        if self.locked_at.is_some() {
            ThreadStatus::Locked
        } else if self.closed_at.is_some() || (!room.board() && self.auto_archive_at() <= now) {
            ThreadStatus::Closed
        } else {
            ThreadStatus::Active
        }
    }

    /// `auto_archive_at`
    pub fn auto_archive_at(&self) -> Timestamp {
        self.last_activity_at
            .since(SignedDuration::from_mins(self.auto_archive_after_minutes))
    }

    /// `stale?`: open, unlocked, outside a board, and quiet past its auto-archive window.
    pub fn stale(&self, conn: &Connection, now: Timestamp) -> Result<bool> {
        if self.closed_at.is_some() || self.locked_at.is_some() || self.auto_archive_at() > now {
            return Ok(false);
        }
        Ok(!Room::find(conn, self.room_id)?.board())
    }

    /// `board_post?`
    pub fn board_post(&self, conn: &Connection) -> Result<bool> {
        Ok(Room::find(conn, self.room_id)?.board())
    }

    pub fn work(&self) -> bool {
        self.work_status
            .as_deref()
            .is_some_and(|status| !status.is_empty())
    }

    /// `ChannelThread.close_stale_in(room:)`: closes every stale thread outside boards
    /// (in the room, or everywhere), stamping `closed_at` and advancing each row's version.
    pub fn close_stale_in(tx: &Tx<'_>, room_id: Option<i64>) -> Result<usize> {
        let now = tx.now();
        let room_filter = if room_id.is_some() {
            r#""channel_threads"."room_id" = ? AND "#
        } else {
            ""
        };
        let sql = format!(
            r#"SELECT "channel_threads"."id", "channel_threads"."updated_at" FROM "channel_threads" WHERE {room_filter}"channel_threads"."room_id" NOT IN (SELECT "rooms"."id" FROM "rooms" WHERE "rooms"."type" = 'Rooms::Board') AND "channel_threads"."closed_at" IS NULL AND "channel_threads"."locked_at" IS NULL AND ({STALE_SQL})"#
        );
        let mut values: Vec<rusqlite::types::Value> = vec![];
        if let Some(room_id) = room_id {
            values.push(room_id.into());
        }
        values.push(now.to_db().into());
        let rows: Vec<(i64, Timestamp)> =
            query_all(tx.conn(), &sql, rusqlite::params_from_iter(values), |row| {
                Ok((row.get(0)?, row.get(1)?))
            })?;
        for &(id, previous) in &rows {
            tx.conn().execute_cached(
                "UPDATE channel_threads SET closed_at=?,updated_at=? WHERE id=?",
                params![now, tx.revision_after(previous), id],
            )?;
        }
        Ok(rows.len())
    }

    /// `close_if_stale!(expected_last_activity_at:)`
    pub fn close_if_stale(
        &mut self,
        tx: &mut Tx<'_>,
        expected_last_activity_at: Option<Timestamp>,
    ) -> Result<()> {
        self.reload(tx.conn())?;
        if expected_last_activity_at.is_some_and(|expected| expected != self.last_activity_at) {
            return Ok(());
        }
        if self.stale(tx.conn(), tx.now())? {
            let mut changed = self.clone();
            changed.closed_at = Some(tx.now());
            self.save(tx, changed)?;
        }
        Ok(())
    }

    /// `reopen!`: an explicitly closed (not locked) thread reopens; a stale one also gets fresh
    /// activity, so it doesn't read as closed again straight away.
    pub fn reopen(&mut self, tx: &mut Tx<'_>) -> Result<()> {
        self.reload(tx.conn())?;
        let now = tx.now();
        if self.status(tx.conn(), now)? == ThreadStatus::Closed {
            let mut changed = self.clone();
            changed.closed_at = None;
            self.save(tx, changed)?;
        }
        if self.stale(tx.conn(), now)? {
            let mut changed = self.clone();
            changed.last_activity_at = now;
            self.save(tx, changed)?;
        }
        Ok(())
    }

    /// `close!`: stamps `closed_at` unless locked or already stamped (even when it already reads
    /// closed from staleness).
    pub fn close(&mut self, tx: &mut Tx<'_>) -> Result<()> {
        self.reload(tx.conn())?;
        if self.locked_at.is_none() && self.closed_at.is_none() {
            let mut changed = self.clone();
            changed.closed_at = Some(tx.now());
            self.save(tx, changed)?;
        }
        Ok(())
    }

    /// `lock_conversation!`: closed and locked, keeping earlier stamps.
    pub fn lock_conversation(&mut self, tx: &mut Tx<'_>) -> Result<()> {
        self.reload(tx.conn())?;
        let now = tx.now();
        let mut changed = self.clone();
        changed.closed_at = changed.closed_at.or(Some(now));
        changed.locked_at = changed.locked_at.or(Some(now));
        self.save(tx, changed)
    }

    /// `unlock_conversation!`: unlocked and reopened, with fresh activity if it went stale.
    pub fn unlock_conversation(&mut self, tx: &mut Tx<'_>) -> Result<()> {
        self.reload(tx.conn())?;
        let mut changed = self.clone();
        changed.locked_at = None;
        changed.closed_at = None;
        self.save(tx, changed)?;
        let now = tx.now();
        if self.stale(tx.conn(), now)? {
            let mut changed = self.clone();
            changed.last_activity_at = now;
            self.save(tx, changed)?;
        }
        Ok(())
    }

    // Posting and receiving

    /// `post_message!(creator:, attributes:)`: in one critical section, refuse a locked thread,
    /// require the creator's room membership, join the thread, reopen it with fresh activity, and
    /// create the message in it.
    pub fn post_message(
        &mut self,
        tx: &mut Tx<'_>,
        creator_id: i64,
        attributes: NewMessage,
    ) -> Result<Message> {
        self.post_message_with_agent_delivery(tx, creator_id, attributes, false)
    }

    fn post_message_with_agent_delivery(
        &mut self,
        tx: &mut Tx<'_>,
        creator_id: i64,
        attributes: NewMessage,
        defer_agent_delivery: bool,
    ) -> Result<Message> {
        self.reload(tx.conn())?;
        if self.locked_at.is_some() {
            return Err(Error::Other(LOCKED_MESSAGE.into()));
        }
        Membership::find_by_room_and_user(tx.conn(), self.room_id, creator_id)?
            .or_not_found("Membership")?;
        ThreadMembership::join(tx, self.id, creator_id)?;
        let mut changed = self.clone();
        changed.closed_at = None;
        changed.last_activity_at = tx.now();
        self.save(tx, changed)?;
        Message::create_with_agent_delivery(
            tx,
            NewMessage {
                room_id: self.room_id,
                thread_id: Some(self.id),
                creator_id,
                ..attributes
            },
            defer_agent_delivery,
        )
    }

    /// `receive(message)`, from the message's `after_create_commit`: every other thread member
    /// who is still in the room goes unread (`update_columns`) and hears about it on their unread
    /// threads stream. The push job is persisted by `push_later` in the message's transaction.
    /// A system note does none of it.
    pub(crate) fn receive(tx: &mut Tx<'_>, thread_id: i64, message: &Message) -> Result<()> {
        if message.system_note {
            return Ok(());
        }
        let Some(thread) = Self::find_by_id(tx.conn(), thread_id)? else {
            return Ok(());
        };
        let now = tx.now();
        let mut unread_user_ids = Vec::new();
        for membership in ThreadMembership::for_thread(tx.conn(), thread.id)? {
            if membership.user_id == message.creator_id
                || Membership::find_by_room_and_user(tx.conn(), thread.room_id, membership.user_id)?
                    .is_none()
            {
                continue;
            }
            tx.conn().execute_cached(
                r#"UPDATE "thread_memberships" SET "unread_at" = ?, "updated_at" = ? WHERE "thread_memberships"."id" = ?"#,
                params![message.created_at, now, membership.id],
            )?;
            unread_user_ids.push(membership.user_id);
        }
        let mut seen = HashSet::new();
        for user_id in unread_user_ids.into_iter().filter(|id| seen.insert(*id)) {
            tx.emit_after_commit(Event::broadcast(&Broadcast::Cable {
                stream: format!("user_{user_id}_unread_threads"),
                payload: serde_json::json!({ "threadId": thread.id, "roomId": thread.room_id }),
            }));
        }
        Ok(())
    }

    /// Persist the receive callback's job in the triggering write. The runner is woken after
    /// the unread broadcasts, when the transaction commits (WS3's EventSink::persist).
    pub(crate) fn push_later(tx: &mut Tx<'_>, thread_id: i64, message: &Message) {
        if !message.system_note {
            tx.emit_after_commit(Event::job(&PushMessageJob {
                thread_id,
                message_id: message.id,
            }));
        }
    }

    /// `ChannelThread.refresh_messages_count(thread_id)`: recount with `REPLY_COUNT_SQL`, then
    /// bump the parent message so its indicator re-renders on refresh. Both skip callbacks and
    /// leave the thread's own `updated_at` alone. A deleted thread is a no-op.
    pub fn refresh_messages_count(tx: &Tx<'_>, thread_id: i64) -> Result<()> {
        tx.conn().execute_cached(
            &format!(
                r#"UPDATE "channel_threads" SET {REPLY_COUNT_SQL} WHERE "channel_threads"."id" = ?"#
            ),
            [thread_id],
        )?;
        tx.conn().execute_cached(
            r#"UPDATE "messages" SET "updated_at" = ? WHERE "messages"."id" IN (SELECT "channel_threads"."parent_message_id" FROM "channel_threads" WHERE "channel_threads"."id" = ? AND "channel_threads"."parent_message_id" IS NOT NULL)"#,
            params![tx.now(), thread_id],
        )?;
        Ok(())
    }

    /// `broadcast_thread_indicator_change`: replace the parent message's "N replies" indicator
    /// on its room's message stream, with the committed count (0 once the thread is gone).
    pub(crate) fn broadcast_thread_indicator_change(
        tx: &mut Tx<'_>,
        parent_message_id: Option<i64>,
        reply_count: i64,
    ) -> Result<()> {
        let Some(parent) = parent_message_id
            .map(|id| Message::find_by_id(tx.conn(), id))
            .transpose()?
            .flatten()
        else {
            return Ok(());
        };
        let room = Room::find(tx.conn(), parent.room_id)?;
        tx.emit_after_commit(Event::broadcast(&thread_indicator_broadcast(
            &room,
            &parent,
            reply_count,
        )));
        Ok(())
    }

    /// `broadcast_thread_indicator` on the messages of `thread_ids`, loaded fresh after commit.
    pub(crate) fn broadcast_thread_indicators(tx: &mut Tx<'_>, thread_ids: &[i64]) -> Result<()> {
        for &thread_id in thread_ids {
            if let Some(thread) = Self::find_by_id(tx.conn(), thread_id)? {
                Self::broadcast_thread_indicator_change(
                    tx,
                    thread.parent_message_id,
                    thread.messages_count,
                )?;
            }
        }
        Ok(())
    }

    // Destroying

    /// `destroy`: its tags, its messages (each destroyed, without recounting this thread), its
    /// memberships and the other dependents' rows, then the thread; the parent message is stamped
    /// (`after_destroy :stamp_parent_message`) and its indicator hidden after commit.
    ///
    /// Work/SLA dependents commit with deletion. The agent ledger runs after
    /// commit; a captured deletion job keeps its durable enqueue atomic without
    /// reserving an event ID ahead of ledger publication.
    pub fn destroy(&self, tx: &mut Tx<'_>) -> Result<()> {
        self.destroy_by(tx, None)
    }

    pub fn destroy_by(&self, tx: &mut Tx<'_>, deleted_by_id: Option<i64>) -> Result<()> {
        self.destroy_inner(tx, false, deleted_by_id)
    }

    /// Slack undo propagates `importing` to dependent messages, suppressing delivery callbacks.
    pub fn destroy_imported(&self, tx: &mut Tx<'_>) -> Result<()> {
        self.destroy_inner(tx, true, None)
    }

    fn destroy_inner(
        &self,
        tx: &mut Tx<'_>,
        importing: bool,
        deleted_by_id: Option<i64>,
    ) -> Result<()> {
        let fresh = Self::find(tx.conn(), self.id)?;
        tx.register_record("channel_threads", self.id);
        let snapshot = super::agent_work_events::capture_deleted(tx, &fresh, deleted_by_id)?;
        crate::ScheduledMessage::drop_for_thread(tx, self.id, importing)?;
        // Rails suppresses a dependent tag's row replacement while its parent is destroyed.
        tx.conn().execute_cached(
            "DELETE FROM thread_tags WHERE channel_thread_id=?",
            [self.id],
        )?;
        for message in Message::in_thread(tx.conn(), self.id)? {
            if importing {
                message.destroy_imported_with_conversation(tx)?;
            } else {
                message.destroy_with_conversation(tx)?;
            }
        }
        // WorkThreadEvent's dependent inbox rows must be destroyed before its FK cascade.
        for sql in [
            "DELETE FROM activity_items WHERE source_type='WorkThreadEvent' AND source_id IN (SELECT id FROM work_thread_events WHERE channel_thread_id=?) RETURNING id, user_id",
            "DELETE FROM activity_items WHERE source_type='BoardSlaNudge' AND source_id IN (SELECT id FROM board_sla_nudges WHERE channel_thread_id=?) RETURNING id, user_id",
        ] {
            let removed = crate::sql::query_all(tx.conn(), sql, [self.id], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })?;
            if !importing {
                crate::ActivityItem::emit_removed(tx, removed);
            }
        }
        for sql in [
            r#"DELETE FROM "github_pull_request_threads" WHERE "channel_thread_id" = ?"#,
            r#"DELETE FROM "thread_memberships" WHERE "thread_id" = ?"#,
            r#"DELETE FROM "work_thread_events" WHERE "channel_thread_id" = ?"#,
            r#"DELETE FROM "work_thread_links" WHERE "channel_thread_id" = ?"#,
            r#"DELETE FROM "work_handoffs" WHERE "channel_thread_id" = ?"#,
            r#"DELETE FROM "board_sla_nudges" WHERE "channel_thread_id" = ?"#,
            r#"DELETE FROM "agent_steps" WHERE "channel_thread_id" = ?"#,
        ] {
            tx.conn().execute_cached(sql, [self.id])?;
        }
        tx.conn().execute_cached(
            r#"DELETE FROM "channel_threads" WHERE "channel_threads"."id" = ?"#,
            [self.id],
        )?;
        if let Some(parent_id) = self.parent_message_id.filter(|_| !importing) {
            tx.conn().execute_cached(
                r#"UPDATE "messages" SET "updated_at" = ? WHERE "messages"."id" = ?"#,
                params![tx.now(), parent_id],
            )?;
        }
        fresh.register_board_destruction(tx)?;
        if !importing {
            let parent_message_id = self.parent_message_id;
            // Rails registers the indicator before emit_deleted_work_unassigned.
            // Slack undo suppresses the indicator along with dependent delivery.
            tx.after_commit_record("channel_threads", self.id, move |tx| {
                Self::broadcast_thread_indicator_change(tx, parent_message_id, 0)
            });
        }
        super::agent_work_events::record_deleted(tx, &fresh, deleted_by_id, snapshot)?;
        Ok(())
    }

    // Reading

    pub fn room(&self, conn: &Connection) -> Result<Room> {
        Room::find(conn, self.room_id)
    }

    pub fn tags(&self, conn: &Connection) -> Result<Vec<ThreadTag>> {
        ThreadTag::for_thread(conn, self.id)
    }

    pub fn tag_names(&self, conn: &Connection) -> Result<Vec<String>> {
        Ok(self.tags(conn)?.into_iter().map(|tag| tag.name).collect())
    }

    /// `membership_for(user)`
    pub fn membership_for(
        &self,
        conn: &Connection,
        user_id: i64,
    ) -> Result<Option<ThreadMembership>> {
        ThreadMembership::find_by_thread_and_user(conn, self.id, user_id)
    }

    /// `unread_for?(user)`
    pub fn unread_for(&self, conn: &Connection, user_id: i64) -> Result<bool> {
        Ok(self
            .membership_for(conn, user_id)?
            .is_some_and(|membership| membership.unread()))
    }

    /// `message_count`: every message in the thread (`messages.count`).
    pub fn message_count(&self, conn: &Connection) -> Result<i64> {
        sql::count(
            conn,
            r#"SELECT COUNT(*) FROM "messages" WHERE "messages"."thread_id" = ?"#,
            [self.id],
        )
    }

    /// `manageable_by?(user)`: an administrator or the room's creator.
    pub fn manageable_by(&self, conn: &Connection, user: &User) -> Result<bool> {
        Ok(user.is_administrator() || self.manageable_in_room(&self.room(conn)?, user))
    }

    pub fn manageable_in_room(&self, room: &Room, user: &User) -> bool {
        user.is_administrator() || user.id == room.creator_id
    }
    pub fn settings_manageable_in_room(&self, room: &Room, user: &User) -> bool {
        self.manageable_in_room(room, user) || user.id == self.creator_id
    }

    /// `settings_manageable_by?(user)`: also the thread's creator.
    pub fn settings_manageable_by(&self, conn: &Connection, user: &User) -> Result<bool> {
        Ok(self.manageable_by(conn, user)? || user.id == self.creator_id)
    }

    pub fn reload(&mut self, conn: &Connection) -> Result<()> {
        *self = Self::find(conn, self.id)?;
        Ok(())
    }

    // ChannelThread::MessagePusher

    /// `ChannelThread::MessagePusher#recipients`: every thread member but the author, with
    /// their room membership, whether the message mentions them, and whether it replies to them
    /// and notifies them. Whether each is pushed is `Notifications::Policy`'s call (WS17), which
    /// [`ChannelThread::push_recipients`] takes as `policy`.
    pub fn push_candidates(
        conn: &Connection,
        rich_text: &dyn RichText,
        thread: &ChannelThread,
        message: &Message,
    ) -> Result<Vec<ThreadPushCandidate>> {
        let memberships: Vec<ThreadMembership> = query_all(
            conn,
            r#"SELECT "thread_memberships".* FROM "thread_memberships" WHERE "thread_memberships"."thread_id" = ? AND "thread_memberships"."user_id" != ? ORDER BY "thread_memberships"."id" ASC"#,
            [thread.id, message.creator_id],
            ThreadMembership::from_row,
        )?;
        if memberships.is_empty() {
            return Ok(Vec::new());
        }
        let user_ids: Vec<i64> = memberships.iter().map(|m| m.user_id).collect();
        let mut room_memberships: HashMap<i64, Membership> = HashMap::new();
        let sql = format!(
            r#"SELECT "memberships".* FROM "memberships" WHERE "memberships"."room_id" = ? AND "memberships"."user_id" IN ({})"#,
            placeholders(user_ids.len())
        );
        let values: Vec<i64> = std::iter::once(thread.room_id)
            .chain(user_ids.iter().copied())
            .collect();
        for membership in query_all(
            conn,
            &sql,
            rusqlite::params_from_iter(values),
            Membership::from_row,
        )? {
            room_memberships.insert(membership.user_id, membership);
        }
        let mention_ids: HashSet<i64> = message
            .mentionees(conn, rich_text)?
            .into_iter()
            .map(|user| user.id)
            .collect();
        let reply_author_id = match message.reply_to_message_id {
            Some(id) => Message::find_by_id(conn, id)?.map(|source| source.creator_id),
            None => None,
        };
        let mut candidates = Vec::new();
        let mut users: HashMap<i64, User> = User::where_ids(conn, &user_ids)?
            .into_iter()
            .map(|user| (user.id, user))
            .collect();
        for membership in memberships {
            let recipient = users.remove(&membership.user_id).or_not_found("User")?;
            candidates.push(ThreadPushCandidate {
                mentioned: mention_ids.contains(&membership.user_id),
                reply_to_recipient: Some(membership.user_id) == reply_author_id
                    && message.reply_notify_author,
                room_membership: room_memberships.remove(&membership.user_id),
                thread_membership: membership,
                recipient,
            });
        }
        Ok(candidates)
    }

    /// `ChannelThread::MessagePusher#push`: the pushes to send, one per recipient `policy`
    /// allows who has a push subscription.
    pub fn push_recipients(
        conn: &Connection,
        rich_text: &dyn RichText,
        thread_id: i64,
        message_id: i64,
        policy: &dyn Fn(&ThreadPushCandidate) -> bool,
    ) -> Result<Vec<ThreadPush>> {
        let (Some(thread), Some(message)) = (
            Self::find_by_id(conn, thread_id)?,
            Message::find_by_id(conn, message_id)?,
        ) else {
            return Ok(Vec::new());
        };
        let candidates: Vec<ThreadPushCandidate> =
            Self::push_candidates(conn, rich_text, &thread, &message)?
                .into_iter()
                .filter(|c| policy(c))
                .collect();
        if candidates.is_empty() {
            return Ok(Vec::new());
        }
        let creator = message.creator(conn)?;
        let body = format!(
            "{}: {}",
            creator.name,
            message.plain_text_body(conn, rich_text)?
        );
        let path = format!(
            "/rooms/{}?message_id={}&thread={}",
            thread.room_id, message.id, thread.id
        );
        let mut pushes = Vec::new();
        for candidate in candidates {
            let subscriptions = PushSubscription::for_user(conn, candidate.recipient.id)?;
            if subscriptions.is_empty() {
                continue;
            }
            let title = if candidate.reply_to_recipient {
                format!("Reply in {}", thread.name)
            } else {
                thread.name.clone()
            };
            pushes.push(ThreadPush {
                user_id: candidate.recipient.id,
                payload: PushPayload::new(
                    title,
                    body.clone(),
                    path.clone(),
                    Some(format!("room-{}", thread.room_id)),
                ),
                tag: format!("room-{}", thread.room_id),
                subscriptions,
            });
        }
        Ok(pushes)
    }

    /// The production policy, with status/cache and DND exceptions preloaded once per batch.
    pub fn push_recipients_with_policy(
        conn: &Connection,
        rich_text: &dyn RichText,
        thread_id: i64,
        message_id: i64,
        now: Timestamp,
    ) -> Result<Vec<ThreadPush>> {
        let Some(message) = Message::find_by_id(conn, message_id)? else {
            return Ok(Vec::new());
        };
        let ids: Vec<i64> = query_all(
            conn,
            "SELECT user_id FROM thread_memberships WHERE thread_id=?",
            [thread_id],
            |row| row.get(0),
        )?;
        let users = crate::UserStatusSettings::for_ids(conn, &ids)?;
        let exceptions =
            super::notification_policy::dnd_exceptions_for(conn, &ids, Some(message.creator_id))?;
        Self::push_recipients(conn, rich_text, thread_id, message_id, &|candidate| {
            crate::NotificationPolicy {
                recipient: users.get(&candidate.recipient.id),
                kind: crate::NotificationKind::ThreadMessage,
                room_involvement: candidate.room_membership.as_ref().map(|m| m.involvement),
                thread_involvement: Some(candidate.thread_membership.involvement),
                mentioned: candidate.mentioned,
                reply_to_recipient: candidate.reply_to_recipient,
                keyword_matched: false,
                dnd_exception: exceptions.contains(&candidate.recipient.id),
                now,
            }
            .push()
        })
    }
}

/// `Notifications::Policy#thread_base_push?` for a thread message: the involvement rules, before
/// DND, quiet hours and out-of-office (which, with the rest of the policy, are WS17's).
pub fn thread_base_push(candidate: &ThreadPushCandidate) -> bool {
    use crate::models::Involvement;
    let Some(room_membership) = &candidate.room_membership else {
        return false;
    };
    let room_involvement = room_membership.involvement;
    if matches!(
        room_involvement,
        Some(Involvement::Invisible | Involvement::Nothing)
    ) {
        return false;
    }
    let thread_involvement = candidate.thread_membership.involvement;
    if thread_involvement == ThreadInvolvement::Nothing {
        return false;
    }
    // `thread_mentions_enabled?`: anything but a muted thread.
    let mentions_enabled = thread_involvement != ThreadInvolvement::Nothing;
    if room_involvement == Some(Involvement::Muted) {
        return candidate.mentioned && mentions_enabled;
    }
    thread_involvement == ThreadInvolvement::Everything || (candidate.mentioned && mentions_enabled)
}

/// The indicator replace (`messages/_thread_indicator`) for a thread's parent message.
pub fn thread_indicator_broadcast(room: &Room, parent: &Message, reply_count: i64) -> Broadcast {
    Broadcast::replace_keeping_scroll(
        room_messages(room),
        dom_id(
            "message",
            &parent.client_message_id,
            Some("thread_indicator"),
        ),
        Partial::ThreadIndicator {
            message_id: parent.id,
            reply_count,
        },
    )
}

/// `set_default_name`: the parent message's first line, truncated to 100 with "…", else
/// "New thread".
fn default_name(
    conn: &Connection,
    rich_text: &dyn RichText,
    parent_message_id: Option<i64>,
) -> Result<String> {
    let source = match parent_message_id
        .map(|id| Message::find_by_id(conn, id))
        .transpose()?
        .flatten()
    {
        Some(parent) => parent.plain_text_body(conn, rich_text)?,
        None => String::new(),
    };
    let first_line = source
        .split_inclusive('\n')
        .next()
        .unwrap_or("")
        .trim()
        .to_string();
    let name = truncate(&first_line, NAME_LIMIT, "…");
    Ok(if name.is_empty() {
        "New thread".into()
    } else {
        name
    })
}

/// `String#truncate(limit, omission:)`: at most `limit` characters, the omission included.
pub fn truncate(text: &str, limit: usize, omission: &str) -> String {
    if text.chars().count() <= limit {
        return text.to_string();
    }
    let keep = limit.saturating_sub(omission.chars().count());
    let mut out: String = text.chars().take(keep).collect();
    out.push_str(omission);
    out
}

/// `tag_names=`: stripped, downcased, blanks dropped, deduplicated in order.
pub fn normalize_tag_names(names: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for name in names {
        let name = rails_compat::unicode::downcase(campfire_richtext::ruby::strip(name));
        if !campfire_richtext::ruby::is_blank(&name) && !out.contains(&name) {
            out.push(name);
        }
    }
    out
}
