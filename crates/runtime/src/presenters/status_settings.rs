//! Effective profile status shared by the people API and status broadcasts.
use campfire_db::{Connection, Timestamp, UserStatusSettings};

pub fn profile_status(
    conn: &Connection,
    user_id: i64,
    viewer_id: i64,
    now: Timestamp,
) -> campfire_db::Result<campfire_presentation::users::statuses::ProfileStatus> {
    profile_status_in_zone(
        conn, user_id, viewer_id, now,
        crate::context::renderer_time_zone().tz(),
    )
}

pub fn profile_status_in_zone(
    conn: &Connection,
    user_id: i64,
    viewer_id: i64,
    now: Timestamp,
    request_zone: &jiff::tz::TimeZone,
) -> campfire_db::Result<campfire_presentation::users::statuses::ProfileStatus> {
    let user = UserStatusSettings::find(conn, user_id)?;
    let leases = campfire_db::WorkspacePresenceLease::presence_by_user_id(conn, &[user_id], now)?;
    let presence = user.effective_presence(
        leases
            .get(&user_id)
            .copied()
            .unwrap_or(campfire_db::models::workspace_presence_lease::Presence::Offline),
    );
    let presence = match presence {
        campfire_db::models::workspace_presence_lease::Presence::Online => "online",
        campfire_db::models::workspace_presence_lease::Presence::Idle => "idle",
        campfire_db::models::workspace_presence_lease::Presence::Dnd => "dnd",
        campfire_db::models::workspace_presence_lease::Presence::Offline => "offline",
    };
    Ok(campfire_presentation::users::statuses::ProfileStatus {
        user_id,
        presence: presence.into(),
        status_text: user.status_text_display_in_zone(now, request_zone),
        dnd_allowed: campfire_db::DndAllowedUser::find(conn, viewer_id, user_id)?.is_some(),
    })
}
