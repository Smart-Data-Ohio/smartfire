//! Read-only application-layout projections from the persisted owner settings.
//! WS17 supplies the notification/status domain; WS14g replaces the raw Calendar/Google
//! cache reads with its typed APIs. This adapter neither refreshes caches nor decrypts tokens.
use campfire_db::{Connection, Result, Timestamp};
use campfire_views::layouts::{NotificationSounds, UserPreferences};
use rusqlite::OptionalExtension;

/// Preferences carried by the authenticated row, for read-only page requests.
#[derive(Clone)]
pub(crate) struct LoadedPreferences {
    pub user_id: i64,
    preferences: UserPreferences,
    meetings: bool,
    calendar_ooo: bool,
}
impl LoadedPreferences {
    pub(crate) fn from_row(
        row: &rusqlite::Row<'_>,
        now: jiff::Timestamp,
    ) -> rusqlite::Result<Self> {
        let dnd: bool = row.get("dnd_enabled")?;
        let dnd_until: Option<Timestamp> = row.get("dnd_until")?;
        let presence: String = row.get("presence_setting")?;
        let quiet: bool = row.get("quiet_hours_enabled")?;
        let start: Option<i64> = row.get("quiet_hours_start_minute")?;
        let end: Option<i64> = row.get("quiet_hours_end_minute")?;
        let ooo_until: Option<Timestamp> = row.get("ooo_until")?;
        let keep_notifications: bool = row.get("ooo_notify_enabled")?;
        Ok(Self {
            user_id: row.get("id")?,
            preferences: UserPreferences {
                theme: row.get("theme")?,
                text_size: row.get("text_size")?,
                time_zone: row.get("time_zone")?,
                time_zone_explicit: row.get("time_zone_explicit")?,
                tour_completed: completed_tour(row.get_ref("tour_completed_at")?),
                voice_mode: row.get("voice_mode")?,
                push_to_talk_key: row.get("push_to_talk_key")?,
                notification_sounds: NotificationSounds {
                    muted: (dnd && dnd_until.is_none_or(|until| until.jiff() > now))
                        || presence == "dnd",
                    quiet_hours: if quiet { start.zip(end) } else { None },
                    meeting_quiet: Vec::new(),
                    ooo_quiet: if keep_notifications {
                        Vec::new()
                    } else {
                        ooo_until
                            .filter(|until| until.jiff() > now)
                            .map(|until| vec![(0, until.as_second())])
                            .unwrap_or_default()
                    },
                },
                ..Default::default()
            },
            meetings: row.get::<_, bool>("meeting_status_enabled")?
                && row.get::<_, bool>("meeting_dnd_enabled")?,
            calendar_ooo: !keep_notifications && row.get::<_, bool>("ooo_calendar_enabled")?,
        })
    }

    pub(crate) fn load(mut self, conn: &Connection) -> Result<UserPreferences> {
        let user_id = self.user_id;
        let meetings = self.meetings;
        let calendar_ooo = self.calendar_ooo;
        let sounds = &mut self.preferences.notification_sounds;
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
        self.preferences.google_drive = scopes.as_deref().is_some_and(|scopes| {
            scopes
                .split([' ', '\t', '\n', '\r', '\u{000b}', '\u{000c}'])
                .any(|scope| scope == "https://www.googleapis.com/auth/drive.file")
        });
        Ok(self.preferences)
    }
}

pub(super) fn for_user(
    conn: &Connection,
    user_id: i64,
    now: jiff::Timestamp,
) -> Result<UserPreferences> {
    let loaded = conn.query_row("SELECT * FROM users WHERE id=?", [user_id], |row| {
        LoadedPreferences::from_row(row, now)
    })?;
    loaded.load(conn)
}

/// layouts/_tour.html.erb checks the deserialized datetime's nil?, not SQL NULL.
fn completed_tour(value: rusqlite::types::ValueRef<'_>) -> bool {
    use campfire_db::slash_commands::time_parser::parse_calendar;
    use rusqlite::types::ValueRef;
    match value {
        ValueRef::Null => false,
        // ActiveModel::Type::DateTime preserves non-string scalar values.
        ValueRef::Integer(_) | ValueRef::Real(_) => true,
        ValueRef::Text(raw) | ValueRef::Blob(raw) => {
            let Ok(text) = std::str::from_utf8(raw) else {
                return false;
            };
            if Timestamp::parse_db(text).is_some() {
                return true;
            }
            if text.len() > 128 {
                return false;
            }
            // ActiveModel's fallback needs a supplied year. The owner calendar
            // parser fills omitted fields from its clock; different year defaults
            // distinguish those partial inputs from an actual persisted datetime.
            let zone = jiff::tz::TimeZone::UTC;
            let Some(parsed) = parse_calendar(text, &zone, Timestamp::from_second(0))
                .ok()
                .flatten()
            else {
                return false;
            };
            parse_calendar(text, &zone, Timestamp::from_second(946_684_800))
                .ok()
                .flatten()
                == Some(parsed)
        }
    }
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
