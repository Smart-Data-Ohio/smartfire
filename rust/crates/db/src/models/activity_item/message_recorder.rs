//! ActivityItems::Recorder's message candidates and keyword writer. WS12 owns other sources.
use crate::models::keyword_alert::matching_user_ids;
use crate::rich_text::RichText;
use crate::sql::{placeholders, query_all};
use crate::{
    ActivityItem, Involvement, Message, NotificationKind, NotificationPolicy, Result,
    ThreadInvolvement, Timestamp, Tx, UserStatusSettings,
};
use rusqlite::{Connection, params};
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
    let rooms: HashMap<i64, Option<Involvement>> = query_all(
        conn,
        &format!(
            "SELECT user_id,involvement FROM memberships WHERE room_id=? AND user_id IN ({})",
            placeholders(ids.len())
        ),
        rusqlite::params_from_iter(values),
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?
    .into_iter()
    .collect();
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
    let mut room_member_ids: Vec<_> = rooms.into_keys().collect();
    room_member_ids.sort_unstable();
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
            let user_id = candidate.user_id;
            let event_type = candidate.event_type;
            if let Some(thread) = message
                .thread_id
                .filter(|_| event_type == "thread_activity")
            {
                let grouped: Option<i64> = crate::sql::query_one(
                    tx.conn(),
                    "SELECT id FROM activity_items WHERE user_id=? AND handled_at IS NULL AND event_type=? AND ((source_type='Message' AND source_id IN (SELECT id FROM messages WHERE thread_id=?)) OR (source_type='WorkThreadEvent' AND source_id IN (SELECT id FROM work_thread_events WHERE channel_thread_id=?))) ORDER BY updated_at DESC,id DESC LIMIT 1",
                    params![user_id, event_type, thread, thread],
                    |r| r.get(0),
                )?;
                if let Some(id) = grouped {
                    let before = Self::find(tx.conn(), id)?;
                    let now = tx.now();
                    if before.source_type == "Message"
                        && before.source_id == message.id
                        && before.read_at.is_none()
                        && before.updated_at == now
                    {
                        items.push(before);
                        continue;
                    }
                    // Grouped updates keep handled_at/type but repoint, become unread and touch.
                    tx.conn().execute("UPDATE activity_items SET source_type='Message',source_id=?,read_at=NULL,updated_at=? WHERE id=?",params![message.id,tx.now(),id])?;
                    Self::broadcast_change(tx, user_id, id)?;
                    items.push(Self::find(tx.conn(), id)?);
                    continue;
                }
            }
            let inserted=tx.conn().execute("INSERT INTO activity_items(user_id,source_type,source_id,event_type,created_at,updated_at) VALUES (?,'Message',?,?,?,?) ON CONFLICT(user_id,source_type,source_id) DO NOTHING",params![user_id,message.id,event_type,tx.now(),tx.now()])?;
            let item = Self::find_by_user_and_source(tx.conn(), user_id, "Message", message.id)?
                .ok_or(crate::Error::RecordNotFound("ActivityItem"))?;
            if inserted == 1 {
                Self::broadcast_change(tx, user_id, item.id)?;
            }
            items.push(item);
        }
        Ok(items)
    }
}
