//! `app/models/twitter/post.rb`. Shared string identities and persisted fetch results.
use super::fetcher::{Card, FetchJob};
use campfire_db::{Broadcast, Connection, Errors, Event, Result, Timestamp, Tx};
use jiff::SignedDuration;
use rusqlite::{OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
pub const FETCH_WINDOW: SignedDuration = SignedDuration::from_mins(10);
#[derive(Debug, Clone)]
pub struct Post {
    pub id: i64,
    pub post_id: String,
    pub url: Option<String>,
    pub fetched_at: Option<Timestamp>,
    pub fetch_error: Option<String>,
    pub author_name: Option<String>,
    pub author_handle: Option<String>,
    pub author_avatar_url: Option<String>,
    pub text: Option<String>,
    pub posted_at: Option<Timestamp>,
    pub replies: Option<i64>,
    pub reposts: Option<i64>,
    pub likes: Option<i64>,
    pub media: Vec<Value>,
    pub quote: Option<Value>,
}
impl Post {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            post_id: row.get("post_id")?,
            url: row.get("url")?,
            fetched_at: row.get("fetched_at")?,
            fetch_error: row.get("fetch_error")?,
            author_name: row.get("author_name")?,
            author_handle: row.get("author_handle")?,
            author_avatar_url: row.get("author_avatar_url")?,
            text: row.get("text")?,
            posted_at: row.get("posted_at")?,
            replies: row.get("replies")?,
            reposts: row.get("reposts")?,
            likes: row.get("likes")?,
            media: json_column(row, "media")?
                .and_then(|v| v.as_array().cloned())
                .unwrap_or_default(),
            quote: json_column(row, "quote")?,
        })
    }
    pub fn find(conn: &Connection, id: i64) -> Result<Self> {
        conn.query_row(
            "SELECT * FROM twitter_posts WHERE id=?",
            [id],
            Self::from_row,
        )
        .optional()?
        .ok_or(campfire_db::Error::RecordNotFound("Twitter::Post"))
    }
    pub fn for_reference(tx: &Tx<'_>, post_id: &str, url: Option<&str>) -> Result<Self> {
        let mut errors = Errors::default();
        if campfire_richtext::ruby::is_blank(post_id) {
            errors.add("post_id", "can't be blank");
        }
        errors.into_result()?;
        tx.conn().execute("INSERT INTO twitter_posts (post_id,url,created_at,updated_at) VALUES (?,?,?3,?3) ON CONFLICT(post_id) DO NOTHING",params![post_id,url,tx.now()])?;
        Ok(tx.conn().query_row(
            "SELECT * FROM twitter_posts WHERE post_id=?",
            [post_id],
            Self::from_row,
        )?)
    }
    pub fn for_message(conn: &Connection, message_id: i64) -> Result<Vec<Self>> {
        Ok(Self::for_messages(conn, &[message_id])?
            .remove(&message_id)
            .unwrap_or_default())
    }
    /// One association query per page (chunked only for SQLite's parameter limit).
    pub fn for_messages(conn: &Connection, message_ids: &[i64]) -> Result<HashMap<i64, Vec<Self>>> {
        let mut result: HashMap<i64, Vec<Self>> =
            message_ids.iter().map(|id| (*id, Vec::new())).collect();
        for ids in message_ids.chunks(900) {
            let placeholders = vec!["?"; ids.len()].join(",");
            let mut query = conn.prepare(&format!("SELECT p.*, r.message_id AS reference_message_id FROM twitter_posts p JOIN twitter_post_references r ON r.twitter_post_id=p.id WHERE r.message_id IN ({placeholders}) ORDER BY r.id"))?;
            for row in query.query_map(rusqlite::params_from_iter(ids), |row| {
                Ok((
                    row.get::<_, i64>("reference_message_id")?,
                    Self::from_row(row)?,
                ))
            })? {
                let (message_id, post) = row?;
                result.entry(message_id).or_default().push(post);
            }
        }
        Ok(result)
    }
    pub fn order_cards(posts: &mut [Self]) {
        // Decimal strings may exceed i64; Rails sorts by arbitrary-precision post_id.to_i.
        posts.sort_by(|a, b| {
            let a = a.post_id.trim_start_matches('0');
            let b = b.post_id.trim_start_matches('0');
            a.len().cmp(&b.len()).then_with(|| a.cmp(b))
        });
    }
    pub fn fetch_pending(&self) -> bool {
        self.fetched_at.is_none() && self.fetch_error.is_none()
    }
    pub fn display_handle(&self) -> Option<String> {
        self.author_handle
            .as_ref()
            .filter(|s| !campfire_richtext::ruby::is_blank(s))
            .cloned()
            .or_else(|| {
                super::urls::extract(self.url.as_deref().unwrap_or(""))
                    .into_iter()
                    .next()
                    .and_then(|r| r.handle)
            })
    }
    pub fn display_name(&self) -> String {
        self.author_name
            .as_ref()
            .filter(|s| !campfire_richtext::ruby::is_blank(s))
            .cloned()
            .unwrap_or_else(|| {
                self.display_handle()
                    .map(|h| format!("@{h}"))
                    .unwrap_or_else(|| "Post on X".into())
            })
    }
    pub fn profile_url(&self) -> Option<String> {
        self.display_handle()
            .filter(|s| !campfire_richtext::ruby::is_blank(s))
            .map(|h| format!("https://x.com/{h}"))
    }
    pub fn view_url(&self) -> String {
        self.url
            .as_deref()
            .filter(|s| !campfire_richtext::ruby::is_blank(s))
            .map(str::to_owned)
            .unwrap_or_else(|| format!("https://x.com/i/status/{}", self.post_id))
    }
    pub fn needs_fetch(&self, now: Timestamp) -> bool {
        self.fetched_at.is_none()
            || (self
                .fetch_error
                .as_deref()
                .is_some_and(|e| !campfire_richtext::ruby::is_blank(e))
                && self.fetched_at.is_some_and(|at| at < now.ago(FETCH_WINDOW)))
    }
    pub fn claim_fetch(&self, tx: &Tx<'_>) -> Result<bool> {
        Ok(tx.conn().execute("UPDATE twitter_posts SET fetch_requested_at=? WHERE id=? AND (fetch_requested_at IS NULL OR fetch_requested_at<?)", params![tx.now(),self.id,tx.now().ago(FETCH_WINDOW)])?==1)
    }
    pub fn request_fetch(&self, tx: &mut Tx<'_>) -> Result<bool> {
        if !self.claim_fetch(tx)? {
            return Ok(false);
        }
        tx.emit_after_commit(Event::job(&FetchJob { post_id: self.id }));
        Ok(true)
    }
    /// Broadcast partials render all siblings too. Claim their lost jobs on this same writer,
    /// before commit, so readers and callback delivery never acquire a second writer.
    fn request_pending_siblings(&self, tx: &mut Tx<'_>) -> Result<()> {
        use crate::integrations::message_batches::{self, Reference};
        let mut seen = std::collections::HashSet::new();
        let mut after = None;
        loop {
            let messages =
                message_batches::next(tx.conn(), Reference::TwitterPost(self.id), after)?;
            let ids: Vec<_> = messages.iter().map(|m| m.id).collect();
            let mut posts = Self::for_messages(tx.conn(), &ids)?;
            for message in &messages {
                for post in posts.remove(&message.id).unwrap_or_default() {
                    if post.fetch_pending() && seen.insert(post.id) {
                        post.request_fetch(tx)?;
                    }
                }
            }
            if messages.len() < message_batches::SIZE {
                break;
            }
            after = messages.last().map(|m| m.id);
        }
        Ok(())
    }
    pub fn save_card(&self, tx: &mut Tx<'_>, card: &Card) -> Result<()> {
        tx.conn().execute("UPDATE twitter_posts SET url=?,author_handle=?,author_name=?,author_avatar_url=?,text=?,posted_at=?,replies=?,reposts=?,likes=?,media=?,quote=?,fetch_error=NULL,fetched_at=?12,updated_at=?12 WHERE id=?13", params![card.url,card.author_handle,card.author_name,card.author_avatar_url,card.text,card.posted_at,card.replies,card.reposts,card.likes,card.media.to_string(),card.quote.as_ref().map(Value::to_string),tx.now(),self.id])?;
        self.request_pending_siblings(tx)?;
        tx.emit_after_commit(Event::broadcast(&CardUpdate { post_id: self.id }));
        Ok(())
    }
    /// Rails update! preserves the old payload on failure.
    pub fn save_error(&self, tx: &mut Tx<'_>, error: &str) -> Result<()> {
        tx.conn().execute(
            "UPDATE twitter_posts SET fetch_error=?,fetched_at=?2,updated_at=?2 WHERE id=?3",
            params![error, tx.now(), self.id],
        )?;
        self.request_pending_siblings(tx)?;
        tx.emit_after_commit(Event::broadcast(&CardUpdate { post_id: self.id }));
        Ok(())
    }
}
// ActiveRecord JSON deserializes invalid legacy scalars to nil.
fn json_column(row: &Row<'_>, name: &str) -> rusqlite::Result<Option<Value>> {
    Ok(match row.get_ref(name)? {
        rusqlite::types::ValueRef::Text(bytes) => serde_json::from_slice(bytes).ok(),
        _ => None,
    })
}
#[derive(Debug, Serialize, Deserialize)]
pub struct CardUpdate {
    pub post_id: i64,
}
impl Broadcast for CardUpdate {
    const KIND: &'static str = "Twitter::Post#broadcast_card_updates";
}

#[cfg(test)]
mod tests;
