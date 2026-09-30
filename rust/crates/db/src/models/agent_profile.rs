//! Read-only Agent association seam for the management pages. Lifecycle writes remain WS11.
use crate::sql::query_one;
use crate::{Connection, Result, Timestamp};

#[derive(Clone, Debug)]
pub struct AgentProfile {
    pub id: i64,
    pub owner_id: Option<i64>,
    pub kind: String,
    pub provider: Option<String>,
    pub runtime: Option<String>,
    pub description: Option<String>,
    pub daily_message_cap: Option<i64>,
    pub daily_board_post_cap: Option<i64>,
    pub daily_external_action_cap: Option<i64>,
    pub suspended_at: Option<Timestamp>,
    pub encrypted_webhook_signing_secret: Option<String>,
}
impl AgentProfile {
    pub fn for_user(conn: &Connection, user_id: i64) -> Result<Option<Self>> {
        query_one(
            conn,
            "SELECT * FROM agents WHERE user_id=? LIMIT 1",
            [user_id],
            |r| {
                Ok(Self {
                    id: r.get("id")?,
                    owner_id: r.get("owner_id")?,
                    kind: r.get("kind")?,
                    provider: r.get("provider")?,
                    runtime: r.get("runtime")?,
                    description: r.get("description")?,
                    daily_message_cap: r.get("daily_message_cap")?,
                    daily_board_post_cap: r.get("daily_board_post_cap")?,
                    daily_external_action_cap: r.get("daily_external_action_cap")?,
                    suspended_at: r.get("suspended_at")?,
                    encrypted_webhook_signing_secret: r.get("webhook_signing_secret")?,
                })
            },
        )
    }
}

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

/// Agent.for_directory: no deactivated users; active first, Ruby Unicode downcase order.
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
            record.user.name.to_lowercase(),
        )
    });
    Ok(records)
}
