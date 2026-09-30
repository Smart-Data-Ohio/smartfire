//! Repository subscriptions and notification claims; callbacks remain in the write transaction.
use super::{blank, client::ruby_strip, notifier::bot_user};
use campfire_db::{Connection, Errors, Result, Room, Tx, User};
use rusqlite::{OptionalExtension, params};
use serde_json::{Value, json};

pub const EVENT_KEYS: [&str; 6] = [
    "opened",
    "merged",
    "closed",
    "review_requested",
    "review_submitted",
    "checks_failed",
];
pub const DEFAULT_EVENTS: [&str; 4] = ["opened", "merged", "review_requested", "checks_failed"];
#[derive(Clone, Debug)]
pub struct RepositorySubscription {
    pub id: i64,
    pub room_id: i64,
    pub owner: String,
    pub repo: String,
    pub events: Value,
    pub reader_verified: bool,
    pub created_by_id: Option<i64>,
}
impl RepositorySubscription {
    fn from_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        let events: String = r.get("events")?;
        Ok(Self {
            id: r.get("id")?,
            room_id: r.get("room_id")?,
            owner: r.get("owner")?,
            repo: r.get("repo")?,
            events: serde_json::from_str(&events).unwrap_or(Value::Null),
            reader_verified: r.get("reader_verified")?,
            created_by_id: r.get("created_by_id")?,
        })
    }
    pub fn find(conn: &Connection, id: i64) -> Result<Self> {
        conn.query_row(
            "SELECT * FROM github_repository_subscriptions WHERE id=?",
            [id],
            Self::from_row,
        )
        .optional()?
        .ok_or(campfire_db::Error::RecordNotFound(
            "Github::RepositorySubscription",
        ))
    }
    pub fn full_name(&self) -> String {
        format!("{}/{}", self.owner, self.repo)
    }
    pub fn subscribed_to(&self, event: &str) -> bool {
        self.events
            .as_array()
            .is_some_and(|v| v.iter().any(|e| e.as_str() == Some(event)))
    }
    pub fn create(
        tx: &mut Tx<'_>,
        room_id: i64,
        owner: &str,
        repo: &str,
        events: Value,
        created_by_id: Option<i64>,
        reader_verified: bool,
    ) -> Result<Self> {
        let (owner, repo) = (
            ruby_strip(owner).to_lowercase(),
            ruby_strip(repo).to_lowercase(),
        );
        let events = if value_blank(&events) {
            json!(DEFAULT_EVENTS)
        } else {
            events
        };
        validate(tx.conn(), None, room_id, &owner, &repo, &events, true)?.into_result()?;
        tx.savepoint(|tx| {
            let id=tx.conn().query_row("INSERT INTO github_repository_subscriptions (room_id,owner,repo,events,created_by_id,reader_verified,created_at,updated_at) VALUES (?,?,?,?,?,?,?,?) RETURNING id",params![room_id,owner,repo,events.to_string(),created_by_id,reader_verified,tx.now(),tx.now()],|r|r.get(0))?;
            let bot=bot_user(tx)?;
            let room=Room::find(tx.conn(),room_id)?;
            User::find(tx.conn(),bot)?;
            // Membership's create callback only changes direct-room keys (excluded above).
            // Stage membership validation defaults a new member to listener.
            tx.conn().execute("INSERT INTO memberships (room_id,user_id,involvement,stage_role,created_at,updated_at) VALUES (?,?,?,?,?,?) ON CONFLICT(user_id,room_id) DO NOTHING",params![room_id,bot,room.default_involvement(),room.stage().then_some("listener"),tx.now(),tx.now()])?;
            Self::find(tx.conn(),id)
        })
    }
    pub fn update(
        &mut self,
        tx: &Tx<'_>,
        owner: &str,
        repo: &str,
        events: Value,
        reader_verified: bool,
    ) -> Result<()> {
        let (owner, repo) = (
            ruby_strip(owner).to_lowercase(),
            ruby_strip(repo).to_lowercase(),
        );
        validate(
            tx.conn(),
            Some(self.id),
            self.room_id,
            &owner,
            &repo,
            &events,
            false,
        )?
        .into_result()?;
        tx.conn().execute("UPDATE github_repository_subscriptions SET owner=?,repo=?,events=?,reader_verified=?,updated_at=? WHERE id=?",params![owner,repo,events.to_string(),reader_verified,tx.now(),self.id])?;
        *self = Self::find(tx.conn(), self.id)?;
        Ok(())
    }
    pub fn destroy(self, tx: &Tx<'_>) -> Result<()> {
        tx.conn().execute(
            "DELETE FROM github_notifications WHERE subscription_id=?",
            [self.id],
        )?;
        tx.conn().execute(
            "DELETE FROM github_repository_subscriptions WHERE id=?",
            [self.id],
        )?;
        if !tx.conn().query_row(
            "SELECT EXISTS(SELECT 1 FROM github_repository_subscriptions WHERE room_id=?)",
            [self.room_id],
            |r| r.get::<_, bool>(0),
        )? {
            // Rails deliberately delete_all's membership: no disconnect/destroy callbacks.
            tx.conn().execute("DELETE FROM memberships WHERE room_id=? AND user_id=(SELECT id FROM users WHERE role=2 AND status=0 AND name='GitHub' ORDER BY id LIMIT 1)",[self.room_id])?;
        }
        Ok(())
    }
}
fn value_blank(value: &Value) -> bool {
    match value {
        Value::Null | Value::Bool(false) => true,
        Value::String(s) => blank(s),
        Value::Array(a) => a.is_empty(),
        Value::Object(o) => o.is_empty(),
        _ => false,
    }
}
pub fn validate(
    conn: &Connection,
    id: Option<i64>,
    room_id: i64,
    owner: &str,
    repo: &str,
    events: &Value,
    create: bool,
) -> Result<Errors> {
    let mut errors = Errors::default();
    let room = Room::find_by_id(conn, room_id)?;
    if room.is_none() {
        errors.add("room", "must exist");
    }
    for (key, name) in [("owner", owner), ("repo", repo)] {
        if blank(name) {
            errors.add(key, "can't be blank");
        }
        if name.is_empty()
            || [".", ".."].contains(&name)
            || !name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "_.-".contains(c))
        {
            errors.add(key, "is invalid");
        }
    }
    if conn.query_row("SELECT EXISTS(SELECT 1 FROM github_repository_subscriptions WHERE room_id=? AND LOWER(owner)=LOWER(?) AND repo=? AND id!=COALESCE(?,0))",params![room_id,owner,repo,id],|r|r.get::<_,bool>(0))? {errors.add("owner","has already been taken");}
    if !events.as_array().is_some_and(|a| {
        a.iter()
            .all(|e| e.as_str().is_some_and(|e| EVENT_KEYS.contains(&e)))
    }) {
        errors.add("events","must be a subset of opened, merged, closed, review_requested, review_submitted, and checks_failed");
    }
    if !create && value_blank(events) {
        errors.add("events", "must include at least one event");
    }
    if room.is_some_and(|r| r.direct()) {
        errors.add("room", "must not be a direct room");
    }
    Ok(errors)
}

/// `Github::Notification.claim!`: validation failures and duplicate losers return nil.
pub fn claim_notification(tx: &Tx<'_>, subscription: i64, key: &str) -> Result<Option<i64>> {
    if blank(key)
        || !tx.conn().query_row(
            "SELECT EXISTS(SELECT 1 FROM github_repository_subscriptions WHERE id=?)",
            [subscription],
            |r| r.get::<_, bool>(0),
        )?
    {
        return Ok(None);
    }
    Ok(tx.conn().query_row("INSERT INTO github_notifications (subscription_id,dedupe_key,created_at,updated_at) VALUES (?,?,?,?) ON CONFLICT(subscription_id,dedupe_key) DO NOTHING RETURNING id",params![subscription,key,tx.now(),tx.now()],|r|r.get(0)).optional()?)
}
