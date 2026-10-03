//! `app/models/link_embed.rb`, `link_embed_reference.rb`, and `link_embed/reference_sync.rb`.
use campfire_db::{Connection, Errors, Event, Message, Result, Timestamp, Tx};
use jiff::SignedDuration;
use rusqlite::{OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};

use super::{FetchJob, metadata_parser::Metadata};

pub const SUCCESS_TTL: SignedDuration = SignedDuration::from_hours(24);
pub const NEGATIVE_TTL: SignedDuration = SignedDuration::from_hours(1);
pub const FETCH_WINDOW: SignedDuration = SignedDuration::from_mins(10);

#[derive(Debug, Clone, PartialEq)]
pub struct Embed {
    pub id: i64,
    pub normalized_url: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub site_name: Option<String>,
    pub image_url: Option<String>,
    pub fetch_error: Option<String>,
    pub expires_at: Option<Timestamp>,
    pub fetched_at: Option<Timestamp>,
    pub fetch_requested_at: Option<Timestamp>,
}

impl Embed {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            normalized_url: row.get("normalized_url")?,
            title: row.get("title")?,
            description: row.get("description")?,
            site_name: row.get("site_name")?,
            image_url: row.get("image_url")?,
            fetch_error: row.get("fetch_error")?,
            expires_at: row.get("expires_at")?,
            fetched_at: row.get("fetched_at")?,
            fetch_requested_at: row.get("fetch_requested_at")?,
        })
    }

    pub fn find(conn: &Connection, id: i64) -> Result<Self> {
        conn.query_row("SELECT * FROM link_embeds WHERE id = ?", [id], Self::from_row)
            .optional()?
            .ok_or(campfire_db::Error::RecordNotFound("LinkEmbed"))
    }

    pub fn for_reference(tx: &Tx<'_>, key: &str) -> Result<Self> {
        let mut errors = Errors::default();
        if key.trim().is_empty() {
            errors.add("normalized_url", "can't be blank");
        }
        errors.into_result()?;
        if let Some(embed) = tx
            .conn()
            .query_row("SELECT * FROM link_embeds WHERE normalized_url = ?", [key], Self::from_row)
            .optional()?
        {
            return Ok(embed);
        }
        tx.conn().execute(
            "INSERT INTO link_embeds (normalized_url, created_at, updated_at) VALUES (?1, ?2, ?2) ON CONFLICT(normalized_url) DO NOTHING",
            params![key, tx.now()],
        )?;
        Ok(tx
            .conn()
            .query_row("SELECT * FROM link_embeds WHERE normalized_url = ?", [key], Self::from_row)?)
    }

    pub fn needs_fetch(&self, now: Timestamp) -> bool {
        self.expires_at.is_none_or(|expires| expires <= now)
    }
    pub fn usable(&self) -> bool {
        self.title.as_deref().is_some_and(present) || self.description.as_deref().is_some_and(present)
    }
    pub fn linkedin(&self) -> bool {
        crate::integrations::linkedin::is_post_url(&self.normalized_url)
    }

    /// update_all bypasses callbacks and updated_at; the cutoff is strictly less than.
    pub fn claim_fetch(&self, tx: &Tx<'_>) -> Result<bool> {
        Ok(tx.conn().execute(
            "UPDATE link_embeds SET fetch_requested_at = ?2 WHERE id = ?1 AND (fetch_requested_at IS NULL OR fetch_requested_at < ?3)",
            params![self.id, tx.now(), tx.now().ago(FETCH_WINDOW)],
        )? == 1)
    }

    pub fn save_metadata(&self, tx: &mut Tx<'_>, metadata: &Metadata) -> Result<()> {
        let mut errors = Errors::default();
        for (name, value, limit) in [
            ("title", &metadata.title, 300),
            ("description", &metadata.description, 1000),
            ("site_name", &metadata.site_name, 100),
        ] {
            if value.as_ref().is_some_and(|s| s.chars().count() > limit) {
                errors.add(name, format!("is too long (maximum is {limit} characters)"));
            }
        }
        errors.into_result()?;
        tx.conn().execute("UPDATE link_embeds SET title=?2, description=?3, site_name=?4, image_url=?5, fetched_at=?6, expires_at=?7, fetch_error=NULL, updated_at=?6 WHERE id=?1", params![self.id, metadata.title, metadata.description, metadata.site_name, metadata.image_url, tx.now(), tx.now().since(SUCCESS_TTL)])?;
        self.broadcast_after_commit(tx)?;
        Ok(())
    }

    /// Negative results retain any previous usable metadata, exactly as update! in Rails.
    pub fn save_negative(&self, tx: &mut Tx<'_>, error: &str) -> Result<()> {
        tx.conn().execute(
            "UPDATE link_embeds SET fetched_at=?2, expires_at=?3, fetch_error=?4, updated_at=?2 WHERE id=?1",
            params![self.id, tx.now(), tx.now().since(NEGATIVE_TTL), error],
        )?;
        self.broadcast_after_commit(tx)?;
        Ok(())
    }

    fn broadcast_after_commit(&self, tx: &mut Tx<'_>) -> Result<()> {
        // The callback renders this card container, whose helper also requests stale siblings.
        // Make those requests in the triggering transaction, before the synchronous broadcast.
        use crate::integrations::message_batches::{self, Reference as Source};
        let mut requested = std::collections::HashSet::new();
        let mut after = None;
        loop {
            let messages = message_batches::next(tx.conn(), Source::LinkEmbed(self.id), after)?;
            let visible: Vec<_> = messages
                .iter()
                .filter(|m| !m.embeds_suppressed)
                .map(|m| m.id)
                .collect();
            let mut references = Reference::for_messages(tx.conn(), &visible)?;
            for message in &messages {
                for reference in references.remove(&message.id).unwrap_or_default() {
                    if reference.embed.linkedin() == self.linkedin()
                        && requested.insert(reference.embed.id)
                    {
                        request_fetch(tx, &reference.embed)?;
                    }
                }
            }
            if messages.len() < message_batches::SIZE {
                break;
            }
            after = messages.last().map(|m| m.id);
        }
        tx.emit_after_commit(Event::broadcast(&CardUpdate { embed_id: self.id }));
        Ok(())
    }
}

