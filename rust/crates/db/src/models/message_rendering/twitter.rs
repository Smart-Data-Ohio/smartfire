//! WS15e persisted X card facts; fetches, claims and write callbacks remain provider-owned.
use crate::{Result, Timestamp};
use rusqlite::{Connection, Row};
use serde_json::Value;
use std::collections::HashMap;
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
}

fn json_column(row: &Row<'_>, name: &str) -> rusqlite::Result<Option<Value>> {
    Ok(match row.get_ref(name)? {
        rusqlite::types::ValueRef::Text(bytes) => serde_json::from_slice(bytes).ok(),
        _ => None,
    })
}
pub(super) fn load(conn: &Connection, ids: &[i64]) -> Result<HashMap<i64, Vec<Post>>> {
    let mut result = HashMap::<i64, Vec<Post>>::new();
    for (message, post) in super::rows(
        conn,
        "SELECT r.message_id,p.* FROM twitter_post_references r JOIN twitter_posts p ON p.id=r.twitter_post_id WHERE r.message_id IN ($ids) ORDER BY r.id",
        ids,
        |r| Ok((r.get::<_, i64>("message_id")?, Post::from_row(r)?)),
    )? {
        result.entry(message).or_default().push(post);
    }
    for posts in result.values_mut() {
        posts.sort_by(|a, b| {
            let a = a.post_id.trim_start_matches('0');
            let b = b.post_id.trim_start_matches('0');
            a.len().cmp(&b.len()).then_with(|| a.cmp(b))
        });
    }
    Ok(result)
}
