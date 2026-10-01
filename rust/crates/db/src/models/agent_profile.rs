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
    pub status_changed_at: Option<Timestamp>,
    pub last_seen_at: Option<Timestamp>,
}

/// The owner supplies Rails Unicode ordering and active/suspended grouping.
pub fn for_directory(conn: &Connection) -> Result<Vec<DirectoryRecord>> {
    crate::Agent::for_directory(conn)?
        .into_iter()
        .map(|agent| {
            Ok(DirectoryRecord {
                id: agent.id,
                user: crate::User::find(conn, agent.user_id)?,
                owner: agent
                    .owner_id
                    .map(|id| crate::User::find_by_id(conn, id))
                    .transpose()?
                    .flatten(),
                kind: agent.kind.name().into(),
                status: agent.status,
                status_note: agent.status_note,
                suspended: agent.suspended_at.is_some(),
                created_at: agent.created_at,
                status_changed_at: agent.status_changed_at,
                last_seen_at: agent.last_seen_at,
            })
        })
        .collect()
}
