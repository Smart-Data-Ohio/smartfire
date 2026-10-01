//! User-page adapter over the merged WS11 agent model. Private rooms never become view facts.
use campfire_db::{Agent, Connection, Result, Timestamp, User};
use campfire_richtext::ruby::is_blank;
use campfire_views::users::AgentProfile;
pub fn load(
    conn: &Connection,
    user: &User,
    viewer: &User,
    now: Timestamp,
) -> Result<(Option<AgentProfile>, bool)> {
    let agent = Agent::for_user(conn, user.id)?;
    let manage = user.is_bot()
        && (viewer.is_administrator()
            || agent
                .as_ref()
                .is_some_and(|a| a.owner_id == Some(viewer.id)));
    let Some(agent) = agent else {
        return Ok((None, manage));
    };
    let mut query = conn.prepare("SELECT rooms.id FROM rooms INNER JOIN memberships ON memberships.room_id=rooms.id WHERE memberships.user_id=? AND rooms.deleted_at IS NULL ORDER BY LOWER(rooms.name)")?;
    let ids = query
        .query_map([user.id], |r| r.get::<_, i64>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let rooms = ids
        .into_iter()
        .map(|id| campfire_db::Room::find(conn, id))
        .collect::<Result<Vec<_>>>()?;
    let mut shared = Vec::new();
    for room in &rooms {
        if campfire_db::Membership::find_by_room_and_user(conn, room.id, viewer.id)?.is_some() {
            shared.push((
                room.id,
                if room.direct() {
                    room.direct_display_name(conn, Some(viewer), None)?
                        .unwrap_or_default()
                } else {
                    room.name.clone().unwrap_or_default()
                },
            ));
        }
    }
    let provider = agent.provider.as_deref().filter(|s| !is_blank(s));
    let runtime = agent.runtime.as_deref().filter(|s| !is_blank(s));
    let provider_runtime = provider
        .into_iter()
        .chain(runtime)
        .collect::<Vec<_>>()
        .join(" · ");
    // The profile helper uses the viewer's request Time.zone, as Rails Date.current does.
    let zone = conn.query_row("SELECT time_zone FROM users WHERE id=?", [viewer.id], |r| {
        r.get::<_, Option<String>>(0)
    })?;
    let zone = campfire_views::time::Zone::for_user(zone.as_deref());
    let budget_usage = if manage {
        Some(usage(conn, &agent, now, zone.tz())?)
    } else {
        None
    };
    Ok((
        Some(AgentProfile {
            id: agent.id,
            kind_description: agent.kind_description(conn)?,
            provider_runtime: (!provider_runtime.is_empty()).then_some(provider_runtime),
            description: agent.description.clone().filter(|s| !is_blank(s)),
            status: agent.status.clone(),
            status_note: agent.status_note.clone().filter(|s| !is_blank(s)),
            status_since: agent.status_changed_at.unwrap_or(agent.created_at).jiff(),
            last_seen_at: agent.last_seen_at.map(|t| t.jiff()),
            suspended: agent.suspended(),
            hidden_rooms: rooms.len() - shared.len(),
            rooms: shared,
            grants_summary: agent.grants_summary(conn)?,
            activity_summary: if manage {
                Some(agent.activity_summary(conn, now)?)
            } else {
                None
            },
            budget_usage,
            now: now.jiff(),
        }),
        manage,
    ))
}
fn usage(
    conn: &Connection,
    agent: &Agent,
    now: Timestamp,
    zone: &jiff::tz::TimeZone,
) -> Result<String> {
    let day = campfire_db::models::agent_posting::daily_window(now, zone)?;
    let queries = [
        (
            "SELECT COUNT(*) FROM messages WHERE creator_id=? AND created_at BETWEEN ? AND ? AND board_post_opener=0",
            agent.user_id,
            agent.daily_message_cap,
            "messages",
        ),
        (
            "SELECT COUNT(*) FROM channel_threads WHERE creator_id=? AND created_at BETWEEN ? AND ? AND room_id IN (SELECT id FROM rooms WHERE type='Rooms::Board')",
            agent.user_id,
            agent.daily_board_post_cap,
            "board posts",
        ),
        (
            "SELECT COUNT(*) FROM agent_approvals WHERE agent_id=? AND created_at BETWEEN ? AND ?",
            agent.id,
            agent.daily_external_action_cap,
            "external actions",
        ),
    ];
    let mut cells = Vec::new();
    for (sql, id, limit, noun) in queries {
        let count: i64 =
            conn.query_row(sql, rusqlite::params![id, day.start, day.end], |r| r.get(0))?;
        cells.push(match limit {
            Some(limit) => format!("{count}/{limit} {noun}"),
            None => format!("{count} {noun}"),
        });
    }
    Ok(cells.join(" · "))
}
