//! `reference/app/models/saved_item.rb`, `saved_item/reminder_dispatcher.rb`,
//! `saved_item/reminder_pusher.rb` and `app/jobs/saved_item/reminder_push_job.rb`: a user's
//! saved-for-later messages, with an optional reminder that fires once into the inbox and a push.

use rusqlite::{Connection, Row, params};
use serde::{Deserialize, Serialize};

use crate::database::Tx;
use crate::error::{Errors, OptionalExt, Result};
use crate::events::{Event, Job};
use crate::models::activity_item::ActivityItem;
use crate::models::message_pin::message_path;
use crate::models::{Message, PushPayload, PushSubscription, Room, User};
use crate::rich_text::RichText;
use crate::sql::{self, CachedStatements, query_all, query_one};
use crate::time::Timestamp;

/// `ActivityItem#source_type` for a saved item.
pub const SOURCE_TYPE: &str = "SavedItem";
/// `enum :status, %w[ in_progress done ]`
pub const STATUSES: [&str; 2] = ["in_progress", "done"];

#[derive(Debug, Clone, PartialEq)]
pub struct SavedItem {
    pub id: i64,
    pub user_id: i64,
    pub message_id: i64,
    pub status: String,
    pub remind_at: Option<Timestamp>,
    pub reminded_at: Option<Timestamp>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// `SavedItem::ReminderPushJob.perform_later(saved_item)`, from the dispatcher's claim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReminderPushJob {
    pub saved_item_id: i64,
}

impl Job for ReminderPushJob {
    const CLASS: &'static str = "SavedItem::ReminderPushJob";
}

/// What `SavedItem::ReminderPusher#push` hands the Web Push pool.
#[derive(Debug, Clone, PartialEq)]
pub struct ReminderPush {
    pub user_id: i64,
    pub payload: PushPayload,
    /// `tag: "saved-#{saved_item.id}"`
    pub tag: String,
    pub subscriptions: Vec<PushSubscription>,
}

const SELECT_DUE: &str = r#"SELECT "saved_items".* FROM "saved_items" WHERE "saved_items"."remind_at" IS NOT NULL AND "saved_items"."reminded_at" IS NULL AND "saved_items"."remind_at" <= ?"#;

impl SavedItem {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            user_id: row.get("user_id")?,
            message_id: row.get("message_id")?,
            status: row.get("status")?,
            remind_at: row.get("remind_at")?,
            reminded_at: row.get("reminded_at")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    pub fn find(conn: &Connection, id: i64) -> Result<Self> {
        Self::find_by_id(conn, id)?.or_not_found("SavedItem")
    }

