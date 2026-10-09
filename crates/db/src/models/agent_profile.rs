//! Directory presentation facts projected from WS11 Agent; ordering stays with the owner.

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
    pub updated_at: Timestamp,
    pub status_changed_at: Option<Timestamp>,
    pub last_seen_at: Option<Timestamp>,
}

/// The owner supplies Rails Unicode ordering and active/suspended grouping.
pub fn for_directory(conn: &Connection) -> Result<Vec<DirectoryRecord>> {
    Ok(crate::Agent::directory_rows(conn)?
        .into_iter()
        .map(|(agent, user, owner)| DirectoryRecord {
            id: agent.id,
            user,
            owner,
            kind: agent.kind.name().into(),
            status: agent.status,
            status_note: agent.status_note,
            suspended: agent.suspended_at.is_some(),
            created_at: agent.created_at,
            updated_at: agent.updated_at,
            status_changed_at: agent.status_changed_at,
            last_seen_at: agent.last_seen_at,
        })
        .collect())
}
