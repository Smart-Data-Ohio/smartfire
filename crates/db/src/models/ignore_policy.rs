//! Personal visibility and one-way contact prevention. People remain discoverable.
use crate::sql::{exists, query_all};
use crate::{Connection, Error, Result};
use rusqlite::params;
use std::collections::{BTreeSet, HashMap};

pub fn ignored(conn: &Connection, viewer: i64, author: i64) -> Result<bool> {
    exists(
        conn,
        "SELECT 1 FROM user_blocks WHERE blocker_id=? AND blocked_id=?",
        params![viewer, author],
    )
}

pub fn ignored_authors(conn: &Connection, viewer: i64) -> Result<BTreeSet<i64>> {
    Ok(query_all(
        conn,
        "SELECT blocked_id FROM user_blocks WHERE blocker_id=?",
        [viewer],
        |row| row.get(0),
    )?
    .into_iter()
    .collect())
}

pub fn blockers_of(conn: &Connection, author: i64) -> Result<Vec<i64>> {
    query_all(
        conn,
        "SELECT blocker_id FROM user_blocks WHERE blocked_id=?",
        [author],
        |row| row.get(0),
    )
}

pub fn blockers_for_authors(
    conn: &Connection,
    authors: impl IntoIterator<Item = i64>,
) -> Result<HashMap<i64, Vec<i64>>> {
    let authors: BTreeSet<_> = authors.into_iter().collect();
    let mut blockers: HashMap<i64, Vec<i64>> = HashMap::new();
    if authors.is_empty() {
        return Ok(blockers);
    }
    let rows = query_all(
        conn,
        "SELECT blocked_id,blocker_id FROM user_blocks WHERE blocked_id IN (SELECT value FROM json_each(?))",
        [serde_json::json!(authors).to_string()],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
    )?;
    for (author, viewer) in rows {
        blockers.entry(author).or_default().push(viewer);
    }
    Ok(blockers)
}

/// SQL expressions here are owned by callers, never request text.
pub fn visible_sql(viewer: &str, author: &str) -> String {
    format!(
        "NOT EXISTS (SELECT 1 FROM user_blocks ignored_person WHERE ignored_person.blocker_id={viewer} AND ignored_person.blocked_id={author})"
    )
}

/// The same unavailable-person error for every refused contact. Existing DMs bypass creation.
pub fn require_contact(conn: &Connection, sender: i64, recipients: &[i64]) -> Result<()> {
    for recipient in recipients {
        if ignored(conn, *recipient, sender)? {
            return Err(Error::RecordNotFound("User"));
        }
    }
    Ok(())
}
