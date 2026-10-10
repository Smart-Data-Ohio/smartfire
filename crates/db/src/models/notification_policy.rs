//! Notifications::Policy: a pure decision over preloaded status/membership data.
use super::user_status_settings::UserStatusSettings;
use crate::sql::{placeholders, query_all};
use crate::{Involvement, Result, ThreadInvolvement, Timestamp};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationKind {
    RoomMessage,
    ThreadMessage,
    Reminder,
    Huddle,
    HuddleJoin,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("Unknown notification kind: {0}")]
pub struct UnknownNotificationKind(pub String);

impl std::str::FromStr for NotificationKind {
    type Err = UnknownNotificationKind;
    fn from_str(kind: &str) -> std::result::Result<Self, Self::Err> {
        match kind {
            "room_message" => Ok(Self::RoomMessage),
            "thread_message" => Ok(Self::ThreadMessage),
            "reminder" => Ok(Self::Reminder),
            "huddle" => Ok(Self::Huddle),
            "huddle_join" => Ok(Self::HuddleJoin),
            other => Err(UnknownNotificationKind(other.into())),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationLevel {
    #[default]
    Everything,
    Mentions,
    Nothing,
}

impl NotificationLevel {
    pub fn involvement(self) -> Involvement {
        match self {
            Self::Everything => Involvement::Everything,
            Self::Mentions => Involvement::Mentions,
            Self::Nothing => Involvement::Nothing,
        }
    }
}

/// Existing memberships remain overrides until the person chooses to inherit.
/// A null mute deadline means muted until the person turns it back on.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct NotificationPreferences {
    pub default_notification_level: NotificationLevel,
    pub room_notification_levels: BTreeMap<i64, Option<NotificationLevel>>,
    pub room_mute_until: BTreeMap<i64, Option<jiff::Timestamp>>,
    #[serde(flatten)]
    pub legacy: BTreeMap<String, serde_json::Value>,
}

impl NotificationPreferences {
    pub fn load(conn: &Connection, user_id: i64) -> Result<Self> {
        let raw: Option<String> = conn.query_row(
            "SELECT inbox_preferences FROM users WHERE id=?",
            [user_id],
            |r| r.get(0),
        )?;
        Ok(Self::parse(raw.as_deref()))
    }

    pub fn parse(raw: Option<&str>) -> Self {
        raw.and_then(|raw| serde_json::from_str(raw).ok())
            .unwrap_or_default()
    }

    pub fn muted(&self, room_id: i64, now: Timestamp) -> bool {
        self.room_mute_until
            .get(&room_id)
            .is_some_and(|until| until.is_none_or(|until| until > now.jiff()))
    }

    pub fn involvement(&self, room_id: i64, stored: Option<Involvement>) -> Option<Involvement> {
        self.room_notification_levels
            .get(&room_id)
            .map(|level| {
                level
                    .unwrap_or(self.default_notification_level)
                    .involvement()
            })
            .or(stored)
    }
}

pub const NOTIFICATION_PREFERENCE_KEYS: &[&str] = &[
    "default_notification_level",
    "room_notification_levels",
    "room_mute_until",
];

pub fn valid_preference(key: &str, value: &serde_json::Value) -> bool {
    match key {
        "default_notification_level" => {
            serde_json::from_value::<NotificationLevel>(value.clone()).is_ok()
        }
        "room_notification_levels" => {
            serde_json::from_value::<BTreeMap<i64, Option<NotificationLevel>>>(value.clone())
                .is_ok_and(|levels| levels.keys().all(|id| *id > 0))
        }
        "room_mute_until" => {
            serde_json::from_value::<BTreeMap<i64, Option<jiff::Timestamp>>>(value.clone())
                .is_ok_and(|mutes| mutes.keys().all(|id| *id > 0))
        }
        _ => false,
    }
}

/// These expressions read the same JSON policy in the aggregate badge queries.
pub fn involvement_sql(membership: &str, preferences: &str) -> String {
    let path = format!("'$.room_notification_levels.\"' || {membership}.room_id || '\"'");
    format!(
        "CASE json_type({preferences}, {path}) WHEN 'null' THEN COALESCE(json_extract({preferences}, '$.default_notification_level'), 'everything') WHEN 'text' THEN json_extract({preferences}, {path}) ELSE {membership}.involvement END"
    )
}

pub fn unmuted_sql(room: &str, preferences: &str, now: &str) -> String {
    let path = format!("'$.room_mute_until.\"' || {room} || '\"'");
    format!(
        "(COALESCE(json_type({preferences}, {path}), '') != 'null' AND (json_type({preferences}, {path}) IS NULL OR julianday(json_extract({preferences}, {path})) <= julianday({now})))"
    )
}

/// `Some(None)` represents an existing membership whose involvement is SQL NULL.
pub struct NotificationPolicy<'a> {
    pub room_id: Option<i64>,
    pub recipient: Option<&'a UserStatusSettings>,
    pub kind: NotificationKind,
    pub room_involvement: Option<Option<Involvement>>,
    pub thread_involvement: Option<ThreadInvolvement>,
    pub mentioned: bool,
    pub reply_to_recipient: bool,
    pub keyword_matched: bool,
    /// The actual sender's DndAllowedUser exception; no exception for a senderless reminder.
    pub dnd_exception: bool,
    pub now: Timestamp,
}

impl NotificationPolicy<'_> {
    fn room_involvement(&self) -> Option<Option<Involvement>> {
        self.room_involvement
            .map(|stored| match (self.recipient, self.room_id) {
                (Some(user), Some(room)) => user.notification_preferences.involvement(room, stored),
                _ => stored,
            })
    }
    fn muted(&self) -> bool {
        self.recipient
            .zip(self.room_id)
            .is_some_and(|(user, room)| user.notification_preferences.muted(room, self.now))
    }

