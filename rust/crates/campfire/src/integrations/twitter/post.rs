//! `app/models/twitter/post.rb`. Shared string identities and persisted fetch results.
use super::fetcher::{Card, FetchJob};
use campfire_db::{Broadcast, Connection, Errors, Event, Result, Timestamp, Tx};
use jiff::SignedDuration;
use rusqlite::{OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;
pub const FETCH_WINDOW: SignedDuration = SignedDuration::from_mins(10);
#[derive(Debug, Clone)]
pub struct Post {
    pub id: i64,
    pub post_id: String,
    pub url: Option<String>,
    pub fetched_at: Option<Timestamp>,
    pub fetch_error: Option<String>,
}
impl Post {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            post_id: row.get("post_id")?,
            url: row.get("url")?,
            fetched_at: row.get("fetched_at")?,
            fetch_error: row.get("fetch_error")?,
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
    pub fn save_card(&self, tx: &mut Tx<'_>, card: &Card) -> Result<()> {
        tx.conn().execute("UPDATE twitter_posts SET url=?,author_handle=?,author_name=?,author_avatar_url=?,text=?,posted_at=?,replies=?,reposts=?,likes=?,media=?,quote=?,fetch_error=NULL,fetched_at=?12,updated_at=?12 WHERE id=?13", params![card.url,card.author_handle,card.author_name,card.author_avatar_url,card.text,card.posted_at,card.replies,card.reposts,card.likes,card.media.to_string(),card.quote.as_ref().map(Value::to_string),tx.now(),self.id])?;
        tx.emit_after_commit(Event::broadcast(&CardUpdate { post_id: self.id }));
        Ok(())
    }
    /// Rails update! preserves the old payload on failure.
    pub fn save_error(&self, tx: &mut Tx<'_>, error: &str) -> Result<()> {
        tx.conn().execute(
            "UPDATE twitter_posts SET fetch_error=?,fetched_at=?2,updated_at=?2 WHERE id=?3",
            params![error, tx.now(), self.id],
        )?;
        tx.emit_after_commit(Event::broadcast(&CardUpdate { post_id: self.id }));
        Ok(())
    }
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
