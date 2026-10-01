//! Read-only application-layout projections from the persisted owner settings.
//! WS17 supplies the notification/status domain; WS14g replaces the raw Calendar/Google
//! cache reads with its typed APIs. This adapter neither refreshes caches nor decrypts tokens.
use campfire_db::{Connection, Result, Timestamp};
use campfire_views::layouts::{NotificationSounds, UserPreferences};
use rusqlite::OptionalExtension;

pub(super) fn fill(
    conn: &Connection,
    user_id: i64,
    now: jiff::Timestamp,
    preferences: &mut UserPreferences,
) -> Result<()> {
    let (mut sounds, meetings, calendar_ooo) = conn.query_row(
        "SELECT dnd_enabled,dnd_until,presence_setting,quiet_hours_enabled,quiet_hours_start_minute,quiet_hours_end_minute,meeting_status_enabled,meeting_dnd_enabled,ooo_until,ooo_calendar_enabled,ooo_notify_enabled FROM users WHERE id=?",
        [user_id],
        |row| {
            let dnd: bool = row.get(0)?;
            let dnd_until: Option<Timestamp> = row.get(1)?;
            let presence: String = row.get(2)?;
            let quiet: bool = row.get(3)?;
            let start: Option<i64> = row.get(4)?;
            let end: Option<i64> = row.get(5)?;
            let ooo_until: Option<Timestamp> = row.get(8)?;
            let keep_notifications: bool = row.get(10)?;
            Ok((
                NotificationSounds {
                    muted: (dnd && dnd_until.is_none_or(|until| until.jiff() > now)) || presence == "dnd",
                    quiet_hours: if quiet { start.zip(end) } else { None },
                    meeting_quiet: Vec::new(),
                    ooo_quiet: if keep_notifications { Vec::new() } else {
                        ooo_until.filter(|until| until.jiff() > now).map(|until| vec![(0, until.as_second())]).unwrap_or_default()
                    },
                },
                row.get::<_, bool>(6)? && row.get::<_, bool>(7)?,
                !keep_notifications && row.get::<_, bool>(9)?,
            ))
        },
    )?;
    if (meetings || calendar_ooo)
        && let Some((busy, ooo)) = conn
            .query_row(
                "SELECT busy_intervals,ooo_intervals FROM calendar_meeting_caches WHERE user_id=?",
                [user_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?
    {
        if meetings {
            sounds.meeting_quiet = window_epochs(&busy);
        }
        if calendar_ooo {
            sounds.ooo_quiet.extend(window_epochs(&ooo));
        }
    }
    preferences.notification_sounds = sounds;
    // GoogleAccount#drive? checks the exact scope, including disconnected accounts. It does
    // not check workspace configuration, connected? or usable? and must never decrypt here.
    let scopes: Option<String> = conn
        .query_row(
            "SELECT scopes FROM google_accounts WHERE user_id=?",
            [user_id],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()?
        .flatten();
    preferences.google_drive = scopes.as_deref().is_some_and(|scopes| {
        scopes
            .split([' ', '\t', '\n', '\r', '\u{000b}', '\u{000c}'])
            .any(|scope| scope == "https://www.googleapis.com/auth/drive.file")
    });
    Ok(())
}

/// Calendar::MeetingCache#quiet_window_epochs / #ooo_window_epochs. The refresh producer
/// stores ISO 8601 pairs. Preserve order, duplicates, reversed intervals and future windows:
/// the browser decides whether a window is active on each play, rather than at render time.
/// WS14g owns the broader Time.zone.parse grammar for non-producer cache values.
fn window_epochs(raw: &str) -> Vec<(i64, i64)> {
    windows(raw)
        .into_iter()
        .map(|(start, end)| (start.as_second(), end.as_second()))
        .collect()
}

fn windows(raw: &str) -> Vec<(jiff::Timestamp, jiff::Timestamp)> {
    let pairs: serde_json::Value = serde_json::from_str(raw).unwrap_or_default();
    pairs
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|pair| {
            let pair = pair.as_array()?;
            let start = pair.first()?.as_str()?.parse::<jiff::Timestamp>().ok()?;
            let end = pair.get(1)?.as_str()?.parse::<jiff::Timestamp>().ok()?;
            Some((start, end))
        })
        .collect()
}
