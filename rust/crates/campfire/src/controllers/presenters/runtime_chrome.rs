//! Read-only adapters for layout chrome. These never call notification policy,
//! decrypt Google tokens, refresh calendars, or send anything to a transport.
#[cfg(test)]
use campfire_db::{CachedStatements, Connection, Timestamp};
#[cfg(test)]
use campfire_views::layouts::{RecentSearch, UserPreferences};
#[cfg(test)]
use rusqlite::OptionalExtension;
#[cfg(test)]
pub(crate) fn recent_searches(
    conn: &Connection,
    user_id: Option<i64>,
) -> campfire_db::Result<Vec<RecentSearch>> {
    let Some(user_id) = user_id else {
        return Ok(Vec::new());
    };
    Ok(conn
        .prepare_cached(
            "SELECT id,query FROM searches WHERE user_id=? ORDER BY updated_at DESC LIMIT 10",
        )?
        .query_map([user_id], |r| {
            Ok(RecentSearch {
                id: r.get(0)?,
                query: r.get(1)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?)
}
#[cfg(test)]
pub(crate) fn preferences(
    conn: &Connection,
    user_id: i64,
    now: Timestamp,
    mut preferences: UserPreferences,
) -> campfire_db::Result<UserPreferences> {
    preferences.notification_sounds = Default::default();
    let (dnd,until,presence,hours,start,end,meeting_dnd,meeting_status,ooo_notify,ooo_calendar,ooo_until)=conn.query_row_cached("SELECT dnd_enabled,dnd_until,presence_setting,quiet_hours_enabled,quiet_hours_start_minute,quiet_hours_end_minute,meeting_dnd_enabled,meeting_status_enabled,ooo_notify_enabled,ooo_calendar_enabled,ooo_until FROM users WHERE id=?",[user_id],|r|Ok((r.get::<_,bool>(0)?,r.get::<_,Option<Timestamp>>(1)?,r.get::<_,String>(2)?,r.get::<_,bool>(3)?,r.get::<_,Option<i64>>(4)?,r.get::<_,Option<i64>>(5)?,r.get::<_,bool>(6)?,r.get::<_,bool>(7)?,r.get::<_,bool>(8)?,r.get::<_,bool>(9)?,r.get::<_,Option<Timestamp>>(10)?)))?;
    preferences.notification_sounds.muted =
        (dnd && until.is_none_or(|until| until > now)) || presence == "dnd";
    preferences.notification_sounds.quiet_hours = if hours { start.zip(end) } else { None };
    if (meeting_dnd && meeting_status) || (!ooo_notify && ooo_calendar) {
        let cache = conn
            .query_row_cached(
                "SELECT busy_intervals,ooo_intervals FROM calendar_meeting_caches WHERE user_id=?",
                [user_id],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
            )
            .optional()?;
        if let Some((busy, ooo)) = cache {
            let zone = campfire_views::time::Zone::for_user(preferences.time_zone.as_deref());
            if meeting_dnd && meeting_status {
                preferences.notification_sounds.meeting_quiet = epochs(&busy, &zone);
            }
            if !ooo_notify && ooo_calendar {
                preferences.notification_sounds.ooo_quiet = epochs(&ooo, &zone);
            }
        }
    }
    if !ooo_notify && let Some(until) = ooo_until.filter(|until| *until > now) {
        preferences
            .notification_sounds
            .ooo_quiet
            .insert(0, (0, until.as_second()));
    }
    let scopes = conn
        .query_row_cached(
            "SELECT scopes FROM google_accounts WHERE user_id=?",
            [user_id],
            |r| r.get::<_, Option<String>>(0),
        )
        .optional()?
        .flatten();
    // Rails drive? deliberately ignores connected?/usable?: no token is read.
    preferences.google_drive = scopes.is_some_and(|s| {
        s.split_whitespace()
            .any(|scope| scope == "https://www.googleapis.com/auth/drive.file")
    });
    Ok(preferences)
}
pub(crate) fn epochs(raw: &str, zone: &campfire_views::time::Zone) -> Vec<(i64, i64)> {
    let Ok(serde_json::Value::Array(pairs)) = serde_json::from_str(raw) else {
        return Vec::new();
    };
    pairs
        .iter()
        .filter_map(|pair| {
            let pair = pair.as_array()?;
            let parse = |value: &serde_json::Value| {
                let raw = value.as_str()?;
                if let Ok(at) = raw.parse::<jiff::Timestamp>() {
                    return Some(at.as_second());
                }
                let local = raw.parse::<jiff::civil::DateTime>().ok().or_else(|| {
                    raw.parse::<jiff::civil::Date>()
                        .ok()
                        .map(|date| date.at(0, 0, 0, 0))
                })?;
                zone.tz()
                    .to_ambiguous_timestamp(local)
                    .compatible()
                    .ok()
                    .map(|at| at.as_second())
            };
            Some((parse(pair.first()?)?, parse(pair.get(1)?)?))
        })
        .collect()
}
