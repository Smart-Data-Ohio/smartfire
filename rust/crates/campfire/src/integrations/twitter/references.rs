//! `Twitter::PostReferenceSync` and `PostReferenceBackfill`: writer-only, no HTTP/HTML rendering.
use super::{post::Post, urls};
use campfire_db::{Message, Result, Tx};
use rusqlite::params;
pub fn sync_message(tx: &mut Tx<'_>, message: &Message, enqueue: bool) -> Result<()> {
    let html = message.body_html(tx.conn())?.unwrap_or_default();
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
/// Batches the exact Rails matching walk. Caller supplies the quiet/enqueue choice in its transaction.
#[allow(
    dead_code,
    reason = "Quiet deploy/import backfill caller belongs to WS16"
)]
pub fn backfill(tx: &mut Tx<'_>, enqueue: bool) -> Result<usize> {
    let mut synced = 0;
    let mut last = 0;
    loop {
        let ids = {
            let mut query = tx
                .conn()
                .prepare("SELECT id FROM messages WHERE id>? ORDER BY id LIMIT 1000")?;
            query
                .query_map([last], |r| r.get::<_, i64>(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?
        };
        if ids.is_empty() {
            break;
        }
        for id in ids {
            last = id;
            let message = Message::find(tx.conn(), id)?;
            if message
                .markdown_source
                .as_deref()
                .is_some_and(urls::is_post_url)
                || message
                    .body_html(tx.conn())?
                    .as_deref()
                    .is_some_and(urls::is_post_url)
            {
                sync_message(tx, &message, enqueue)?;
                synced += 1;
            }
        }
    }
    Ok(synced)
}
#[cfg(test)]
mod tests;
