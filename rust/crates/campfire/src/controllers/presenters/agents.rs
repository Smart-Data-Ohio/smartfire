//! Safe facts for the human directory. No credential, grant or signing secret enters a view model.
pub mod history;
use super::{attachments, resolve_avatar_icon, user_summary};
use campfire_db::{Connection, Result};
use campfire_views::agents::DirectoryAgent;

pub fn directory(
    conn: &Connection,
    secrets: &rails_compat::Secrets,
) -> Result<Vec<DirectoryAgent>> {
    campfire_db::models::agent_profile::for_directory(conn)?
        .into_iter()
        .map(|record| directory_record(conn, secrets, record))
        .collect()
}

/// One row for WS11's committed status callback, including deactivated users:
/// Rails broadcasts the existing row even when the directory excludes it.
pub fn directory_agent(
    conn: &Connection,
    secrets: &rails_compat::Secrets,
    id: i64,
) -> Result<Option<DirectoryAgent>> {
    let Some(agent) = campfire_db::Agent::find(conn, id)? else {
        return Ok(None);
    };
    let record = campfire_db::models::agent_profile::DirectoryRecord {
        id: agent.id,
        user: campfire_db::User::find(conn, agent.user_id)?,
        owner: agent
            .owner_id
            .map(|id| campfire_db::User::find_by_id(conn, id))
            .transpose()?
            .flatten(),
        kind: agent.kind.name().into(),
        status: agent.status,
        status_note: agent.status_note,
        suspended: agent.suspended_at.is_some(),
        created_at: agent.created_at,
        status_changed_at: agent.status_changed_at,
        last_seen_at: agent.last_seen_at,
    };
    directory_record(conn, secrets, record).map(Some)
}
fn directory_record(
    conn: &Connection,
    secrets: &rails_compat::Secrets,
    record: campfire_db::models::agent_profile::DirectoryRecord,
) -> Result<DirectoryAgent> {
    let user = &record.user;
    let icon_name: Option<String> =
        conn.query_row("SELECT icon_name FROM users WHERE id=?", [user.id], |row| {
            row.get(0)
        })?;
    let icon = if user.is_bot()
        && attachments::attached_blob(conn, "User", user.id, "avatar")?.is_none()
    {
        icon_name
            .as_deref()
            .and_then(|name| resolve_avatar_icon(conn, name))
    } else {
        None
    };
    let kind_description = match record.owner {
        None => "no owner recorded".into(),
        Some(owner) if record.kind == "personal" => {
            format!("Personal agent of {}", owner.name)
        }
        Some(owner) => format!("Workspace agent, managed by {}", owner.name),
    };
    Ok(DirectoryAgent {
        id: record.id,
        user: user_summary(secrets, user),
        icon,
        kind_description,
        status: record.status,
        status_note: record.status_note,
        suspended: record.suspended,
        created_at: record.created_at.jiff(),
        status_changed_at: record.status_changed_at.map(|time| time.jiff()),
        last_seen_at: record.last_seen_at.map(|time| time.jiff()),
    })
}
