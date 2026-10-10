//! Read-only facts for users/index and users/cards. Status mutations remain WS17's domain.
use super::User;
use crate::{Result, Timestamp};
use rusqlite::{Connection, named_params};

#[derive(Debug, Clone)]
pub struct AgentIdentity {
    pub id: i64,
    pub owner_id: Option<i64>,
    pub owner_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Person {
    pub user: User,
    pub agent: Option<AgentIdentity>,
    pub starred: bool,
    pub online: bool,
    pub presence: String,
    pub custom_status: Option<String>,
}

/// Rails User.active.ordered, excluding Current.user, partitioned by the viewer's stars.
pub fn directory(conn: &Connection, viewer: i64, now: Timestamp) -> Result<Vec<Person>> {
    let mut people = load(conn, viewer, None, now)?;
    people.sort_by_key(|person| !person.starred);
    Ok(people)
}

pub fn card(conn: &Connection, viewer: i64, id: i64, now: Timestamp) -> Result<Option<Person>> {
    Ok(load(conn, viewer, Some(id), now)?.pop())
}

fn load(conn: &Connection, viewer: i64, id: Option<i64>, now: Timestamp) -> Result<Vec<Person>> {
    let mut stmt = conn.prepare(
        &format!("SELECT u.*, a.id AS agent_id,a.owner_id AS agent_owner_id,{},
         a.suspended_at AS agent_suspended_at,a.last_seen_at AS agent_last_seen_at,
         EXISTS(SELECT 1 FROM user_stars s WHERE s.user_id=:viewer AND s.starred_user_id=u.id) AS starred,
         EXISTS(SELECT 1 FROM workspace_presence_leases l JOIN sessions s ON s.id=l.session_id
           WHERE l.user_id=u.id AND s.user_id=u.id AND u.status=0 AND l.expires_at>=:now) AS live,
         EXISTS(SELECT 1 FROM workspace_presence_leases l JOIN sessions s ON s.id=l.session_id
           WHERE l.user_id=u.id AND s.user_id=u.id AND u.status=0 AND l.expires_at>=:now
           AND (l.last_active_at IS NULL OR l.last_active_at>=:cutoff)) AS active_lease
         FROM users u LEFT JOIN agents a ON a.user_id=u.id LEFT JOIN users owner ON owner.id=a.owner_id
         WHERE (:id IS NOT NULL AND u.id=:id) OR (:id IS NULL AND u.status=0 AND u.id!=:viewer)
         ORDER BY LOWER(u.name)", User::projection("owner", "owner_")),
    )?;
    let rows = stmt.query_map(
        named_params! {
            ":viewer":viewer, ":id":id, ":now":now,
            ":cutoff":now.ago(super::super::workspace_presence_lease::IDLE_AFTER),
        },
        |row| {
            let user = User::from_row(row)?;
            let agent = row
                .get::<_, Option<i64>>("agent_id")?
                .map(|id| {
                    Ok::<_, rusqlite::Error>(AgentIdentity {
                        id,
                        owner_id: row.get("agent_owner_id")?,
                        owner_name: if row.get::<_, Option<i64>>("owner_id")?.is_some() {
                            Some(User::from_prefixed_row(row, "owner_")?.display_name().to_owned())
                        } else { None },
                    })
                })
                .transpose()?;
            let live: bool = row.get("live")?;
            let online = user.is_active()
                && if user.is_bot() && agent.is_some() {
                    row.get::<_, Option<Timestamp>>("agent_suspended_at")?
                        .is_none()
                        && row
                            .get::<_, Option<Timestamp>>("agent_last_seen_at")?
                            .is_some()
                } else {
                    live
                };
            let setting: String = row.get("presence_setting")?;
            let presence = if user.is_bot() && agent.is_some() {
                if online { "agent" } else { "offline" }
            } else if setting == "invisible" || !live {
                "offline"
            } else if setting == "dnd" {
                "dnd"
            } else if row.get::<_, bool>("active_lease")? {
                "online"
            } else {
                "idle"
            };
            let expiry: Option<Timestamp> = row.get("custom_status_expires_at")?;
            let custom_status = if expiry.is_none_or(|expiry| expiry > now) {
                let parts = [
                    row.get::<_, Option<String>>("custom_status_emoji")?,
                    row.get::<_, Option<String>>("custom_status_text")?,
                ];
                let text = parts
                    .into_iter()
                    .flatten()
                    .filter(|s| !s.chars().all(char::is_whitespace))
                    .collect::<Vec<_>>()
                    .join(" ");
                (!text.is_empty()).then_some(text)
            } else {
                None
            };
            Ok(Person {
                user,
                agent,
                starred: row.get("starred")?,
                online,
                presence: presence.into(),
                custom_status,
            })
        },
    )?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}
