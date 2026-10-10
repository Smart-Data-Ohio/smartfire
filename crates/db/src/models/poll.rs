//! `reference/app/models/poll.rb`, `poll_option.rb` and `poll_vote.rb`: a poll rides on a saved
//! message (its question is the message's text), with 2 to 10 options, single or multiple
//! choice, optionally anonymous, optionally closing at a set time. `Poll.close_due!` runs from
//! the periodic runner ("poll closing", every `EVENT_REMINDERS_INTERVAL`).

use rusqlite::{Connection, Row, params};
use serde_json::json;

use crate::database::Tx;
use crate::error::{Error, Errors, OptionalExt, Result};
use crate::events::Event;
use crate::models::{Message, User};
use crate::sql::{self, CachedStatements, placeholders, query_all, query_one};
use crate::time::Timestamp;

/// `Poll::MIN_OPTIONS`
pub const MIN_OPTIONS: usize = 2;
/// `Poll::MAX_OPTIONS`
pub const MAX_OPTIONS: usize = 10;
/// `PollOption::LABEL_LIMIT`
pub const LABEL_LIMIT: usize = 200;

#[derive(Debug, Clone, PartialEq)]
pub struct Poll {
    pub id: i64,
    pub message_id: i64,
    pub multiple: bool,
    pub anonymous: bool,
    pub closes_at: Option<Timestamp>,
    pub closed_at: Option<Timestamp>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PollOption {
    pub id: i64,
    pub poll_id: i64,
    pub label: String,
    pub position: i64,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PollVote {
    pub id: i64,
    pub poll_id: i64,
    pub poll_option_id: i64,
    pub user_id: i64,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// Sync-only (the classic app has no broadcast for it): the poll's counts changed, from a vote
/// or a close. `voter_id` is who voted, for their own `poll.ballot`; `None` when it closed.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PollChanged {
    pub poll_id: i64,
    pub voter_id: Option<i64>,
}

impl crate::events::Broadcast for PollChanged {
    const KIND: &'static str = "Poll#sync_changed";
}

/// `create_for_message!`'s options.
#[derive(Debug, Clone, Default)]
pub struct NewPoll {
    pub labels: Vec<String>,
    pub multiple: bool,
    pub anonymous: bool,
    pub closes_at: Option<Timestamp>,
}

fn invalid(attribute: &'static str, message: &str) -> Error {
    let mut errors = Errors::default();
    errors.add(attribute, message);
    Error::RecordInvalid(errors)
}

impl Poll {
    pub(crate) fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            message_id: row.get("message_id")?,
            multiple: row.get("multiple")?,
            anonymous: row.get("anonymous")?,
            closes_at: row.get("closes_at")?,
            closed_at: row.get("closed_at")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    pub fn find(conn: &Connection, id: i64) -> Result<Self> {
        query_one(conn, r#"SELECT "polls".* FROM "polls" WHERE "polls"."id" = ? LIMIT 1"#, [id], Self::from_row)?.or_not_found("Poll")
    }

    /// `message.poll`
    pub fn find_by_message(conn: &Connection, message_id: i64) -> Result<Option<Self>> {
        query_one(conn, r#"SELECT "polls".* FROM "polls" WHERE "polls"."message_id" = ? LIMIT 1"#, [message_id], Self::from_row)
    }

    /// `Poll.joins(:message).where(messages: { room_id: }).find(id)`
    pub fn find_in_room(conn: &Connection, room_id: i64, id: i64) -> Result<Self> {
        query_one(
            conn,
            r#"SELECT "polls".* FROM "polls" INNER JOIN "messages" ON "messages"."id" = "polls"."message_id" WHERE "messages"."room_id" = ? AND "polls"."id" = ? LIMIT 1"#,
            [room_id, id],
            Self::from_row,
        )?
        .or_not_found("Poll")
    }

    /// `Poll.normalize_labels`: stripped, blanks dropped.
    pub fn normalize_labels(labels: &[String]) -> Vec<String> {
        labels.iter().map(|label| campfire_richtext::ruby::strip(label).to_string()).filter(|label| !campfire_richtext::ruby::is_blank(label)).collect()
    }

    /// `Poll.create_for_message!(message:, labels:, multiple:, anonymous:, closes_at:)`: never on
    /// a streaming message (its question doesn't exist until it finalizes); 2 to 10 non-blank
    /// labels; then the poll and its options, in label order.
    pub fn create_for_message(tx: &mut Tx<'_>, message: &Message, poll: NewPoll) -> Result<Self> {
        if message.streaming {
            return Err(invalid("base", "A streaming message cannot carry a poll"));
        }
        let labels = Self::normalize_labels(&poll.labels);
        if !(MIN_OPTIONS..=MAX_OPTIONS).contains(&labels.len()) {
            return Err(invalid("base", &format!("Poll needs between {MIN_OPTIONS} and {MAX_OPTIONS} options")));
        }
        Self::validate(tx.conn(), tx.now(), message.id, poll.closes_at)?.into_result()?;
        let now = tx.now();
        let id: i64 = tx.conn().query_row_cached(
            r#"INSERT INTO "polls" ("anonymous", "closes_at", "created_at", "message_id", "multiple", "updated_at") VALUES (?, ?, ?, ?, ?, ?) RETURNING "id""#,
            params![poll.anonymous, poll.closes_at, now, message.id, poll.multiple, now],
            |r| r.get(0),
        )?;
        for (position, label) in labels.iter().enumerate() {
            PollOption::create(tx, id, label, position as i64)?;
        }
        Self::find(tx.conn(), id)
    }

    /// `belongs_to :message`, `validates :message_id, uniqueness: true`, and
    /// `closes_at_must_be_future` for a new poll.
    pub fn validate(conn: &Connection, now: Timestamp, message_id: i64, closes_at: Option<Timestamp>) -> Result<Errors> {
        let mut errors = Errors::default();
        if Message::find_by_id(conn, message_id)?.is_none() {
            errors.add("message", "must exist");
        }
        if sql::exists(conn, r#"SELECT 1 AS one FROM "polls" WHERE "polls"."message_id" = ? LIMIT 1"#, [message_id])? {
            errors.add("message_id", "has already been taken");
        }
        if closes_at.is_some_and(|closes_at| closes_at <= now) {
            errors.add("closes_at", "must be in the future");
        }
        Ok(errors)
    }

    pub fn single(&self) -> bool {
        !self.multiple
    }

    /// `closed?(now:)`: stamped closed, or past its closing time.
    pub fn closed(&self, now: Timestamp) -> bool {
        self.closed_at.is_some() || self.closes_at.is_some_and(|closes_at| closes_at <= now)
    }

    pub fn open(&self, now: Timestamp) -> bool {
        !self.closed(now)
    }

    /// `question`: the message's text.
    pub fn question(&self, conn: &Connection, rich_text: &dyn crate::RichText) -> Result<String> {
        Message::find(conn, self.message_id)?.plain_text_body(conn, rich_text)
    }

    pub fn options(&self, conn: &Connection) -> Result<Vec<PollOption>> {
        PollOption::for_poll(conn, self.id)
    }

    pub fn votes(&self, conn: &Connection) -> Result<Vec<PollVote>> {
        PollVote::for_poll(conn, self.id)
    }
    /// Standalone cards use the same ordered votes with optional voter names in one read.
    pub fn votes_with_names(&self, conn: &Connection) -> Result<Vec<(PollVote, Option<String>)>> {
        query_all(conn,
            &format!("SELECT poll_votes.*,{} FROM poll_votes LEFT JOIN users ON users.id=poll_votes.user_id WHERE poll_votes.poll_id=? ORDER BY poll_votes.id", User::projection("users", "voter_")),
            [self.id],|r|Ok((PollVote::from_row(r)?,r.get::<_, Option<i64>>("voter_id")?.map(|_| User::from_prefixed_row(r, "voter_").map(|user| user.display_name().to_owned())).transpose()?)))
    }

    /// The ids of the polls `close_due!` closes at `now`: unstamped, with a closing time come.
    pub fn due(conn: &Connection, now: Timestamp) -> Result<Vec<i64>> {
        query_all(
            conn,
            r#"SELECT "polls"."id" FROM "polls" WHERE "polls"."closed_at" IS NULL AND (closes_at IS NOT NULL AND closes_at <= ?) ORDER BY "polls"."id" ASC"#,
            [now],
            |r| r.get(0),
        )
    }

    /// `Poll.close_due!(now:)`, in one transaction. The periodic task closes each due poll in
    /// its own write instead ([`Poll::due`], then [`Poll::close_by_id`]), so one failure is
    /// logged and skipped, as Ruby rescues per poll.
    pub fn close_due(tx: &mut Tx<'_>, now: Timestamp) -> Result<Vec<i64>> {
        let mut closed = Vec::new();
        for id in Self::due(tx.conn(), now)? {
            if Self::close_by_id(tx, id, now)? {
                closed.push(id);
            }
        }
        Ok(closed)
    }

    /// `close!(now:)` of the poll `id`.
    pub fn close_by_id(tx: &mut Tx<'_>, id: i64, now: Timestamp) -> Result<bool> {
        let Some(mut poll) = query_one(tx.conn(), r#"SELECT * FROM "polls" WHERE "id" = ?"#, [id], Self::from_row)? else {
            return Ok(false);
        };
        poll.close(tx, now)
    }

    /// `close!(now:)`: a conditional claim (`update_all` where still unstamped), so a close
    /// racing another wins once; the winner re-broadcasts the card.
    pub fn close(&mut self, tx: &mut Tx<'_>, now: Timestamp) -> Result<bool> {
        let claimed = tx.conn().execute_cached(
            r#"UPDATE "polls" SET "closed_at" = ?, "updated_at" = ? WHERE "polls"."id" = ? AND "polls"."closed_at" IS NULL"#,
            params![now, now, self.id],
        )? == 1;
        if !claimed {
            return Ok(false);
        }
        *self = Self::find(tx.conn(), self.id)?;
        self.broadcast_card_replace(tx, None)?;
        Ok(true)
    }

    /// `cast_vote!(voter, option_ids)`: replaces the voter's ballot (empty retracts). Options must
    /// be this poll's, a single-choice poll takes one, and a closed poll none. The poll is touched
    /// so the message's fragment cache key moves with every vote, and the card re-broadcast.
    pub fn cast_vote(&mut self, tx: &mut Tx<'_>, voter_id: i64, option_ids: &[i64]) -> Result<()> {
        let mut ids: Vec<i64> = Vec::new();
        for id in option_ids {
            if !ids.contains(id) {
                ids.push(*id);
            }
        }
        let options: Vec<i64> = self.options(tx.conn())?.into_iter().map(|option| option.id).collect();
        if ids.iter().any(|id| !options.contains(id)) {
            return Err(invalid("base", "That option is not part of this poll"));
        }
        if self.single() && ids.len() > 1 {
            return Err(invalid("base", "This poll allows only one option"));
        }
        // `lock!` reloads.
        *self = Self::find(tx.conn(), self.id)?;
        if self.closed(tx.now()) {
            return Err(invalid("base", "This poll is closed"));
        }
        tx.conn().execute_cached(
            r#"DELETE FROM "poll_votes" WHERE "poll_votes"."poll_id" = ? AND "poll_votes"."user_id" = ?"#,
            [self.id, voter_id],
        )?;
        for option_id in ids {
            PollVote::create(tx, self.id, option_id, voter_id)?;
        }
        let now = tx.now();
        tx.conn().execute_cached(r#"UPDATE "polls" SET "updated_at" = ? WHERE "polls"."id" = ?"#, params![now, self.id])?;
        self.updated_at = now;
        self.broadcast_card_replace(tx, Some(voter_id))
    }

    /// `broadcast_card_replace`: the card, replaced in the message's conversation. Then the
    /// sync-only [`PollChanged`], for the single-page app's `poll.updated` and the voter's
    /// `poll.ballot`.
    fn broadcast_card_replace(&self, tx: &mut Tx<'_>, voter_id: Option<i64>) -> Result<()> {
        tx.emit_after_commit(Event::broadcast(&PollChanged { poll_id: self.id, voter_id }));
        Ok(())
    }

    /// `results_payload(viewer:)`: the card's, the vote response's and the agent API's JSON.
    /// Anonymous polls carry counts only.
    pub fn results_payload(&self, conn: &Connection, rich_text: &dyn crate::RichText, now: Timestamp, viewer_id: Option<i64>, verifier: &dyn campfire_storage::Verifier) -> Result<serde_json::Value> {
        let message = Message::find(conn, self.message_id)?;
        let votes = self.votes(conn)?;
        let voted: Vec<i64> = votes.iter().filter(|vote| Some(vote.user_id) == viewer_id).map(|vote| vote.poll_option_id).collect();
        let voter_ids: Vec<i64> = votes.iter().map(|vote| vote.user_id).collect();
        let names = voter_names(conn, &voter_ids)?;
        let options: Vec<serde_json::Value> = self
            .options(conn)?
            .into_iter()
            .map(|option| {
                let option_votes: Vec<&PollVote> = votes.iter().filter(|vote| vote.poll_option_id == option.id).collect();
                let mut entry = json!({
                    "id": option.id,
                    "label": option.label,
                    "votes": option_votes.len(),
                    "voted": voted.contains(&option.id),
                });
                if !self.anonymous {
                    let mut voters: Vec<String> =
                        option_votes.iter().filter_map(|vote| names.iter().find(|(id, _)| *id == vote.user_id).map(|(_, name)| name.clone())).collect();
                    voters.sort();
                    entry["voters"] = json!(voters);
                }
                if let Some(media) = option.media(conn, verifier)? {
                    entry["media"] = media;
                }
                Ok(entry)
            })
            .collect::<Result<_>>()?;
        Ok(json!({
            "id": self.id,
            "message_id": self.message_id,
            "room_id": message.room_id,
            "question": message.plain_text_body(conn, rich_text)?,
            "multiple": self.multiple,
            "anonymous": self.anonymous,
            "closes_at": self.closes_at.map(json_time),
            "closed_at": self.closed_at.map(json_time),
            "closed": self.closed(now),
            "total_votes": votes.len(),
            "options": options,
        }))
    }

    /// `message.poll` in `Message#destroy` (`has_one :poll, dependent: :destroy`): the options
    /// (and their votes), then the votes, then the poll.
    pub(crate) fn destroy_for_message(tx: &mut Tx<'_>, message_id: i64) -> Result<()> {
        let Some(poll) = Self::find_by_message(tx.conn(), message_id)? else { return Ok(()) };
        poll.destroy(tx)
    }

    /// `destroy`: `has_many :poll_options, dependent: :destroy` (each destroying its votes), then
    /// `has_many :poll_votes, dependent: :destroy`, then the row.
    pub fn destroy(&self, tx: &mut Tx<'_>) -> Result<()> {
        for option in self.options(tx.conn())? {
            option.destroy(tx)?;
        }
        tx.conn().execute_cached(r#"DELETE FROM "poll_votes" WHERE "poll_votes"."poll_id" = ?"#, [self.id])?;
        tx.conn().execute_cached(r#"DELETE FROM "polls" WHERE "polls"."id" = ?"#, [self.id])?;
        Ok(())
    }
}

/// `vote.user&.name` for each voter.
fn voter_names(conn: &Connection, user_ids: &[i64]) -> Result<Vec<(i64, String)>> {
    if user_ids.is_empty() {
        return Ok(Vec::new());
    }
    let sql = format!(r#"SELECT "users".* FROM "users" WHERE "users"."id" IN ({})"#, placeholders(user_ids.len()));
    query_all(conn, &sql, rusqlite::params_from_iter(user_ids), |r| {
        let user = User::from_row(r)?;
        Ok((user.id, user.display_name().to_owned()))
    })
}

/// A time as Rails' JSON encodes it: ISO 8601 in UTC with milliseconds.
pub fn json_time(time: Timestamp) -> String {
    time.jiff().strftime("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

impl PollOption {
    pub(crate) fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            poll_id: row.get("poll_id")?,
            label: row.get("label")?,
            position: row.get("position")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    /// `poll.poll_options`: `order(:position, :id)`
    pub fn for_poll(conn: &Connection, poll_id: i64) -> Result<Vec<Self>> {
        query_all(
            conn,
            r#"SELECT "poll_options".* FROM "poll_options" WHERE "poll_options"."poll_id" = ? ORDER BY "poll_options"."position" ASC, "poll_options"."id" ASC"#,
            [poll_id],
            Self::from_row,
        )
    }

    /// `poll.poll_options.create!(label:, position:)`
    pub fn create(tx: &Tx<'_>, poll_id: i64, label: &str, position: i64) -> Result<Self> {
        Self::validate(tx.conn(), poll_id, label)?.into_result()?;
        let now = tx.now();
        let id: i64 = tx.conn().query_row_cached(
            r#"INSERT INTO "poll_options" ("created_at", "label", "poll_id", "position", "updated_at") VALUES (?, ?, ?, ?, ?) RETURNING "id""#,
            params![now, label, poll_id, position, now],
            |r| r.get(0),
        )?;
        query_one(tx.conn(), r#"SELECT * FROM "poll_options" WHERE "id" = ?"#, [id], Self::from_row)?.or_not_found("PollOption")
    }

    /// A single polymorphic slot also stores emoji descriptors, without changing the schema.
    pub fn attach_media(&self, tx: &Tx<'_>, blob_id: i64) -> Result<()> {
        use crate::models::active_storage::Attachment;
        if Attachment::find_for(tx.conn(), "PollOption", self.id, "media")?.is_some() {
            return Err(invalid("media", "already exists"));
        }
        Attachment::create(tx, "PollOption", self.id, "media", blob_id)?;
        Ok(())
    }

    pub fn attach_emoji(&self, tx: &Tx<'_>, content: &str) -> Result<()> {
        use crate::models::active_storage::Blob;
        // This descriptor has no file. Purging it uses the same attachment lifecycle as images.
        let blob = Blob::create(tx, &Blob {
            id: 0, key: crate::sql::uuid(), filename: "poll-emoji".into(),
            content_type: Some("application/vnd.smartfire.poll-emoji".into()),
            metadata: Some(json!({"poll_emoji": content, "poll_media": true, "identified": true, "analyzed": true}).to_string()),
            service_name: "local".into(), byte_size: 0, checksum: None, created_at: tx.now(),
        })?;
        self.attach_media(tx, blob.id)
    }

    pub fn media(&self, conn: &Connection, verifier: &dyn campfire_storage::Verifier) -> Result<Option<serde_json::Value>> {
        let Some(blob) = campfire_storage::Blob::attached(conn, "PollOption", self.id, "media").map_err(|e| Error::Other(e.to_string()))? else {
            return Ok(None);
        };
        if blob.content_type() == "application/vnd.smartfire.poll-emoji" {
            return Ok(blob.metadata.get("poll_emoji").and_then(campfire_storage::Json::as_str).map(|content| json!({"kind": "emoji", "content": content})));
        }
        let url = campfire_storage::paths::blob_redirect_path(verifier, &blob, None);
        let still_url = (campfire_storage::branding::animated(&blob) || matches!(blob.metadata.get("poll_still"), Some(campfire_storage::Json::Bool(true)))).then(|| {
            campfire_storage::paths::representation_redirect_path(verifier, &blob, &campfire_storage::branding::Kind::Logo.still_variation())
        });
        Ok(Some(json!({"kind": "image", "url": url, "stillUrl": still_url})))
    }

    pub fn destroy(&self, tx: &mut Tx<'_>) -> Result<()> {
        if let Some(attachment) = crate::models::active_storage::Attachment::find_for(tx.conn(), "PollOption", self.id, "media")? {
            attachment.delete(tx)?;
            tx.emit_after_commit(Event::PurgeBlob { blob_id: attachment.blob_id });
        }
        tx.conn().execute_cached("DELETE FROM poll_votes WHERE poll_option_id=?", [self.id])?;
        tx.conn().execute_cached("DELETE FROM poll_options WHERE id=?", [self.id])?;
        Ok(())
    }

    /// `belongs_to :poll`, `validates :label, presence: true, length: { maximum: 200 }`
    pub fn validate(conn: &Connection, poll_id: i64, label: &str) -> Result<Errors> {
        let mut errors = Errors::default();
        if !sql::exists(conn, r#"SELECT 1 AS one FROM "polls" WHERE "id" = ? LIMIT 1"#, [poll_id])? {
            errors.add("poll", "must exist");
        }
        if label.trim().is_empty() {
            errors.add("label", "can't be blank");
        }
        if label.chars().count() > LABEL_LIMIT {
            errors.add("label", format!("is too long (maximum is {LABEL_LIMIT} characters)"));
        }
        Ok(errors)
    }
}

impl PollVote {
    pub(crate) fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            poll_id: row.get("poll_id")?,
            poll_option_id: row.get("poll_option_id")?,
            user_id: row.get("user_id")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    pub fn for_poll(conn: &Connection, poll_id: i64) -> Result<Vec<Self>> {
        query_all(
            conn,
            r#"SELECT "poll_votes".* FROM "poll_votes" WHERE "poll_votes"."poll_id" = ? ORDER BY "poll_votes"."id" ASC"#,
            [poll_id],
            Self::from_row,
        )
    }

    /// `poll.poll_votes.create!(poll_option_id:, user:)`
    pub fn create(tx: &Tx<'_>, poll_id: i64, poll_option_id: i64, user_id: i64) -> Result<Self> {
        Self::validate(tx.conn(), poll_id, poll_option_id, user_id)?.into_result()?;
        let now = tx.now();
        let id: i64 = tx.conn().query_row_cached(
            r#"INSERT INTO "poll_votes" ("created_at", "poll_id", "poll_option_id", "updated_at", "user_id") VALUES (?, ?, ?, ?, ?) RETURNING "id""#,
            params![now, poll_id, poll_option_id, now, user_id],
            |r| r.get(0),
        )?;
        query_one(tx.conn(), r#"SELECT * FROM "poll_votes" WHERE "id" = ?"#, [id], Self::from_row)?.or_not_found("PollVote")
    }

    /// The `belongs_to`s, `validates :user_id, uniqueness: { scope: :poll_option_id }` and
    /// `option_must_belong_to_poll`.
    pub fn validate(conn: &Connection, poll_id: i64, poll_option_id: i64, user_id: i64) -> Result<Errors> {
        let mut errors = Errors::default();
        let poll_exists = sql::exists(conn, r#"SELECT 1 AS one FROM "polls" WHERE "id" = ? LIMIT 1"#, [poll_id])?;
        let option_poll: Option<i64> = query_one(conn, r#"SELECT "poll_id" FROM "poll_options" WHERE "id" = ?"#, [poll_option_id], |r| r.get(0))?;
        if !poll_exists {
            errors.add("poll", "must exist");
        }
        if option_poll.is_none() {
            errors.add("poll_option", "must exist");
        }
        if User::find_by_id(conn, user_id)?.is_none() {
            errors.add("user", "must exist");
        }
        if sql::exists(
            conn,
            r#"SELECT 1 AS one FROM "poll_votes" WHERE "poll_votes"."user_id" = ? AND "poll_votes"."poll_option_id" = ? LIMIT 1"#,
            [user_id, poll_option_id],
        )? {
            errors.add("user_id", "has already been taken");
        }
        if poll_exists && option_poll.is_some_and(|option_poll| option_poll != poll_id) {
            errors.add("poll_option", "is not part of this poll");
        }
        Ok(errors)
    }
}
