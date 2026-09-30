//! Domain-to-view adapter for the two owned settings forms. Queries happen before rendering.
use campfire_db::{Connection, Errors, Timestamp, UserStatusSettings};
use campfire_views::users::{SettingsFormData, SettingsPerson};
use rusqlite::OptionalExtension;

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

pub fn google_configured() -> bool {
    ["GOOGLE_CLIENT_ID", "GOOGLE_CLIENT_SECRET"]
        .iter()
        .all(|key| {
            std::env::var(key)
                .ok()
                .is_some_and(|s| !s.chars().all(char::is_whitespace))
        })
}
