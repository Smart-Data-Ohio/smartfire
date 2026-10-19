//! The channel directory: the alive open rooms (the only rooms anyone may join), searched by name
//! and topic and paged by keyset.
use rusqlite::{Connection, named_params};

use crate::sql::query_all;
use crate::{Result, Timestamp};

pub struct DirectoryRoom {
    /// The room's id and `updated_at` are in its key.
    pub key: DirectoryKey,
    pub name: Option<String>,
    pub topic: Option<String>,
    /// Active users with a membership of any involvement, as `room_members::for_room` lists them.
    pub member_count: i64,
    /// The viewer's membership isn't hidden, so the room shows in their sidebar.
    pub joined: bool,
}

/// A row's place in the order: match rank (0 exact name, 1 name prefix, 2 name anywhere, 3 topic
/// only; 0 for every row with no query), then latest `updated_at`, then highest id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirectoryKey {
    pub rank: i64,
    pub updated_at: Timestamp,
    pub id: i64,
}

const OPEN_ROOMS: &str = r#"SELECT rooms.id, rooms.name, rooms.topic, page.updated_at, page.match_rank,
  (SELECT COUNT(*) FROM memberships JOIN users ON users.id = memberships.user_id WHERE memberships.room_id = rooms.id AND users.status = 0) AS member_count,
  EXISTS(SELECT 1 FROM memberships WHERE memberships.room_id = rooms.id AND memberships.user_id = :viewer AND memberships.involvement != 'invisible') AS joined
FROM (
  SELECT id, updated_at, match_rank FROM (
    SELECT rooms.id, rooms.updated_at, CASE
      WHEN :query = '' THEN 0
      WHEN LOWER(rooms.name) = :query THEN 0
      WHEN LOWER(rooms.name) LIKE :prefix ESCAPE '\' THEN 1
      WHEN LOWER(rooms.name) LIKE :anywhere ESCAPE '\' THEN 2
      WHEN LOWER(rooms.topic) LIKE :anywhere ESCAPE '\' THEN 3
    END AS match_rank
    FROM rooms WHERE rooms.type = 'Rooms::Open' AND rooms.deleted_at IS NULL
      AND NOT (:joinable_only AND EXISTS(SELECT 1 FROM memberships WHERE memberships.room_id = rooms.id AND memberships.user_id = :viewer AND memberships.involvement != 'invisible'))
  )
  WHERE match_rank IS NOT NULL
    AND (:after_id IS NULL OR match_rank > :after_rank OR (match_rank = :after_rank AND (updated_at < :after_at OR (updated_at = :after_at AND id < :after_id))))
  ORDER BY match_rank, updated_at DESC, id DESC
  LIMIT :limit
) AS page JOIN rooms ON rooms.id = page.id
ORDER BY page.match_rank, page.updated_at DESC, page.id DESC"#;

/// Up to `limit` open rooms matching `query` (already trimmed; `""` matches every room) after
/// `after`, for `viewer`.
pub fn open_rooms(
    conn: &Connection,
    viewer: i64,
    query: &str,
    joinable_only: bool,
    after: Option<DirectoryKey>,
    limit: usize,
) -> Result<Vec<DirectoryRoom>> {
    // SQLite's LOWER folds ASCII only; folding more here would miss non-ASCII capitals.
    let query = query.to_ascii_lowercase();
    let literal = query
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    query_all(
        conn,
        OPEN_ROOMS,
        named_params! {
            ":viewer": viewer,
            ":joinable_only": joinable_only,
            ":query": query,
            ":prefix": format!("{literal}%"),
            ":anywhere": format!("%{literal}%"),
            ":after_rank": after.map(|key| key.rank),
            ":after_at": after.map(|key| key.updated_at),
            ":after_id": after.map(|key| key.id),
            ":limit": limit as i64,
        },
        |row| {
            Ok(DirectoryRoom {
                key: DirectoryKey {
                    rank: row.get("match_rank")?,
                    updated_at: row.get("updated_at")?,
                    id: row.get("id")?,
                },
                name: row.get("name")?,
                topic: row.get("topic")?,
                member_count: row.get("member_count")?,
                joined: row.get("joined")?,
            })
        },
    )
}