fn present(text: &str) -> bool {
    !text.chars().all(char::is_whitespace)
}

#[derive(Debug, Clone)]
pub struct Reference {
    pub id: i64,
    #[allow(dead_code, reason = "Persisted reference attributes exposed at the WS8a model seam")]
    pub message_id: i64,
    pub url: Option<String>,
    #[allow(dead_code, reason = "SQL orders by this persisted attribute; consumers may inspect the reference order")]
    pub position: i64,
    pub embed: Embed,
}

impl Reference {
    pub fn display_url(&self) -> &str {
        self.url.as_deref().filter(|s| present(s)).unwrap_or(&self.embed.normalized_url)
    }
    pub fn for_message(conn: &Connection, message: &Message) -> Result<Vec<Self>> {
        let mut query = conn.prepare("SELECT r.id AS reference_id, r.message_id, r.url AS reference_url, r.position, e.* FROM link_embed_references r JOIN link_embeds e ON e.id = r.link_embed_id WHERE r.message_id=? ORDER BY r.position, r.id")?;
        Ok(query
            .query_map([message.id], |row| {
                Ok(Self {
                    id: row.get("reference_id")?,
                    message_id: row.get("message_id")?,
                    url: row.get("reference_url")?,
                    position: row.get("position")?,
                    embed: Embed::from_row(row)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?)
    }
    /// Read-only page preload seam; reference order and display URL remain owner facts.
    pub fn for_messages(conn: &Connection, ids: &[i64]) -> Result<std::collections::HashMap<i64, Vec<Self>>> {
        let mut result: std::collections::HashMap<i64, Vec<Self>> = ids.iter().map(|id| (*id, Vec::new())).collect();
        if ids.is_empty() { return Ok(result); }
        let mut query = conn.prepare("SELECT r.id AS reference_id, r.message_id, r.url AS reference_url, r.position, e.* FROM link_embed_references r JOIN link_embeds e ON e.id = r.link_embed_id WHERE r.message_id IN (SELECT value FROM json_each(?)) ORDER BY r.message_id,r.position,r.id")?;
        for reference in query.query_map([serde_json::json!(ids).to_string()], |row| Ok(Self {
            id: row.get("reference_id")?, message_id: row.get("message_id")?,
            url: row.get("reference_url")?, position: row.get("position")?, embed: Embed::from_row(row)?,
        }))? {
            let reference = reference?; result.entry(reference.message_id).or_default().push(reference);
        }
        Ok(result)
    }

}

#[derive(Debug, Serialize, Deserialize)]
pub struct CardUpdate {
    pub embed_id: i64,
}
impl campfire_db::Broadcast for CardUpdate {
    const KIND: &'static str = "LinkEmbed#broadcast_card_updates";
}

pub fn request_fetch(tx: &mut Tx<'_>, embed: &Embed) -> Result<bool> {
    if embed.needs_fetch(tx.now()) && embed.claim_fetch(tx)? {
        // EventSink::persist inserts the durable row now; only its wakeup waits for commit.
        tx.emit_after_commit(Event::job(&FetchJob { embed_id: embed.id }));
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Small WS8a seam: call in the creation/edit/finalize transaction after body/source writes.
/// The caller owns streaming/import gates. Legacy messages clear references.
pub fn sync_message(tx: &mut Tx<'_>, message: &Message, enqueue: bool) -> Result<()> {
    if message.markdown_source.is_none() && !message.forwarded_markdown {
        tx.conn()
            .execute("DELETE FROM link_embed_references WHERE message_id=?", [message.id])?;
        return Ok(());
    }
    let html = message.body_html(tx.conn())?.unwrap_or_default();
    let selected = super::reference_urls(
        &html,
        message.markdown_source.as_deref().unwrap_or(""),
        message.forward_note.as_deref().unwrap_or(""),
    )
    .map_err(|error| campfire_db::Error::Other(error.to_string()))?;
    let mut retained = Vec::new();
    for (position, reference) in selected.into_iter().enumerate() {
        let embed = Embed::for_reference(tx, &reference.normalized_url)?;
        retained.push(embed.id);
        // Both belongs_to records exist, and the composite unique index enforces uniqueness.
        let existing: Option<(i64, i64, Option<String>)> = tx
            .conn()
            .query_row(
                "SELECT id,position,url FROM link_embed_references WHERE message_id=?1 AND link_embed_id=?2",
                params![message.id, embed.id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;
        if let Some((id, old_position, old_url)) = existing {
            if old_position != position as i64 || old_url.as_deref() != Some(&reference.url) {
                tx.conn().execute(
                    "UPDATE link_embed_references SET position=?2,url=?3,updated_at=?4 WHERE id=?1",
                    params![id, position as i64, reference.url, tx.now()],
                )?;
            }
        } else {
            tx.conn().execute("INSERT INTO link_embed_references (message_id, link_embed_id, position, url, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?5)", params![message.id, embed.id, position as i64, reference.url, tx.now()])?;
        }
        if enqueue && !message.embeds_suppressed {
            request_fetch(tx, &embed)?;
        }
    }
    for reference in Reference::for_message(tx.conn(), message)? {
        if !retained.contains(&reference.embed.id) {
            tx.conn().execute("DELETE FROM link_embed_references WHERE id=?", [reference.id])?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
