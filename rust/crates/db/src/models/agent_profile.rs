//! Remaining directory ordering adapter. Bot profile and cap fields use WS11 Agent.
mod downcase_table;

use crate::{Connection, Result, Timestamp};

/// Public directory facts only. Suspension and banned-user activity are distinct.
#[derive(Clone, Debug)]
pub struct DirectoryRecord {
    pub id: i64,
    pub user: crate::User,
    pub owner: Option<crate::User>,
    pub kind: String,
    pub status: String,
    pub status_note: Option<String>,
    pub suspended: bool,
    pub created_at: Timestamp,
    pub status_changed_at: Option<Timestamp>,
    pub last_seen_at: Option<Timestamp>,
}

/// FLAGGED WS11 directory adapter: preserve the pinned Ruby downcase table until
/// Agent::for_directory uses it instead of Rust's contextual Unicode lowercase.
/// No deactivated users; active first, Ruby Unicode downcase order.
pub fn for_directory(conn: &Connection) -> Result<Vec<DirectoryRecord>> {
    let mut records = crate::sql::query_all(conn,
        "SELECT a.id, a.user_id, a.owner_id, a.kind, a.status, a.status_note, a.suspended_at, a.created_at, a.status_changed_at, a.last_seen_at FROM agents a JOIN users u ON u.id=a.user_id WHERE u.status != 1", [], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?, r.get::<_, Option<i64>>(2)?,
                r.get::<_, String>(3)?, r.get::<_, String>(4)?, r.get::<_, Option<String>>(5)?,
                r.get::<_, Option<Timestamp>>(6)?, r.get::<_, Timestamp>(7)?, r.get::<_, Option<Timestamp>>(8)?, r.get::<_, Option<Timestamp>>(9)?))
        })?.into_iter().map(|(id, user_id, owner_id, kind, status, status_note, suspended_at, created_at, status_changed_at, last_seen_at)| {
            Ok(DirectoryRecord { id, user: crate::User::find(conn, user_id)?,
                owner: owner_id.map(|id| crate::User::find_by_id(conn, id)).transpose()?.flatten(),
                kind, status, status_note, suspended: suspended_at.is_some(), created_at, status_changed_at, last_seen_at })
        }).collect::<Result<Vec<_>>>()?;
    records.sort_by_cached_key(|record| {
        (
            !(record.user.is_active() && !record.suspended),
            ruby_downcase(&record.user.name),
        )
    });
    Ok(records)
}

fn ruby_downcase(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    for character in value.chars() {
        match downcase_table::MAPPINGS.binary_search_by_key(&character, |&(character, _)| character) {
            Ok(index) => result.push_str(downcase_table::MAPPINGS[index].1),
            Err(_) => result.push(character),
        }
    }
    result
}
