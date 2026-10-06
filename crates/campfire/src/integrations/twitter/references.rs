//! `Twitter::PostReferenceSync` and `PostReferenceBackfill`: no HTTP or network fetches.
use super::{post::Post, urls};
use campfire_db::{Connection, Env, Message, Result, Tx, run_write};
use rusqlite::params;
pub fn sync_message(tx: &mut Tx<'_>, message: &Message, enqueue: bool) -> Result<()> {
    let html = message.body_html(tx.conn())?.unwrap_or_default();
    // Rails reads Content#to_html, not the raw database column or sanitized rendered HTML.
    let html = tx
        .rich_text()
        .try_canonicalize_html(tx.conn(), &html)
        .map_err(campfire_db::Error::Other)?;
    let body = urls::non_code_text(&html).map_err(|e| campfire_db::Error::Other(e.to_string()))?;
    let refs = urls::extract(&format!(
        "{}\n{}",
        body,
        message.forward_note.as_deref().unwrap_or("")
    ));
    let mut posts = Vec::new();
    for reference in refs {
        let url = format!(
            "https://x.com/{}/status/{}",
            reference.handle.as_deref().unwrap_or("i"),
            reference.post_id
        );
        posts.push(Post::for_reference(tx, &reference.post_id, Some(&url))?);
    }
    tx.conn().execute(
        "DELETE FROM twitter_post_references WHERE message_id=? AND twitter_post_id NOT IN (SELECT value FROM json_each(?))",
        params![message.id, serde_json::json!(posts.iter().map(|post| post.id).collect::<Vec<_>>()).to_string()],
    )?;
    for post in posts {
        let inserted=tx.conn().execute("INSERT INTO twitter_post_references (message_id,twitter_post_id,created_at,updated_at) VALUES (?1,?2,?3,?3) ON CONFLICT(message_id,twitter_post_id) DO NOTHING",params![message.id,post.id,tx.now()])?==1;
        if enqueue && inserted && post.needs_fetch(tx.now()) {
            post.request_fetch(tx)?;
        }
    }
    Ok(())
}
struct BackfillMessage {
    id: i64,
    markdown: Option<String>,
    html: Option<String>,
}

impl BackfillMessage {
    fn selected(&self, conn: &Connection, rich_text: &dyn campfire_db::RichText) -> Result<bool> {
        // PostReferenceBackfill.call short-circuits on Markdown; otherwise it tests
        // message.body.to_s, with Action Text's attachment rendering and sanitization.
        Ok(self.markdown.as_deref().is_some_and(urls::is_post_url)
            || match &self.html {
                Some(html) => urls::is_post_url(
                    &rich_text
                        .try_rendered_html(conn, html)
                        .map_err(campfire_db::Error::Other)?,
                ),
                None => false,
            })
    }
}

/// Rails find_each starts at the first primary key (including zero and negative IDs).
/// Preload selection fields per batch, as Message.includes(:rich_text_body) does.
fn backfill_batch(conn: &Connection, last: Option<i64>) -> Result<Vec<BackfillMessage>> {
    let sql = if last.is_some() {
        "SELECT m.id,m.markdown_source,r.body FROM messages m LEFT JOIN action_text_rich_texts r ON r.record_type='Message' AND r.record_id=m.id AND r.name='body' WHERE m.id>?1 ORDER BY m.id LIMIT 1000"
    } else {
        "SELECT m.id,m.markdown_source,r.body FROM messages m LEFT JOIN action_text_rich_texts r ON r.record_type='Message' AND r.record_id=m.id AND r.name='body' WHERE ?1 IS NULL ORDER BY m.id LIMIT 1000"
    };
    let mut query = conn.prepare(sql)?;
    Ok(query
        .query_map([last], |row| {
            Ok(BackfillMessage {
                id: row.get(0)?,
                markdown: row.get(1)?,
                html: row.get(2)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?)
}

/// Operator walk: bounded batches and independently committed message syncs. The job sink
/// persists each fetch with its reference write; no runner or network client is started.
pub fn backfill_database(conn: &Connection, env: &Env) -> Result<usize> {
    let mut synced = 0;
    let mut last = None;
    loop {
        let batch = backfill_batch(conn, last)?;
        let Some(next) = batch.last().map(|message| message.id) else {
            break;
        };
        for row in batch {
            // Render and sync in primary-key order, preserving earlier progress if a
            // later render fails, and observing references committed by previous rows.
            if !row.selected(conn, &*env.rich_text)? {
                continue;
            }
            let id = row.id;
            run_write(conn, env, |tx| {
                let message = Message::find(tx.conn(), id)?;
                sync_message(tx, &message, true)
            })?;
            synced += 1;
        }
        last = Some(next);
    }
    Ok(synced)
}

/// Quiet/import callers may supply their own transaction and enqueue choice.
#[allow(
    dead_code,
    reason = "Quiet deploy/import backfill caller belongs to WS16"
)]
pub fn backfill(tx: &mut Tx<'_>, enqueue: bool) -> Result<usize> {
    let mut synced = 0;
    let mut last = None;
    loop {
        let batch = backfill_batch(tx.conn(), last)?;
        let Some(next) = batch.last().map(|message| message.id) else {
            break;
        };
        for row in batch {
            if !row.selected(tx.conn(), tx.rich_text())? {
                continue;
            }
            let message = Message::find(tx.conn(), row.id)?;
            sync_message(tx, &message, enqueue)?;
            synced += 1;
        }
        last = Some(next);
    }
    Ok(synced)
}
#[cfg(test)]
mod tests;
