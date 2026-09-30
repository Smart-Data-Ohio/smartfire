//! Writes and virtual setters from app/models/user/status_settings.rb. Rendering stays in views.
use super::UserStatusSettings;
use crate::models::keyword_alert;
use crate::slash_commands::time_parser;
use crate::{Errors, KeywordAlert, Result, Timestamp, Tx};
use jiff::{SignedDuration, Span};
use rusqlite::types::Value;

pub fn clock_time_to_minutes(value: &str) -> Option<i64> {
    let parts: Vec<_> = value.split(':').collect();
    if !(2..=3).contains(&parts.len())
        || !(1..=2).contains(&parts[0].len())
        || parts[1].len() != 2
        || parts.get(2).is_some_and(|s| s.len() != 2)
        || parts.iter().any(|s| !s.bytes().all(|c| c.is_ascii_digit()))
    {
        return None;
    }
    let hour = parts[0].parse::<i64>().ok()?;
    let minute = parts[1].parse::<i64>().ok()?;
    (hour <= 23 && minute <= 59).then_some(hour * 60 + minute)
}

pub fn minutes_to_clock_time(value: Option<i64>) -> Option<String> {
    value.map(|m| format!("{:02}:{:02}", m / 60, m % 60))
}

fn sentence(messages: &[String]) -> String {
    match messages {
        [] => String::new(),
        [one] => one.clone(),
        [one, two] => format!("{one} and {two}"),
        many => format!(
            "{}, and {}",
            many[..many.len() - 1].join(", "),
            many.last().unwrap()
        ),
    }
}

/// Typed equivalent of User#replace_keyword_alerts after the caller has applied Ruby Array/to_s.
/// The nested savepoint lets callers rescue validation without committing a deleted old list.
pub fn replace_keyword_alerts(tx: &mut Tx<'_>, user_id: i64, lines: &[String]) -> Result<()> {
    let mut phrases = Vec::new();
    for phrase in lines
        .iter()
        .flat_map(|line| line.split('\n'))
        .map(keyword_alert::normalize)
        .filter(|s| !s.chars().all(char::is_whitespace))
    {
        if !phrases
            .iter()
            .any(|s: &String| s.to_lowercase() == phrase.to_lowercase())
        {
            phrases.push(phrase);
        }
        if phrases.len() > keyword_alert::MAX_PER_USER as usize {
            break;
        }
    }
    tx.savepoint(|tx| {
        tx.conn()
            .execute("DELETE FROM keyword_alerts WHERE user_id=?", [user_id])?;
        for phrase in phrases {
            KeywordAlert::create(tx, user_id, &phrase)?;
        }
        Ok(())
    })
    .map_err(|error| match error {
        crate::Error::RecordInvalid(errors) => {
            let mut user_errors = Errors::default();
            user_errors.add("base", sentence(&errors.full_messages()));
            crate::Error::RecordInvalid(user_errors)
        }
        other => other,
    })
}

impl UserStatusSettings {
    pub fn dnd_allows(&self, conn: &rusqlite::Connection, sender: Option<i64>) -> Result<bool> {
        Ok(
            crate::models::notification_policy::dnd_exceptions_for(conn, &[self.user.id], sender)?
                .contains(&self.user.id),
        )
    }

    pub fn notifications_muted(
        &self,
        conn: &rusqlite::Connection,
        sender: Option<i64>,
        now: Timestamp,
    ) -> Result<bool> {
        Ok(self.dnd_active(now) && !self.dnd_allows(conn, sender)?)
    }

    pub fn find(conn: &rusqlite::Connection, user_id: i64) -> Result<Self> {
        Self::for_ids(conn, &[user_id])?
            .remove(&user_id)
            .ok_or(crate::Error::RecordNotFound("User"))
    }

