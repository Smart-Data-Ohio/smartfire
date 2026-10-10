//! Domain-to-view adapter for the two owned settings forms. Queries happen before rendering.
use campfire_db::{Connection, Errors, Timestamp, UserStatusSettings};
use campfire_presentation::users::{SettingsFormData, SettingsPerson};
use rusqlite::OptionalExtension;

/// RoomsController#show's uncached, ordered other active human DM members. No OOO filter:
/// a member whose OOO begins later must already have a live stream mounted on this page.
pub fn ooo_notice_members(
    conn: &Connection,
    secrets: &rails_compat::Secrets,
    room: &campfire_db::Room,
    viewer_id: i64,
    now: Timestamp,
) -> campfire_db::Result<Vec<campfire_presentation::users::statuses::OooNoticeMember>> {
    if room.room_type != campfire_db::RoomType::Direct {
        return Ok(Vec::new());
    }
    let mut q = conn.prepare("SELECT users.id FROM users INNER JOIN memberships ON memberships.user_id=users.id WHERE memberships.room_id=? AND users.id!=? AND users.status=? AND users.role!=? ORDER BY LOWER(users.name)")?;
    let ids = q
        .query_map(
            rusqlite::params![
                room.id,
                viewer_id,
                campfire_db::Status::Active,
                campfire_db::Role::Bot
            ],
            |r| r.get::<_, i64>(0),
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    // The viewer provides Time.zone when a peer has no stored zone. Include it in
    // the same association preload, rather than adding one query per notice.
    let mut preload_ids = ids.clone();
    preload_ids.push(viewer_id);
    let mut users = UserStatusSettings::for_ids(conn, &preload_ids)?;
    let request_zone = users
        .get(&viewer_id)
        .map(UserStatusSettings::zone)
        .unwrap_or(jiff::tz::TimeZone::UTC);
    Ok(ids
        .into_iter()
        .map(|id| {
            let user = users.remove(&id).expect("batch includes selected user");
            let gid = crate::cable::user_gid(id).to_param();
            campfire_presentation::users::statuses::OooNoticeMember {
                id,
                name: user.user.name.clone(),
                visible: user.ooo_status_visible(now),
                until_date: user.ooo_until_date_in_zone(now, &request_zone),
                note: user
                    .ooo_note
                    .clone()
                    .filter(|note| user.manual_ooo_active(now) && !note.trim().is_empty()),
                stream_name: rails_compat::turbo::signed_stream_name(
                    secrets,
                    &[&gid, "ooo_notice"],
                ),
            }
        })
        .collect())
}

pub fn profile_status(
    conn: &Connection,
    secrets: &rails_compat::Secrets,
    user_id: i64,
    viewer_id: i64,
    now: Timestamp,
) -> campfire_db::Result<campfire_presentation::users::statuses::ProfileStatus> {
    profile_status_in_zone(
        conn, secrets, user_id, viewer_id, now,
        crate::context::renderer_time_zone().tz(),
    )
}

pub fn profile_status_in_zone(
    conn: &Connection,
    secrets: &rails_compat::Secrets,
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
    let gid = crate::cable::user_gid(user_id).to_param();
    Ok(campfire_presentation::users::statuses::ProfileStatus {
        user_id,
        stream_name: rails_compat::turbo::signed_stream_name(secrets, &[&gid, "status"]),
        presence: presence.into(),
        status_text: user.status_text_display_in_zone(now, request_zone),
        dnd_allowed: campfire_db::DndAllowedUser::find(conn, viewer_id, user_id)?.is_some(),
    })
}

pub fn forms(
    conn: &Connection,
    user: &UserStatusSettings,
    errors: Errors,
    now: Timestamp,
    google_configured: bool,
) -> campfire_db::Result<SettingsFormData> {
    let id = user.user.id;
    let google: Option<(String, Option<String>, Option<String>)> = conn
        .query_row(
            "SELECT email,disconnected_reason,scopes FROM google_accounts WHERE user_id=?",
            [id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;
    let blank = |s: &str| s.chars().all(char::is_whitespace);
    let calendar_connected = google.as_ref().is_some_and(|(_, reason, scopes)| {
        reason.as_deref().is_none_or(blank)
            && scopes.as_deref().is_none_or(|s| {
                blank(s)
                    || s.split_whitespace()
                        .any(|s| s == "https://www.googleapis.com/auth/calendar.events")
            })
    });
    let fetch_error = conn
        .query_row(
            "SELECT fetch_error FROM calendar_meeting_caches WHERE user_id=?",
            [id],
            |r| r.get::<_, Option<String>>(0),
        )
        .optional()?
        .flatten();
    let mut stmt = conn.prepare("SELECT users.id,users.name FROM users INNER JOIN dnd_allowed_users ON users.id=dnd_allowed_users.allowed_user_id WHERE dnd_allowed_users.user_id=? ORDER BY LOWER(users.name)")?;
    let allowed_people = stmt
        .query_map([id], |r| {
            Ok(SettingsPerson {
                id: r.get(0)?,
                name: r.get(1)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut stmt =
        conn.prepare("SELECT phrase FROM keyword_alerts WHERE user_id=? ORDER BY phrase")?;
    let keyword_alerts = stmt
        .query_map([id], |r| r.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?
        .join("\n");
    let clock = campfire_db::models::user_status_settings::minutes_to_clock_time;
    Ok(SettingsFormData {
        theme: user.theme.clone(),
        text_size: user.text_size.clone(),
        time_zone: user.time_zone.clone(),
        time_zone_choices: campfire_presentation::users::profile_time_zone_choices(now.jiff()),
        presence_setting: user.presence_setting.clone(),
        custom_status_emoji: user.custom_status_emoji.clone(),
        custom_status_text: user.custom_status_text.clone(),
        ooo_note: user.ooo_note.clone(),
        dnd_active: user.manual_dnd_active(now),
        quiet_hours_enabled: user.quiet_hours_enabled,
        quiet_hours_start: clock(user.quiet_hours_start_minute),
        quiet_hours_end: clock(user.quiet_hours_end_minute),
        meeting_status_enabled: user.meeting_status_enabled,
        meeting_dnd_enabled: user.meeting_dnd_enabled,
        ooo_calendar_enabled: user.ooo_calendar_enabled,
        ooo_notify_enabled: user.ooo_notify_enabled,
        out_of_office: user.out_of_office(now),
        manual_ooo_active: user.manual_ooo_active(now),
        ooo_until_date: user.ooo_until_effective(now).map(|end| {
            end.jiff()
                .to_zoned(user.zone())
                .strftime("%B %d, %Y")
                .to_string()
        }),
        google_configured,
        calendar_connected,
        google_email: google.map(|(email, _, _)| email),
        fetch_error,
        allowed_people,
        keyword_alerts,
        errors: errors
            .0
            .into_iter()
            .map(|(attribute, message)| (attribute.into(), message))
            .collect(),
    })
}
