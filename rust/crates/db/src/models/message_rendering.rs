//! Page-scoped reads for Message.with_rendering_details. No HTML or viewer state.
use crate::models::poll::{Poll, PollOption, PollVote};
use crate::{Boost, Message, Result, Room, User};
use rusqlite::{Connection, Row, params_from_iter};
use std::collections::{HashMap, HashSet};

#[derive(Clone)]
pub struct AgentStep {
    pub name: String,
    pub status: String,
    pub duration_ms: Option<i64>,
    pub input_summary: Option<String>,
    pub output_summary: Option<String>,
}
pub struct RenderingUser {
    pub user: User,
    pub uploaded_avatar: bool,
    pub icon_name: Option<String>,
}
#[derive(Default)]
pub struct RenderingRecords {
    pub sources: HashMap<i64, Message>,
    pub quotes: HashMap<i64, Vec<(i64, i64)>>,
    pub rooms: HashMap<i64, Room>,
    pub direct_names: HashMap<i64, Vec<String>>,
    pub room_icons: HashMap<i64, Option<String>>,
    pub bodies: HashMap<i64, Option<String>>,
    pub boosts: HashMap<i64, Vec<Boost>>,
    pub pinned: HashSet<i64>,
    pub reply_counts: HashMap<i64, u64>,
    pub drive_files: HashMap<i64, Vec<String>>,
    pub steps: HashMap<i64, Vec<AgentStep>>,
    pub polls: HashMap<i64, Poll>,
    pub options: HashMap<i64, Vec<PollOption>>,
    pub votes: HashMap<i64, Vec<(PollVote, Option<String>)>>,
}
/// IDs always originate in the authorized, bounded search window. Empty sets never load a table.
fn rows<T>(
    conn: &Connection,
    sql: &str,
    ids: &[i64],
    map: impl FnMut(&Row<'_>) -> rusqlite::Result<T>,
) -> Result<Vec<T>> {
    if ids.is_empty() {
        return Ok(vec![]);
    }
    let sql = sql.replace("$ids", &vec!["?"; ids.len()].join(","));
    Ok(conn
        .prepare(&sql)?
        .query_map(params_from_iter(ids), map)?
        .collect::<rusqlite::Result<Vec<_>>>()?)
}
impl RenderingRecords {
    pub fn load(conn: &Connection, messages: &[Message]) -> Result<Self> {
        let mut data = Self::default();
        if messages.is_empty() {
            return Ok(data);
        }
        let ids: Vec<_> = messages.iter().map(|m| m.id).collect();
        for (message, reference, source) in rows(conn,
            "SELECT message_id,id,referenced_message_id FROM message_references WHERE message_id IN ($ids) ORDER BY id",
            &ids, |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))? {
            data.quotes.entry(message).or_default().push((reference, source));
        }
        let reply_ids: Vec<_> = messages
            .iter()
            .filter_map(|m| m.reply_to_message_id)
            .chain(data.quotes.values().flatten().map(|(_, source)| *source))
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        data.sources = rows(
            conn,
            "SELECT * FROM messages WHERE id IN ($ids)",
            &reply_ids,
            Message::from_row,
        )?
        .into_iter()
        .map(|m| (m.id, m))
        .collect();
        let body_ids: Vec<_> = ids
            .iter()
            .copied()
            .chain(data.sources.keys().copied())
            .collect();
        data.bodies = rows(conn, "SELECT record_id,body FROM action_text_rich_texts WHERE record_type='Message' AND name='body' AND record_id IN ($ids) ORDER BY id", &body_ids, |r| Ok((r.get(0)?,r.get(1)?)))?.into_iter().rev().collect();
        let room_ids: Vec<_> = messages
            .iter()
            .chain(data.sources.values())
            .map(|m| m.room_id)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        let rooms = rows(
            conn,
            "SELECT * FROM rooms WHERE id IN ($ids)",
            &room_ids,
            |r| Ok((Room::from_row(r)?, r.get::<_, Option<String>>("icon_name")?)),
        )?;
        for (room, icon) in rooms {
            data.room_icons.insert(room.id, icon);
            data.rooms.insert(room.id, room);
        }
        let directs: Vec<_> = data
            .rooms
            .values()
            .filter(|r| r.direct())
            .map(|r| r.id)
            .collect();
        for (id, name) in rows(
            conn,
            "SELECT memberships.room_id,users.name FROM users JOIN memberships ON users.id=memberships.user_id WHERE memberships.room_id IN ($ids)",
            &directs,
            |r| Ok((r.get(0)?, r.get(1)?)),
        )? {
            data.direct_names.entry(id).or_default().push(name);
        }
        for boost in rows(
            conn,
            "SELECT * FROM boosts WHERE message_id IN ($ids) ORDER BY created_at",
            &ids,
            Boost::from_row,
        )? {
            data.boosts.entry(boost.message_id).or_default().push(boost);
        }
        data.pinned = rows(
            conn,
            "SELECT message_id FROM message_pins WHERE message_id IN ($ids)",
            &ids,
            |r| r.get(0),
        )?
        .into_iter()
        .collect();
        data.reply_counts = rows(conn,"SELECT parent_message_id,messages_count FROM channel_threads WHERE parent_message_id IN ($ids)",&ids,|r|Ok((r.get(0)?,r.get(1)?)))?.into_iter().collect();
        for (id, file) in rows(
            conn,
            "SELECT message_id,file_id FROM drive_attachments WHERE message_id IN ($ids) ORDER BY id",
            &ids,
            |r| Ok((r.get(0)?, r.get(1)?)),
        )? {
            data.drive_files.entry(id).or_default().push(file);
        }
        for (id, step) in rows(
            conn,
            "SELECT * FROM agent_steps WHERE message_id IN ($ids) ORDER BY position,id",
            &ids,
            |r| {
                Ok((
                    r.get("message_id")?,
                    AgentStep {
                        name: r.get("name")?,
                        status: r.get("status")?,
                        duration_ms: r.get("duration_ms")?,
                        input_summary: r.get("input_summary")?,
                        output_summary: r.get("output_summary")?,
                    },
                ))
            },
        )? {
            data.steps.entry(id).or_default().push(step);
        }
        data.polls = rows(
            conn,
            "SELECT * FROM polls WHERE message_id IN ($ids)",
            &ids,
            Poll::from_row,
        )?
        .into_iter()
        .map(|p| (p.message_id, p))
        .collect();
        let poll_ids: Vec<_> = data.polls.values().map(|p| p.id).collect();
        for option in rows(
            conn,
            "SELECT * FROM poll_options WHERE poll_id IN ($ids) ORDER BY position,id",
            &poll_ids,
            PollOption::from_row,
        )? {
            data.options.entry(option.poll_id).or_default().push(option);
        }
        for (vote, name) in rows(
            conn,
            "SELECT poll_votes.*,users.name AS user_name FROM poll_votes LEFT JOIN users ON users.id=poll_votes.user_id WHERE poll_id IN ($ids) ORDER BY poll_votes.id",
            &poll_ids,
            |r| Ok((PollVote::from_row(r)?, r.get("user_name")?)),
        )? {
            data.votes
                .entry(vote.poll_id)
                .or_default()
                .push((vote, name));
        }
        Ok(data)
    }
    pub fn users(
        &self,
        conn: &Connection,
        messages: &[Message],
        mentions: &[i64],
    ) -> Result<HashMap<i64, RenderingUser>> {
        let ids: Vec<_> = messages
            .iter()
            .chain(self.sources.values())
            .map(|m| m.creator_id)
            .chain(self.boosts.values().flatten().map(|b| b.booster_id))
            .chain(mentions.iter().copied())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        Ok(rows(conn,"SELECT users.*,EXISTS(SELECT 1 FROM active_storage_attachments a WHERE a.record_type='User' AND a.record_id=users.id AND a.name='avatar') AS uploaded_avatar FROM users WHERE users.id IN ($ids)",&ids,|r|Ok(RenderingUser {user:User::from_row(r)?, uploaded_avatar:r.get("uploaded_avatar")?,icon_name:r.get("icon_name")?}))?.into_iter().map(|u|(u.user.id,u)).collect())
    }
    pub fn body_ids(&self, messages: &[Message]) -> Vec<i64> {
        messages
            .iter()
            .map(|m| m.id)
            .chain(self.sources.keys().copied())
            .collect()
    }
}