    /// custom_status_expires_in=: blank preserves the old expiry; the `never` preset clears it.
    pub fn set_custom_status_expires_in(&mut self, preset: &str, now: Timestamp) -> Result<()> {
        if preset.chars().all(char::is_whitespace) {
            return Ok(());
        }
        let zone = self.zone();
        self.custom_status_expires_at = match preset {
            "minutes_30" => Some(now.since(SignedDuration::from_mins(30))),
            "hour_1" => Some(now.since(SignedDuration::from_hours(1))),
            "hours_4" => Some(now.since(SignedDuration::from_hours(4))),
            "today" => time_parser::end_of_day(now, &zone),
            "week" => {
                let delta = 6 - i64::from(
                    now.jiff()
                        .to_zoned(zone.clone())
                        .date()
                        .weekday()
                        .to_monday_zero_offset(),
                );
                time_parser::add_days(now, delta, &zone)
                    .and_then(|day| time_parser::end_of_day(day, &zone))
            }
            "never" => None,
            _ => {
                return Err(crate::Error::Other(format!(
                    "Unknown custom status expiry: {preset}"
                )));
            }
        };
        Ok(())
    }

    /// OOO presets use fresh local dates, unlike the custom-status setter's period-preserving
    /// end-of-day. Custom values use TimeZone#parse, not slash-command relative-time grammar.
    pub fn ooo_preset_until(
        &self,
        preset: &str,
        custom: Option<&str>,
        now: Timestamp,
    ) -> Result<Option<Timestamp>> {
        let zone = self.zone();
        let date = now.jiff().to_zoned(zone.clone()).date();
        let days = match preset {
            "tomorrow" => 1,
            "week" => 7,
            "monday" => {
                let n = (7 - i64::from(date.weekday().to_monday_zero_offset())) % 7;
                if n == 0 { 7 } else { n }
            }
            "custom" => {
                return Ok(custom
                    .filter(|s| !s.chars().all(char::is_whitespace))
                    .and_then(|s| time_parser::fallback(s, &zone, now)));
            }
            _ => return Err(crate::Error::Other(format!("Unknown OOO preset: {preset}"))),
        };
        Ok(date
            .checked_add(Span::new().days(days))
            .ok()
            .and_then(|date| time_parser::date_end_of_day(date, &zone)))
    }

    /// The notification switch clears expired timers, preserves a live timer, and turning it
    /// off clears the timer. Evaluate after assigning the submitted switch, as Rails does.
    pub fn reconcile_dnd_timer(&mut self, now: Timestamp) {
        if !self.dnd_enabled || !self.manual_dnd_active(now) {
            self.dnd_until = None;
        }
    }

    /// User#save: validate before changing rows, preserve unchanged expired OOO ends, and
    /// touch updated_at only for a changed save. Callback orchestration belongs to controllers.
    pub fn save(&mut self, tx: &mut Tx<'_>) -> Result<()> {
        let now = tx.now();
        let previous = Self::find(tx.conn(), self.user.id)?;
        self.time_zone = self
            .time_zone
            .take()
            .filter(|s| !s.chars().all(char::is_whitespace));
        self.validate_settings(&previous, now).into_result()?;
        let before = previous.attributes();
        let changes: Vec<_> = self
            .attributes()
            .into_iter()
            .zip(before)
            .filter_map(|((key, value), (_, old))| (value != old).then_some((key, value)))
            .collect();
        if changes.is_empty() {
            return Ok(());
        }
        let mut columns = changes
            .iter()
            .map(|(key, _)| format!("{key}=?"))
            .collect::<Vec<_>>();
        columns.push("updated_at=?".into());
        let mut values = changes
            .into_iter()
            .map(|(_, value)| value)
            .collect::<Vec<_>>();
        values.push(Value::Text(now.to_db()));
        values.push(Value::Integer(self.user.id));
        tx.conn().execute(
            &format!("UPDATE users SET {} WHERE id=?", columns.join(",")),
            rusqlite::params_from_iter(values),
        )?;
        self.user.updated_at = now;
        Ok(())
    }

    pub fn save_with_keywords(&mut self, tx: &mut Tx<'_>, lines: Option<&[String]>) -> Result<()> {
        tx.savepoint(|tx| {
            if let Some(lines) = lines {
                replace_keyword_alerts(tx, self.user.id, lines)?;
            }
            self.save(tx)
        })
    }

