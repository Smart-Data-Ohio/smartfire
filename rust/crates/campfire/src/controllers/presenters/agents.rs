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

/// Rails public bot profile, using WS11's kind/grants/activity readers.
pub fn profile(
    conn: &Connection,
    secrets: &rails_compat::Secrets,
    user_id: i64,
    viewer: &campfire_db::User,
    now: campfire_db::Timestamp,
    zone: &campfire_views::time::Zone,
) -> Result<Option<campfire_views::agents::Profile>> {
    let Some(agent) = campfire_db::Agent::for_user(conn, user_id)? else {
        return Ok(None);
    };
    let directory = directory_agent(conn, secrets, agent.id)?.expect("same agent row");
    let mut all_rooms = campfire_db::Room::for_user(conn, user_id)?;
    super::accounts::sort_by_lower_name(&mut all_rooms, |r| r.name.as_deref().unwrap_or(""));
    let total = all_rooms.len();
    let rooms = all_rooms
        .into_iter()
        .filter_map(|room| {
            match campfire_db::Membership::find_by_room_and_user(conn, room.id, viewer.id) {
                Ok(Some(_)) => Some(
                    super::accounts::room_display_name(conn, &room, viewer)
                        .map(|name| (room.id, name)),
                ),
                Ok(None) => None,
                Err(e) => Some(Err(e)),
            }
        })
        .collect::<Result<Vec<_>>>()?;
    let management = (viewer.is_administrator() || agent.owner_id == Some(viewer.id))
        .then(|| -> Result<(String, String)> {
            Ok((
                agent.activity_summary(conn, now)?,
                budget_usage_line(conn, &agent, now, zone)?,
            ))
        })
        .transpose()?;
    let present = |v: &Option<String>| v.clone().filter(|s| !campfire_richtext::ruby::is_blank(s));
    Ok(Some(campfire_views::agents::Profile {
        provider_runtime: [present(&agent.provider), present(&agent.runtime)]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" · "),
        description: present(&agent.description),
        has_rooms: total > 0,
        hidden_room_count: total - rooms.len(),
        rooms,
        grants: agent.grants_summary(conn)?,
        management,
        agent: directory,
    }))
}
pub fn budget_usage_line(
    conn: &Connection,
    agent: &campfire_db::Agent,
    now: campfire_db::Timestamp,
    zone: &campfire_views::time::Zone,
) -> Result<String> {
    use campfire_db::models::agent_posting::{self, Cap};
    let window = agent_posting::daily_window(now, zone.tz())?;
    [
        (Cap::Messages, agent.daily_message_cap, "messages"),
        (Cap::BoardPosts, agent.daily_board_post_cap, "board posts"),
        (
            Cap::ExternalActions,
            agent.daily_external_action_cap,
            "external actions",
        ),
    ]
    .into_iter()
    .map(|(cap, limit, noun)| {
        let used = agent_posting::cap_usage(conn, agent.id, agent.user_id, cap, &window)?;
        Ok(limit
            .map(|limit| format!("{used}/{limit} {noun}"))
            .unwrap_or_else(|| format!("{used} {noun}")))
    })
    .collect::<Result<Vec<_>>>()
    .map(|cells| cells.join(" · "))
}
