//! Notifications::Policy(kind: :reminder) and User::StatusSettings. No sender/DND exception.
//! WS17 can replace this narrow adapter with its shared policy without changing the job seam.
use crate::{Connection, Result, Timestamp};
use rusqlite::OptionalExtension;
use serde_json::Value;
pub fn allows(conn: &Connection, user_id: i64, now: Timestamp) -> Result<bool> {
    let (zone,presence,dnd,until,quiet,start,end,meeting_status,meeting_dnd,ooo_until,calendar_ooo,ooo_notify)=conn.query_row("SELECT time_zone,presence_setting,dnd_enabled,dnd_until,quiet_hours_enabled,quiet_hours_start_minute,quiet_hours_end_minute,meeting_status_enabled,meeting_dnd_enabled,ooo_until,ooo_calendar_enabled,ooo_notify_enabled FROM users WHERE id=?",[user_id],|r|Ok((r.get::<_,Option<String>>(0)?,r.get::<_,String>(1)?,r.get::<_,bool>(2)?,r.get::<_,Option<Timestamp>>(3)?,r.get::<_,bool>(4)?,r.get::<_,Option<i64>>(5)?,r.get::<_,Option<i64>>(6)?,r.get::<_,bool>(7)?,r.get::<_,bool>(8)?,r.get::<_,Option<Timestamp>>(9)?,r.get::<_,bool>(10)?,r.get::<_,bool>(11)?)))?;
    if (dnd && until.is_none_or(|until| until > now)) || presence == "dnd" {
        return Ok(false);
    }
    if quiet
        && let (Some(start), Some(end)) = (start, end)
        && start != end
    {
        let zone = crate::slash_commands::time_parser::known_zone(
            zone.as_deref().filter(|s| !s.is_empty()).unwrap_or("UTC"),
        )
        .ok_or_else(|| crate::Error::Other("Invalid reminder time zone".into()))?;
        let time = now.jiff().to_zoned(zone);
        let minute = i64::from(time.hour()) * 60 + i64::from(time.minute());
        if if start < end {
            minute >= start && minute < end
        } else {
            minute >= start || minute < end
        } {
            return Ok(false);
        }
    }
    if !ooo_notify && ooo_until.is_some_and(|until| until > now) {
        return Ok(false);
    }
    if !(meeting_status && meeting_dnd || calendar_ooo && !ooo_notify) {
        return Ok(true);
    }
    let intervals = conn
        .query_row(
            "SELECT busy_intervals,ooo_intervals FROM calendar_meeting_caches WHERE user_id=?",
            [user_id],
            |r| {
                Ok((
                    r.get::<_, Option<String>>(0)?,
                    r.get::<_, Option<String>>(1)?,
                ))
            },
        )
        .optional()?;
    let Some((busy, ooo)) = intervals else {
        return Ok(true);
    };
    if meeting_status && meeting_dnd && covering(busy.as_deref(), now)? {
        return Ok(false);
    }
    if calendar_ooo && !ooo_notify && covering(ooo.as_deref(), now)? {
        return Ok(false);
    }
    Ok(true)
}
fn covering(raw: Option<&str>, now: Timestamp) -> Result<bool> {
    let value: Value = serde_json::from_str(raw.unwrap_or("null"))
        .map_err(|e| crate::Error::Other(e.to_string()))?;
    Ok(value.as_array().is_some_and(|pairs| {
        pairs.iter().any(|pair| {
            let Some(pair) = pair.as_array() else {
                return false;
            };
            let parse = |v: Option<&Value>| {
                v.and_then(Value::as_str)
                    .and_then(|v| crate::slash_commands::time_parser::parse(v, "UTC", now))
            };
            match (parse(pair.first()), parse(pair.get(1))) {
                (Some(start), Some(end)) => start <= now && now < end,
                _ => false,
            }
        })
    }))
}