    fn validate_settings(&self, previous: &Self, now: Timestamp) -> Errors {
        let mut errors = Errors::default();
        for (key, value, allowed) in [
            (
                "presence_setting",
                self.presence_setting.as_str(),
                ["auto", "dnd", "invisible"].as_slice(),
            ),
            (
                "theme",
                self.theme.as_str(),
                ["light", "dark", "system"].as_slice(),
            ),
            (
                "text_size",
                self.text_size.as_str(),
                ["smaller", "small", "default", "large", "larger"].as_slice(),
            ),
        ] {
            if !allowed.contains(&value) {
                errors.add(key, "is not included in the list");
            }
        }
        for (key, value, limit) in [
            (
                "custom_status_emoji",
                self.custom_status_emoji.as_deref(),
                8,
            ),
            (
                "custom_status_text",
                self.custom_status_text.as_deref(),
                100,
            ),
            ("ooo_note", self.ooo_note.as_deref(), 140),
        ] {
            if value.is_some_and(|s| s.chars().count() > limit) {
                errors.add(key, format!("is too long (maximum is {limit} characters)"));
            }
        }
        if self.ooo_until != previous.ooo_until && self.ooo_until.is_some_and(|end| end <= now) {
            errors.add("ooo_until", "must be in the future");
        }
        for (key, value) in [
            ("quiet_hours_start_minute", self.quiet_hours_start_minute),
            ("quiet_hours_end_minute", self.quiet_hours_end_minute),
        ] {
            if let Some(value) = value {
                if value < 0 {
                    errors.add(key, "must be greater than or equal to 0");
                }
                if value >= 1440 {
                    errors.add(key, "must be less than 1440");
                }
            }
        }
        if self
            .time_zone
            .as_deref()
            .is_some_and(|zone| !known_zone(zone))
        {
            errors.add("time_zone", "is not a valid time zone");
        }
        if self.quiet_hours_enabled
            && (self.quiet_hours_start_minute.is_none() || self.quiet_hours_end_minute.is_none())
        {
            errors.add(
                "quiet_hours_start",
                "needs a start and an end while quiet hours are on",
            );
        }
        errors
    }

    fn attributes(&self) -> Vec<(&'static str, Value)> {
        let text = |s: &Option<String>| s.clone().map(Value::Text).unwrap_or(Value::Null);
        let stamp = |s: Option<Timestamp>| s.map(|s| Value::Text(s.to_db())).unwrap_or(Value::Null);
        let number = |n: Option<i64>| n.map(Value::Integer).unwrap_or(Value::Null);
        vec![
            (
                "presence_setting",
                Value::Text(self.presence_setting.clone()),
            ),
            ("theme", Value::Text(self.theme.clone())),
            ("text_size", Value::Text(self.text_size.clone())),
            ("custom_status_emoji", text(&self.custom_status_emoji)),
            ("custom_status_text", text(&self.custom_status_text)),
            (
                "custom_status_expires_at",
                stamp(self.custom_status_expires_at),
            ),
            ("time_zone", text(&self.time_zone)),
            (
                "time_zone_explicit",
                Value::Integer(self.time_zone_explicit.into()),
            ),
            ("dnd_enabled", Value::Integer(self.dnd_enabled.into())),
            ("dnd_until", stamp(self.dnd_until)),
            (
                "quiet_hours_enabled",
                Value::Integer(self.quiet_hours_enabled.into()),
            ),
            (
                "quiet_hours_start_minute",
                number(self.quiet_hours_start_minute),
            ),
            (
                "quiet_hours_end_minute",
                number(self.quiet_hours_end_minute),
            ),
            (
                "meeting_status_enabled",
                Value::Integer(self.meeting_status_enabled.into()),
            ),
            (
                "meeting_dnd_enabled",
                Value::Integer(self.meeting_dnd_enabled.into()),
            ),
            ("ooo_until", stamp(self.ooo_until)),
            ("ooo_note", text(&self.ooo_note)),
            (
                "ooo_calendar_enabled",
                Value::Integer(self.ooo_calendar_enabled.into()),
            ),
            (
                "ooo_notify_enabled",
                Value::Integer(self.ooo_notify_enabled.into()),
            ),
        ]
    }
}

fn known_zone(name: &str) -> bool {
    let zones: serde_json::Value =
        serde_json::from_str(include_str!("../../tests/ws17_settings_zones.json"))
            .expect("pinned Rails zone table");
    zones["names"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v.as_str() == Some(name))
}
