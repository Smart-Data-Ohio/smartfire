//! ActivityItems::Recorder's message candidates and keyword writer. WS12 owns other sources.
use crate::models::keyword_alert::matching_user_ids;
use crate::rich_text::RichText;
use crate::sql::{placeholders, query_all};
use crate::{
    ActivityItem, Involvement, Message, NotificationKind, NotificationPolicy, Result,
    ThreadInvolvement, Timestamp, Tx, UserStatusSettings,
};
use rusqlite::Connection;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageCandidate {
    pub user_id: i64,
    pub event_type: &'static str,
}
#[derive(Debug, Default)]
pub struct MessageCandidates {
    pub recipients: Vec<MessageCandidate>,
    /// Memberships are loaded only for root candidates or the thread's members.
    pub room_member_ids: Vec<i64>,
}

pub fn candidates(
    conn: &Connection,
    rich_text: &dyn RichText,
    message: &Message,
    now: Timestamp,
) -> Result<MessageCandidates> {
    let mentioned_ids: Vec<i64> = message
        .mentionees(conn, rich_text)?
        .iter()
        .map(|u| u.id)
        .collect();
    let mentioned: HashSet<i64> = mentioned_ids.iter().copied().collect();
    let reply_author = message
        .reply_to_message_id
        .map(|id| Message::find_by_id(conn, id))
        .transpose()?
        .flatten()
        .map(|m| m.creator_id);
    let text = message.plain_text_body(conn, rich_text)?;
    let threads: Vec<(i64, ThreadInvolvement)> = match message.thread_id {
        Some(id) => query_all(
            conn,
            "SELECT user_id,involvement FROM thread_memberships WHERE thread_id=?",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?,
        None => Vec::new(),
    };
    let mut ids: Vec<i64> = if message.thread_id.is_some() {
        threads.iter().map(|(id, _)| *id).collect()
    } else {
        mentioned_ids
    };
    let mut matched = Vec::new();
    if message.thread_id.is_none() {
        if let Some(id) = reply_author
            && !ids.contains(&id)
        {
            ids.push(id);
        }
        if !text.chars().all(char::is_whitespace) {
            let rows: Vec<(i64, String)> = query_all(
                conn,
                "SELECT user_id,phrase FROM keyword_alerts WHERE user_id IN (SELECT user_id FROM memberships WHERE room_id=?)",
                [message.room_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?;
            matched = match_rows(rows, &text)?;
            for id in &matched {
                if !ids.contains(id) {
                    ids.push(*id);
                }
            }
        }
    }
    if ids.is_empty() {
        return Ok(MessageCandidates::default());
    }
    let mut values = vec![message.room_id];
    values.extend(&ids);
    let room_rows: Vec<(i64, Option<Involvement>)> = query_all(
        conn,
        &format!(
            "SELECT user_id,involvement FROM memberships WHERE room_id=? AND user_id IN ({})",
            placeholders(ids.len())
        ),
        rusqlite::params_from_iter(values),
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let rooms: HashMap<_, _> = room_rows.iter().copied().collect();
    let settings = UserStatusSettings::for_ids(conn, &ids)?;
    if message.thread_id.is_some() && !text.chars().all(char::is_whitespace) {
        let eligible: Vec<_> = threads
            .iter()
            .filter(|(id, involvement)| {
                *involvement != ThreadInvolvement::Nothing
                    && rooms
                        .get(id)
                        .is_some_and(|r| *r != Some(Involvement::Invisible))
                    && settings
                        .get(id)
                        .is_some_and(UserStatusSettings::active_human)
            })
            .map(|(id, _)| *id)
            .collect();
        if !eligible.is_empty() {
            let rows = query_all(
                conn,
                &format!(
                    "SELECT user_id,phrase FROM keyword_alerts WHERE user_id IN ({})",
                    placeholders(eligible.len())
                ),
                rusqlite::params_from_iter(eligible),
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?;
            matched = match_rows(rows, &text)?;
        }
    }
    let thread_map: HashMap<_, _> = threads.into_iter().collect();
    let mut recipients = Vec::new();
    for id in ids {
        if id == message.creator_id {
            continue;
        }
        let policy = NotificationPolicy {
            recipient: settings.get(&id),
            kind: if message.thread_id.is_some() {
                NotificationKind::ThreadMessage
            } else {
                NotificationKind::RoomMessage
            },
            room_involvement: rooms.get(&id).copied(),
            thread_involvement: thread_map.get(&id).copied(),
            mentioned: mentioned.contains(&id),
            reply_to_recipient: reply_author == Some(id) && message.reply_notify_author,
            keyword_matched: matched.contains(&id),
            dnd_exception: false,
            now,
        };
        if let Some(event_type) = policy.inbox_event_type() {
            recipients.push(MessageCandidate {
                user_id: id,
                event_type,
            });
        }
    }
    let room_member_ids = room_rows.into_iter().map(|row| row.0).collect();
    Ok(MessageCandidates {
        recipients,
        room_member_ids,
    })
}
fn match_rows(rows: Vec<(i64, String)>, text: &str) -> Result<Vec<i64>> {
    let mut phrases: Vec<(i64, Vec<String>)> = Vec::new();
    for (id, phrase) in rows {
        if let Some((_, list)) = phrases.iter_mut().find(|(user, _)| *user == id) {
            list.push(phrase);
        } else {
            phrases.push((id, vec![phrase]));
        }
    }
    matching_user_ids(&phrases, text)
}

impl ActivityItem {
    /// Persisted-message seam for WS11 finalize and WS12's recorder. The source caller gates
    /// streaming/importing/system notes; inbox policy deliberately does not apply push quietness.
    pub fn record_message(tx: &mut Tx<'_>, message: &Message) -> Result<Vec<Self>> {
        if Message::find_by_id(tx.conn(), message.id)?.is_none() {
            return Ok(Vec::new());
        }
        let candidates = candidates(tx.conn(), tx.rich_text(), message, tx.now())?;
        let mut items = Vec::new();
        for candidate in candidates.recipients {
            if let Some(item) = Self::record(
                tx,
                candidate.user_id,
                super::ActivitySource::Message(message.id),
                candidate.event_type,
                true,
            )? {
                items.push(item);
            }
        }
        Ok(items)
    }
}