    pub fn find_by_id(conn: &Connection, id: i64) -> Result<Option<Self>> {
        query_one(conn, r#"SELECT "saved_items".* FROM "saved_items" WHERE "saved_items"."id" = ? LIMIT 1"#, [id], Self::from_row)
    }

    pub fn find_by_user_and_message(conn: &Connection, user_id: i64, message_id: i64) -> Result<Option<Self>> {
        query_one(
            conn,
            r#"SELECT "saved_items".* FROM "saved_items" WHERE "saved_items"."user_id" = ? AND "saved_items"."message_id" = ? LIMIT 1"#,
            params![user_id, message_id],
            Self::from_row,
        )
    }

    pub fn in_progress(&self) -> bool {
        self.status == "in_progress"
    }

    /// `reminder_pending?`
    pub fn reminder_pending(&self) -> bool {
        self.remind_at.is_some() && self.reminded_at.is_none()
    }

    /// `reminder_fired?`
    pub fn reminder_fired(&self) -> bool {
        self.reminded_at.is_some()
    }

    /// `SavedItem.accessible_to(user)`, `ordered`: only an active human's items, in alive rooms
    /// they're still a member of (re-checked at view time, so regaining access shows an item
    /// again).
    pub fn accessible_to(conn: &Connection, user_id: i64) -> Result<Vec<Self>> {
        let Some(user) = User::find_by_id(conn, user_id)? else { return Ok(Vec::new()) };
        if !user.is_active() || user.is_bot() {
            return Ok(Vec::new());
        }
        query_all(
            conn,
            r#"SELECT "saved_items".* FROM "saved_items" INNER JOIN "messages" ON "messages"."id" = "saved_items"."message_id" INNER JOIN "rooms" ON "rooms"."id" = "messages"."room_id" INNER JOIN memberships AS saved_item_memberships ON saved_item_memberships.room_id = messages.room_id AND saved_item_memberships.user_id = ? WHERE "rooms"."deleted_at" IS NULL AND "saved_items"."user_id" = ? ORDER BY "saved_items"."created_at" DESC, "saved_items"."id" DESC"#,
            params![user_id, user_id],
            Self::from_row,
        )
    }

    /// `SavedItem.due_reminders(now)`: pending reminders whose time has come.
    pub fn due_reminders(conn: &Connection, now: Timestamp) -> Result<Vec<Self>> {
        query_all(conn, &format!(r#"{SELECT_DUE} ORDER BY "saved_items"."id" ASC"#), [now], Self::from_row)
    }

    /// `Current.user.saved_items.find_or_initialize_by(message:)`, `remind_at =`, `save!`: saving
    /// is idempotent per user and message, and re-saving sets the reminder.
    pub fn save_for(tx: &mut Tx<'_>, user_id: i64, message_id: i64, remind_at: Option<Timestamp>) -> Result<Self> {
        match Self::find_by_user_and_message(tx.conn(), user_id, message_id)? {
            Some(mut item) => {
                item.update(tx, SavedItemChanges { remind_at: Some(remind_at), status: None })?;
                Ok(item)
            }
            None => Self::create(tx, NewSavedItem { user_id, message_id, remind_at, status: None }),
        }
    }

    /// `SavedItem.create!`
    pub fn create(tx: &mut Tx<'_>, attributes: NewSavedItem) -> Result<Self> {
        let status = attributes.status.unwrap_or_else(|| "in_progress".into());
        let now = tx.now();
        let mut errors = Errors::default();
        if User::find_by_id(tx.conn(), attributes.user_id)?.is_none() {
            errors.add("user", "must exist");
        }
        if Message::find_by_id(tx.conn(), attributes.message_id)?.is_none() {
            errors.add("message", "must exist");
        }
        Self::validate_status(&mut errors, &status);
        if Self::find_by_user_and_message(tx.conn(), attributes.user_id, attributes.message_id)?.is_some() {
            errors.add("message_id", "has already been taken");
        }
        // `if: :will_save_change_to_remind_at?`: a new nil isn't a change.
        Self::validate_remind_at(&mut errors, attributes.remind_at, now);
        errors.into_result()?;
        let id: i64 = tx.conn().query_row_cached(
            r#"INSERT INTO "saved_items" ("created_at", "message_id", "remind_at", "status", "updated_at", "user_id") VALUES (?, ?, ?, ?, ?, ?) RETURNING "id""#,
            params![now, attributes.message_id, attributes.remind_at, status, now, attributes.user_id],
            |r| r.get(0),
        )?;
        Self::find(tx.conn(), id)
    }

    /// `update!`: the status, and a new reminder time, which re-arms the reminder
    /// (`clear_fired_claim`). An unchanged save writes nothing.
    pub fn update(&mut self, tx: &mut Tx<'_>, changes: SavedItemChanges) -> Result<()> {
        let now = tx.now();
        let status = changes.status.unwrap_or_else(|| self.status.clone());
        let remind_at = changes.remind_at.unwrap_or(self.remind_at);
        let remind_at_changed = remind_at != self.remind_at;
        let mut errors = Errors::default();
        Self::validate_status(&mut errors, &status);
        if remind_at_changed {
            Self::validate_remind_at(&mut errors, remind_at, now);
        }
        errors.into_result()?;
        if status == self.status && !remind_at_changed {
            return Ok(());
        }
        let reminded_at = if remind_at_changed { None } else { self.reminded_at };
        tx.conn().execute_cached(
            r#"UPDATE "saved_items" SET "remind_at" = ?, "reminded_at" = ?, "status" = ?, "updated_at" = ? WHERE "saved_items"."id" = ?"#,
            params![remind_at, reminded_at, status, now, self.id],
        )?;
        *self = Self::find(tx.conn(), self.id)?;
        Ok(())
    }

    /// `validates :status` (the enum's `validate: true`)
    fn validate_status(errors: &mut Errors, status: &str) {
        if !STATUSES.contains(&status) {
            errors.add("status", "is not included in the list");
        }
    }

    /// `remind_at_must_be_future`
    fn validate_remind_at(errors: &mut Errors, remind_at: Option<Timestamp>, now: Timestamp) {
        if remind_at.is_some_and(|at| at <= now) {
            errors.add("remind_at", "must be in the future");
        }
    }

    /// `create_reminder_item!`: the reminder's own inbox item, sourced on the saved item (never
    /// converting a mention or reply item for the message), refreshed unread in place when a
    /// re-armed reminder fires again.
    pub fn create_reminder_item(&self, tx: &mut Tx<'_>) -> Result<ActivityItem> {
        ActivityItem::refresh_unread(tx, self.user_id, SOURCE_TYPE, self.id, "message_reminder")
    }

    /// `destroy!`: its activity items first (`dependent: :destroy`).
    pub fn destroy(&self, tx: &mut Tx<'_>) -> Result<()> {
        ActivityItem::destroy_for_source(tx, SOURCE_TYPE, self.id)?;
        tx.conn().execute_cached(r#"DELETE FROM "saved_items" WHERE "saved_items"."id" = ?"#, [self.id])?;
        Ok(())
    }

    /// `has_many :saved_items, dependent: :destroy` on the message.
    pub(crate) fn destroy_for_message(tx: &mut Tx<'_>, message_id: i64) -> Result<()> {
        let items = query_all(
            tx.conn(),
            r#"SELECT "saved_items".* FROM "saved_items" WHERE "saved_items"."message_id" = ?"#,
            [message_id],
            Self::from_row,
        )?;
        for item in items {
            item.destroy(tx)?;
        }
        Ok(())
    }

    /// WS8bm2 reminder job checks membership before evaluating recipient policy.
    pub fn reminder_room_member(&self, conn: &Connection) -> Result<bool> {
        Self::room_member(conn, self.message_id, self.user_id)
    }

    fn room_member(conn: &Connection, message_id: i64, user_id: i64) -> Result<bool> {
        sql::exists(
            conn,
            r#"SELECT 1 AS one FROM "memberships" INNER JOIN "messages" ON "messages"."room_id" = "memberships"."room_id" WHERE "messages"."id" = ? AND "memberships"."user_id" = ? LIMIT 1"#,
            params![message_id, user_id],
        )
    }

    // SavedItem::ReminderDispatcher

    /// `SavedItem::ReminderDispatcher.dispatch_due!(now:)` in one transaction; the ids notified.
    /// The periodic task claims each due reminder in its own write instead
    /// ([`SavedItem::due_reminder_ids`], then [`SavedItem::dispatch_reminder`]), so one failure
    /// is logged and skipped, as Ruby rescues per item.
    pub fn dispatch_due(tx: &mut Tx<'_>, now: Timestamp) -> Result<Vec<i64>> {
        let mut notified = Vec::new();
        for id in Self::due_reminder_ids(tx.conn(), now)? {
            if Self::dispatch_reminder(tx, id, now)? {
                notified.push(id);
            }
        }
        Ok(notified)
    }

    /// `due_items(now)`: due reminders of active humans in alive rooms.
    pub fn due_reminder_ids(conn: &Connection, now: Timestamp) -> Result<Vec<i64>> {
        query_all(
            conn,
            r#"SELECT "saved_items"."id" FROM "saved_items" INNER JOIN "users" ON "users"."id" = "saved_items"."user_id" INNER JOIN "messages" ON "messages"."id" = "saved_items"."message_id" INNER JOIN "rooms" ON "rooms"."id" = "messages"."room_id" WHERE "saved_items"."remind_at" IS NOT NULL AND "saved_items"."reminded_at" IS NULL AND "saved_items"."remind_at" <= ? AND "users"."status" = 0 AND "users"."role" != 2 AND "rooms"."deleted_at" IS NULL ORDER BY "saved_items"."id" ASC"#,
            [now],
            |r| r.get(0),
        )
    }

    /// `dispatch_item!`, inside its own write transaction (`with_lock`): re-reads the row and
    /// claims it through `reminded_at`, so a second run or runner never fires it twice. A member
    /// gets the inbox item and, after commit, the push job; a saver who lost room access is
    /// claimed without notice; a reminder moved or cleared since selection is left for its new
    /// time. Returns whether it notified.
    pub fn dispatch_reminder(tx: &mut Tx<'_>, id: i64, now: Timestamp) -> Result<bool> {
        let Some(item) = Self::find_by_id(tx.conn(), id)? else { return Ok(false) };
        if item.reminded_at.is_some() {
            return Ok(false);
        }
        if item.remind_at.is_none_or(|at| at > now) {
            return Ok(false);
        }
        let member = Self::room_member(tx.conn(), item.message_id, item.user_id)?;
        if member {
            item.create_reminder_item(tx)?;
        }
        // `update!(reminded_at: now)`: nothing else changes, so only the claim is validated.
        tx.conn().execute_cached(
            r#"UPDATE "saved_items" SET "reminded_at" = ?, "updated_at" = ? WHERE "saved_items"."id" = ?"#,
            params![now, tx.now(), item.id],
        )?;
        if member {
            tx.emit_after_commit(Event::job(&ReminderPushJob { saved_item_id: item.id }));
        }
        Ok(member)
    }

    // SavedItem::ReminderPusher

    /// `SavedItem::ReminderPusher#push` (what `ReminderPushJob` performs): the reminder push for
    /// the saver, only while they're still a room member and `policy` (`Notifications::Policy`
    /// with `kind: :reminder`: DND and quiet hours, WS17's) allows it. `None` when nothing goes
    /// out; subscriptions may be empty, as the pool is handed them regardless.
    pub fn reminder_push(
        conn: &Connection,
        rich_text: &dyn RichText,
        id: i64,
        policy: &dyn Fn(&User) -> bool,
    ) -> Result<Option<ReminderPush>> {
        let Some(item) = Self::find_by_id(conn, id)? else { return Ok(None) };
        if !Self::room_member(conn, item.message_id, item.user_id)? {
            return Ok(None);
        }
        if !policy(&User::find(conn, item.user_id)?) {
            return Ok(None);
        }
        let message = Message::find(conn, item.message_id)?;
        let room = Room::find(conn, message.room_id)?;
        let title = if room.direct() { message.creator(conn)?.name } else { room.name.clone().unwrap_or_default() };
        let text = crate::models::channel_thread::truncate(&message.plain_text_body(conn, rich_text)?, 140, "...");
        Ok(Some(ReminderPush {
            user_id: item.user_id,
            payload: PushPayload::new(title, format!("Reminder: {text}"), message_path(&message), Some(format!("saved-{}", item.id))),
            tag: format!("saved-{}", item.id),
            subscriptions: PushSubscription::for_user(conn, item.user_id)?,
        }))
    }

    pub fn reminder_push_with_policy(conn: &Connection, rich_text: &dyn RichText, id: i64, now: Timestamp) -> Result<Option<ReminderPush>> {
        let Some(item) = Self::find_by_id(conn, id)? else { return Ok(None) };
        let users = crate::UserStatusSettings::for_ids(conn, &[item.user_id])?;
        Self::reminder_push(conn, rich_text, id, &|user| crate::NotificationPolicy {
            recipient: users.get(&user.id), kind: crate::NotificationKind::Reminder,
            room_involvement: None, thread_involvement: None, mentioned: false, reply_to_recipient: false,
            keyword_matched: false, dnd_exception: false, now,
        }.push())
    }
}

/// `SavedItem.create!` attributes.
#[derive(Debug, Clone, Default)]
pub struct NewSavedItem {
    pub user_id: i64,
    pub message_id: i64,
    pub remind_at: Option<Timestamp>,
    /// `None` is the column default, `in_progress`.
    pub status: Option<String>,
}

/// `update!` attributes; `None` leaves one alone. `remind_at: Some(None)` clears the reminder.
#[derive(Debug, Clone, Default)]
pub struct SavedItemChanges {
    pub status: Option<String>,
    pub remind_at: Option<Option<Timestamp>>,
}
