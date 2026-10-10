//! User::StatusSettings readers and writers. Expired settings read as off without modifying rows.
//! Calendar caches are preloaded: these methods never contact Google or acquire a write lock.

use std::collections::HashMap;

use rusqlite::{Connection, Row};
use serde_json::Value;

use crate::sql::{placeholders, query_all};
use crate::{Result, Role, Status, Timestamp, User};

pub mod updates;
mod writes;
pub use writes::{clock_time_to_minutes, minutes_to_clock_time, replace_keyword_alerts};

#[derive(Debug, Clone)]
pub struct UserStatusSettings {
    pub user: User,
    pub notification_preferences: super::notification_policy::NotificationPreferences,
    pub presence_setting: String,
    pub custom_status_emoji: Option<String>,
    pub custom_status_text: Option<String>,
    pub custom_status_expires_at: Option<Timestamp>,
    pub dnd_enabled: bool,
    pub dnd_until: Option<Timestamp>,
    pub quiet_hours_enabled: bool,
    pub quiet_hours_start_minute: Option<i64>,
    pub quiet_hours_end_minute: Option<i64>,
    pub time_zone: Option<String>,
    pub time_zone_explicit: bool,
    pub theme: String,
    pub text_size: String,
    pub meeting_status_enabled: bool,
    pub meeting_dnd_enabled: bool,
    pub ooo_until: Option<Timestamp>,
    pub ooo_note: Option<String>,
    pub ooo_calendar_enabled: bool,
    pub ooo_notify_enabled: bool,
    pub ooo_broadcast: Option<bool>,
    pub meeting_cache: Option<MeetingCache>,
    /// Active Record dirty tracking uses the loaded record, not a new row read during save.
    original_attributes: Vec<(&'static str, rusqlite::types::Value)>,
}

#[derive(Debug, Clone)]
pub struct MeetingCache {
    pub id: i64,
    pub user_id: i64,
    pub busy_intervals: Value,
    pub ooo_intervals: Value,
    pub fetched_at: Option<Timestamp>,
    pub in_meeting_broadcast: Option<bool>,
}

impl MeetingCache {
    /// Calendar::MeetingCache#claim_broadcast!: the loaded object deliberately stays stale.
    pub fn claim_broadcast(&self, tx: &mut crate::Tx<'_>, active: bool) -> Result<bool> {
        Ok(tx.conn().execute(
            "UPDATE calendar_meeting_caches SET in_meeting_broadcast=?,updated_at=? WHERE id=? AND (in_meeting_broadcast IS NULL OR in_meeting_broadcast!=?)",
            rusqlite::params![active,tx.now(),self.id,active],
        )? == 1)
    }

    /// WS14's push-throttle seam. The winner must enqueue the follow-up on this same Tx.
    pub fn claim_refresh_followup(
        &self,
        tx: &mut crate::Tx<'_>,
        now: Timestamp,
        window: jiff::SignedDuration,
    ) -> Result<bool> {
        Ok(tx.conn().execute(
            "UPDATE calendar_meeting_caches SET refresh_pending_at=?,updated_at=? WHERE id=? AND (refresh_pending_at IS NULL OR refresh_pending_at<=?)",
            rusqlite::params![now,tx.now(),self.id,now.since(-window)],
        )? == 1)
    }

    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        let json = |name| -> rusqlite::Result<Value> {
            Ok(row
                .get::<_, Option<String>>(name)?
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or(Value::Null))
        };
        Ok(Self {
            id: row.get("id")?,
            user_id: row.get("user_id")?,
            busy_intervals: json("busy_intervals")?,
            ooo_intervals: json("ooo_intervals")?,
            fetched_at: row.get("fetched_at")?,
            in_meeting_broadcast: row.get("in_meeting_broadcast")?,
        })
    }

    /// MeetingCache#parse_pairs keeps malformed pairs out without sorting or clipping.
    pub fn parsed_pairs(
        pairs: &Value,
        now: Timestamp,
    ) -> impl Iterator<Item = (Timestamp, Timestamp)> + '_ {
        pairs
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(move |pair| {
                let pair = pair.as_array()?;
                let parse = |v: &Value| {
                    v.as_str()?
                        .parse::<jiff::Timestamp>()
                        .ok()
                        .map(Timestamp::from_jiff)
                        .or_else(|| {
                            crate::slash_commands::time_parser::parse(v.as_str()?, "UTC", now)
                        })
                };
                Some((parse(pair.first()?)?, parse(pair.get(1)?)?))
            })
    }
    fn covering_ends(pairs: &Value, now: Timestamp) -> impl Iterator<Item = Timestamp> + '_ {
        Self::parsed_pairs(pairs, now)
            .filter_map(move |(start, end)| (start <= now && now < end).then_some(end))
    }
    pub fn quiet_window_epochs(&self, now: Timestamp) -> Vec<(i64, i64)> {
        Self::parsed_pairs(&self.busy_intervals, now)
            .map(|(start, end)| (start.jiff().as_second(), end.jiff().as_second()))
            .collect()
    }
    pub fn ooo_window_epochs(&self, now: Timestamp) -> Vec<(i64, i64)> {
        Self::parsed_pairs(&self.ooo_intervals, now)
            .map(|(start, end)| (start.jiff().as_second(), end.jiff().as_second()))
            .collect()
    }

    pub fn in_meeting(&self, now: Timestamp) -> bool {
        Self::covering_ends(&self.busy_intervals, now)
            .next()
            .is_some()
    }
    pub fn in_ooo(&self, now: Timestamp) -> bool {
        Self::covering_ends(&self.ooo_intervals, now)
            .next()
            .is_some()
    }
    pub fn ooo_end_covering(&self, now: Timestamp) -> Option<Timestamp> {
        Self::covering_ends(&self.ooo_intervals, now).max()
    }
}

