//! Rails callbacks use find_each; every dependent IN preload receives one bounded batch.
use campfire_db::{Connection, Message, Result};

pub(crate) const SIZE: usize = 1_000;
#[derive(Clone, Copy)]
pub(crate) enum Reference {
    LinkEmbed(i64),
    GithubPullRequest(i64),
    FizzyCard(i64),
    TwitterPost(i64),
    CalendarEvent(i64),
}
pub(crate) fn next(
    conn: &Connection,
    reference: Reference,
    after: Option<i64>,
) -> Result<Vec<Message>> {
    let (table, column, id) = match reference {
        Reference::LinkEmbed(id) => ("link_embed_references", "link_embed_id", id),
        Reference::GithubPullRequest(id) => (
            "github_pull_request_references",
            "github_pull_request_id",
            id,
        ),
        Reference::FizzyCard(id) => ("fizzy_card_references", "fizzy_card_id", id),
        Reference::TwitterPost(id) => ("twitter_post_references", "twitter_post_id", id),
        Reference::CalendarEvent(id) => ("event_references", "event_id", id),
    };
    let ids = conn.prepare(&format!(
        "SELECT id FROM messages WHERE id IN (SELECT message_id FROM {table} WHERE {column}=?1) AND (?2 IS NULL OR id>?2) ORDER BY id LIMIT ?3"
    ))?.query_map(rusqlite::params![id,after,SIZE],|r|r.get::<_,i64>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
    let mut messages = Message::for_ids(conn, &ids)?;
    messages.sort_by_key(|m| m.id);
    Ok(messages)
}
