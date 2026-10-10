use crate::{app::App, integrations::link_embed::Reference};
use campfire_db::{Connection, Message};

/// Claims and durable job rows share this write. Rechecking TTL/claim handles concurrent views.
pub async fn enqueue_render_fetches(app: &App, ids: Vec<i64>, twitter_ids: Vec<i64>) -> campfire_db::Result<()> {
    if ids.is_empty() && twitter_ids.is_empty() {
        return Ok(());
    }
    let result = app.db
        .write(move |tx| {
            for id in ids {
                match crate::integrations::link_embed::Embed::find(tx.conn(), id) {
                    Ok(embed) => {
                        crate::integrations::link_embed::store::request_fetch(tx, &embed)?;
                    }
                    Err(campfire_db::Error::RecordNotFound(_)) => {}
                    Err(error) => return Err(error),
                }
            }
            for id in twitter_ids {
                match crate::integrations::twitter::post::Post::find(tx.conn(), id) {
                    Ok(post) if post.fetch_pending() => { post.request_fetch(tx)?; }
                    Ok(_) | Err(campfire_db::Error::RecordNotFound(_)) => {}
                    Err(error) => return Err(error),
                }
            }
            Ok(())
        })
        .await;
    // Collection fragments can be stored before this writer rejects a fetch job.
    // Do not let a retry reuse that incomplete render and lose its pending requests.
    if result.is_err() { app.fragment_cache.clear(); }
    result
}

/// WS8b's actions-menu seam: the HTTP endpoint also independently enforces this policy.
#[allow(
    dead_code,
    reason = "WS8b consumes the actions-menu policy; endpoint authorization is already wired"
)]
pub fn can_offer_suppression(conn: &Connection, message: &Message, viewer: i64) -> campfire_db::Result<bool> {
    if message.creator_id != viewer || message.system_note || message.embeds_suppressed {
        return Ok(false);
    }
    if let Some(thread) = message.thread_id
        && conn.query_row("SELECT locked_at IS NOT NULL FROM channel_threads WHERE id=?", [thread], |r| {
            r.get::<_, bool>(0)
        })?
    {
        return Ok(false);
    }
    Ok(Reference::for_message(conn, message)?
        .iter()
        .any(|reference| reference.embed.usable() || reference.embed.linkedin()))
}