    fn invisible(&self) -> bool {
        self.room_involvement() == Some(Some(Involvement::Invisible))
    }
    fn room_replies(&self) -> bool {
        matches!(
            self.room_involvement(),
            Some(Some(Involvement::Everything | Involvement::Mentions))
        )
    }
    fn room_mentions(&self) -> bool {
        self.room_replies() || self.room_involvement() == Some(Some(Involvement::Muted))
    }
    fn thread_mentions(&self) -> bool {
        matches!(
            self.thread_involvement,
            Some(ThreadInvolvement::Everything | ThreadInvolvement::Mentions)
        )
    }
    pub fn inbox_event_type(&self) -> Option<&'static str> {
        if self
            .recipient
            .zip(self.room_id)
            .is_some_and(|(user, room)| {
                user.notification_preferences
                    .room_notification_levels
                    .contains_key(&room)
                    && user.notification_preferences.involvement(room, None)
                        == Some(Involvement::Nothing)
            })
        {
            return None;
        }
        if !self.recipient.is_some_and(UserStatusSettings::active_human)
            || self.invisible()
            || self.muted()
        {
            return None;
        }
        match self.kind {
            NotificationKind::RoomMessage => {
                self.room_involvement()?;
                if self.mentioned {
                    Some("mention")
                } else if self.reply_to_recipient && self.room_replies() {
                    Some("reply")
                } else if self.keyword_matched {
                    Some("keyword_alert")
                } else {
                    None
                }
            }
            NotificationKind::ThreadMessage => {
                if self.thread_involvement? == ThreadInvolvement::Nothing {
                    return None;
                }
                self.room_involvement()?;
                if self.mentioned && self.thread_mentions() {
                    Some("mention")
                } else if self.thread_involvement == Some(ThreadInvolvement::Everything)
                    && self.room_replies()
                {
                    Some(if self.reply_to_recipient {
                        "reply"
                    } else {
                        "thread_activity"
                    })
                } else if self.keyword_matched {
                    Some("keyword_alert")
                } else {
                    None
                }
            }
            _ => None,
        }
    }
    pub fn push(&self) -> bool {
        self.recipient.is_some_and(|recipient| {
            !self.muted()
                && self.base_push()
                && (!recipient.quiet_for_push(self.now) || self.dnd_exception)
        })
    }
    pub fn sound(&self) -> bool {
        self.push()
    }
    fn base_push(&self) -> bool {
        use NotificationKind::*;
        match self.kind {
            Reminder | Huddle => true,
            HuddleJoin => {
                self.room_involvement().is_some()
                    && !matches!(
                        self.room_involvement(),
                        Some(Some(
                            Involvement::Invisible | Involvement::Nothing | Involvement::Muted
                        ))
                    )
            }
            RoomMessage => {
                self.room_involvement().is_some()
                    && !self.invisible()
                    && self.room_involvement() != Some(Some(Involvement::Nothing))
                    && (self.room_involvement() == Some(Some(Involvement::Everything))
                        || (self.mentioned && self.room_mentions())
                        || (self.reply_to_recipient && self.room_replies()))
            }
            ThreadMessage => {
                if self.room_involvement().is_none()
                    || self.invisible()
                    || self.room_involvement() == Some(Some(Involvement::Nothing))
                    || self.thread_involvement.is_none()
                    || self.thread_involvement == Some(ThreadInvolvement::Nothing)
                {
                    return false;
                }
                if self.room_involvement() == Some(Some(Involvement::Muted)) {
                    return self.mentioned && self.thread_mentions();
                }
                self.thread_involvement == Some(ThreadInvolvement::Everything)
                    || (self.mentioned && self.thread_mentions())
            }
        }
    }
}

/// One exception lookup for a delivery batch. Stars alone grant no exception in Rails.
pub fn dnd_exceptions_for(
    conn: &Connection,
    ids: &[i64],
    sender_id: Option<i64>,
) -> Result<HashSet<i64>> {
    let Some(sender_id) = sender_id.filter(|_| !ids.is_empty()) else {
        return Ok(HashSet::new());
    };
    let sql = format!(
        "SELECT user_id FROM dnd_allowed_users WHERE allowed_user_id=? AND user_id IN ({})",
        placeholders(ids.len())
    );
    let values: Vec<i64> = std::iter::once(sender_id)
        .chain(ids.iter().copied())
        .collect();
    Ok(
        query_all(conn, &sql, rusqlite::params_from_iter(values), |row| {
            row.get(0)
        })?
        .into_iter()
        .collect(),
    )
}
