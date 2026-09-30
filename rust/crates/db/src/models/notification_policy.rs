//! Notifications::Policy: a pure decision over preloaded status/membership data.
use super::user_status_settings::UserStatusSettings;
use crate::sql::{placeholders, query_all};
use crate::{Involvement, Result, ThreadInvolvement, Timestamp};
use rusqlite::Connection;
use std::collections::HashSet;

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

/// `Some(None)` represents an existing membership whose involvement is SQL NULL.
pub struct NotificationPolicy<'a> {
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
    fn invisible(&self) -> bool {
        self.room_involvement == Some(Some(Involvement::Invisible))
    }
    fn room_replies(&self) -> bool {
        matches!(
            self.room_involvement,
            Some(Some(Involvement::Everything | Involvement::Mentions))
        )
    }
    fn room_mentions(&self) -> bool {
        self.room_replies() || self.room_involvement == Some(Some(Involvement::Muted))
    }
    fn thread_mentions(&self) -> bool {
        matches!(
            self.thread_involvement,
            Some(ThreadInvolvement::Everything | ThreadInvolvement::Mentions)
        )
    }
    pub fn inbox_event_type(&self) -> Option<&'static str> {
        if !self.recipient.is_some_and(UserStatusSettings::active_human) || self.invisible() {
            return None;
        }
        match self.kind {
            NotificationKind::RoomMessage => {
                self.room_involvement?;
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
                self.room_involvement?;
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
            self.base_push() && (!recipient.quiet_for_push(self.now) || self.dnd_exception)
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
                self.room_involvement.is_some()
                    && !matches!(
                        self.room_involvement,
                        Some(Some(
                            Involvement::Invisible | Involvement::Nothing | Involvement::Muted
                        ))
                    )
            }
            RoomMessage => {
                self.room_involvement.is_some()
                    && !self.invisible()
                    && self.room_involvement != Some(Some(Involvement::Nothing))
                    && (self.room_involvement == Some(Some(Involvement::Everything))
                        || (self.mentioned && self.room_mentions())
                        || (self.reply_to_recipient && self.room_replies()))
            }
            ThreadMessage => {
                if self.room_involvement.is_none()
                    || self.invisible()
                    || self.room_involvement == Some(Some(Involvement::Nothing))
                    || self.thread_involvement.is_none()
                    || self.thread_involvement == Some(ThreadInvolvement::Nothing)
                {
                    return false;
                }
                if self.room_involvement == Some(Some(Involvement::Muted)) {
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