impl UserStatusSettings {
    /// Keep the loaded status attributes in step with User#deactivate's persisted OOO reset.
    pub fn deactivate(&mut self, tx: &mut crate::Tx<'_>) -> Result<()> {
        self.user.deactivate(tx)?;
        self.ooo_until = None;
        self.ooo_note = None;
        self.ooo_broadcast = None;
        self.original_attributes = self.attributes();
        Ok(())
    }

    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        let mut user = Self {
            user: User::from_row(row)?,
            notification_preferences: super::notification_policy::NotificationPreferences::parse(row.get::<_, Option<String>>("inbox_preferences")?.as_deref()),
            presence_setting: row.get("presence_setting")?,
            custom_status_emoji: row.get("custom_status_emoji")?,
            custom_status_text: row.get("custom_status_text")?,
            custom_status_expires_at: row.get("custom_status_expires_at")?,
            dnd_enabled: row.get("dnd_enabled")?,
            dnd_until: row.get("dnd_until")?,
            quiet_hours_enabled: row.get("quiet_hours_enabled")?,
            quiet_hours_start_minute: row.get("quiet_hours_start_minute")?,
            quiet_hours_end_minute: row.get("quiet_hours_end_minute")?,
            time_zone: row.get("time_zone")?,
            time_zone_explicit: row.get("time_zone_explicit")?,
            theme: row.get("theme")?,
            text_size: row.get("text_size")?,
            meeting_status_enabled: row.get("meeting_status_enabled")?,
            meeting_dnd_enabled: row.get("meeting_dnd_enabled")?,
            ooo_until: row.get("ooo_until")?,
            ooo_note: row.get("ooo_note")?,
            ooo_calendar_enabled: row.get("ooo_calendar_enabled")?,
            ooo_notify_enabled: row.get("ooo_notify_enabled")?,
            ooo_broadcast: row.get("ooo_broadcast")?,
            meeting_cache: None,
            original_attributes: Vec::new(),
        };
        user.notification_preferences.settings_revision = row.get("activity_revision")?;
        user.original_attributes = user.attributes();
        Ok(user)
    }

    /// Two queries for the batch, regardless of how many subscriptions each person has.
    pub fn for_ids(conn: &Connection, ids: &[i64]) -> Result<HashMap<i64, Self>> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let sql = format!(
            "SELECT * FROM users WHERE id IN ({})",
            placeholders(ids.len())
        );
        let mut users: HashMap<i64, Self> =
            query_all(conn, &sql, rusqlite::params_from_iter(ids), Self::from_row)?
                .into_iter()
                .map(|u| (u.user.id, u))
                .collect();
        let sql = format!(
            "SELECT * FROM calendar_meeting_caches WHERE user_id IN ({})",
            placeholders(ids.len())
        );
        for cache in query_all(
            conn,
            &sql,
            rusqlite::params_from_iter(ids),
            MeetingCache::from_row,
        )? {
            if let Some(user) = users.get_mut(&cache.user_id) {
                user.meeting_cache = Some(cache);
            }
        }
        Ok(users)
    }

    pub fn active_human(&self) -> bool {
        self.user.status == Status::Active && self.user.role != Role::Bot
    }
    pub fn zone(&self) -> jiff::tz::TimeZone {
        self.zone_or(&jiff::tz::TimeZone::UTC)
    }
    pub fn manual_dnd_active(&self, now: Timestamp) -> bool {
        self.dnd_enabled && self.dnd_until.is_none_or(|until| until > now)
    }
    /// Rails time_zone_or_default: a missing member zone falls back to Time.zone.
    pub fn zone_or(&self, request_zone: &jiff::tz::TimeZone) -> jiff::tz::TimeZone {
        self.time_zone
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .map(crate::slash_commands::time_parser::zone)
            .unwrap_or_else(|| request_zone.clone())
    }
    pub fn quiet_hours_active(&self, now: Timestamp) -> bool {
        let (Some(start), Some(end)) = (self.quiet_hours_start_minute, self.quiet_hours_end_minute)
        else {
            return false;
        };
        if !self.quiet_hours_enabled || start == end {
            return false;
        }
        let zoned = now.jiff().to_zoned(self.zone());
        let minute = i64::from(zoned.hour()) * 60 + i64::from(zoned.minute());
        if start < end {
            minute >= start && minute < end
        } else {
            minute >= start || minute < end
        }
    }
    pub fn dnd_active(&self, now: Timestamp) -> bool {
        self.manual_dnd_active(now)
            || self.presence_setting == "dnd"
            || self.quiet_hours_active(now)
    }
    pub fn custom_status_display(&self, now: Timestamp) -> Option<String> {
        if self
            .custom_status_expires_at
            .is_some_and(|until| until <= now)
        {
            return None;
        }
        let parts = [
            self.custom_status_emoji.as_deref(),
            self.custom_status_text.as_deref(),
        ]
        .into_iter()
        .flatten()
        .filter(|s| !s.trim().is_empty())
        .collect::<Vec<_>>();
        (!parts.is_empty()).then(|| parts.join(" "))
    }
    pub fn manual_ooo_active(&self, now: Timestamp) -> bool {
        self.ooo_until.is_some_and(|until| until > now)
    }
    pub fn calendar_ooo_active(&self, now: Timestamp) -> bool {
        self.ooo_calendar_enabled
            && self
                .meeting_cache
                .as_ref()
                .is_some_and(|cache| cache.in_ooo(now))
    }
    pub fn ooo_until_effective(&self, now: Timestamp) -> Option<Timestamp> {
        let manual = self.ooo_until.filter(|until| *until > now);
        let calendar = self
            .meeting_cache
            .as_ref()
            .filter(|_| self.ooo_calendar_enabled)
            .and_then(|cache| cache.ooo_end_covering(now));
        manual.into_iter().chain(calendar).max()
    }
    pub fn out_of_office(&self, now: Timestamp) -> bool {
        self.ooo_until_effective(now).is_some()
    }
    pub fn in_meeting(&self, now: Timestamp) -> bool {
        self.meeting_status_enabled
            && self
                .meeting_cache
                .as_ref()
                .is_some_and(|cache| cache.in_meeting(now))
    }
    pub fn ooo_status_visible(&self, now: Timestamp) -> bool {
        self.out_of_office(now) && self.presence_setting != "invisible"
    }
    pub fn ooo_until_date(&self, now: Timestamp) -> Option<String> {
        self.ooo_until_date_in_zone(now, &jiff::tz::TimeZone::UTC)
    }
    pub fn ooo_until_date_in_zone(
        &self,
        now: Timestamp,
        request_zone: &jiff::tz::TimeZone,
    ) -> Option<String> {
        self.ooo_until_effective(now).map(|until| {
            until
                .jiff()
                .to_zoned(self.zone_or(request_zone))
                .strftime("%B %d, %Y")
                .to_string()
        })
    }
    pub fn ooo_status_text(&self, now: Timestamp) -> Option<String> {
        self.ooo_status_text_in_zone(now, &jiff::tz::TimeZone::UTC)
    }
    pub fn ooo_status_text_in_zone(
        &self,
        now: Timestamp,
        request_zone: &jiff::tz::TimeZone,
    ) -> Option<String> {
        if !self.ooo_status_visible(now) {
            return None;
        }
        let mut text = format!(
            "🌴 Out of office until {}",
            self.ooo_until_date_in_zone(now, request_zone)?
        );
        if self.manual_ooo_active(now)
            && let Some(note) = self.ooo_note.as_deref().filter(|s| !s.trim().is_empty())
        {
            text.push_str(&format!(" — {note}"));
        }
        Some(text)
    }
    pub fn ooo_dnd_active(&self, now: Timestamp) -> bool {
        self.out_of_office(now) && !self.ooo_notify_enabled
    }
    pub fn meeting_status_visible(&self, now: Timestamp) -> bool {
        self.in_meeting(now)
            && !self.out_of_office(now)
            && self.custom_status_display(now).is_none()
            && !self.dnd_active(now)
            && self.presence_setting != "invisible"
    }
    pub fn meeting_dnd_active(&self, now: Timestamp) -> bool {
        self.meeting_dnd_enabled && self.in_meeting(now)
    }
    pub fn quiet_for_push(&self, now: Timestamp) -> bool {
        self.dnd_active(now) || self.meeting_dnd_active(now) || self.ooo_dnd_active(now)
    }
    pub fn status_text_display(&self, now: Timestamp) -> Option<String> {
        self.status_text_display_in_zone(now, &jiff::tz::TimeZone::UTC)
    }
    pub fn status_text_display_in_zone(
        &self,
        now: Timestamp,
        request_zone: &jiff::tz::TimeZone,
    ) -> Option<String> {
        self.ooo_status_text_in_zone(now, request_zone)
            .or_else(|| self.custom_status_display(now))
            .or_else(|| {
                self.meeting_status_visible(now)
                    .then(|| "📅 In a meeting".into())
            })
    }
    pub fn effective_presence(
        &self,
        lease_state: crate::models::workspace_presence_lease::Presence,
    ) -> crate::models::workspace_presence_lease::Presence {
        use crate::models::workspace_presence_lease::Presence;
        match self.presence_setting.as_str() {
            "invisible" => Presence::Offline,
            "dnd" if lease_state != Presence::Offline => Presence::Dnd,
            _ => lease_state,
        }
    }
}
