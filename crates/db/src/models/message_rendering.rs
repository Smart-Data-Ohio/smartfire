//! Page-scoped reads for Message.with_rendering_details. No HTML or viewer state.

use crate::models::poll::{Poll, PollOption, PollVote};
use crate::{Boost, Message, Result, Room, Timestamp, User};
use rusqlite::{Connection, Row};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};

/// Rails MessagesController's body-free conditional-GET aggregates. Unpins need
/// their own digest; a maximum timestamp cannot detect removal of an older pin.
pub fn page_validators(
    conn: &Connection,
    messages: &[Message],
) -> Result<(Option<String>, String)> {
    let ids = messages.iter().map(|m| m.id).collect::<Vec<_>>();
    let stamp=rows(conn,"WITH page AS (SELECT * FROM messages WHERE id IN ($ids)) SELECT MAX(stamp) FROM (
        SELECT edited_at AS stamp FROM page
        UNION ALL SELECT m.updated_at FROM messages m JOIN page ON m.id=page.reply_to_message_id
        UNION ALL SELECT m.edited_at FROM messages m JOIN page ON m.id=page.reply_to_message_id
        UNION ALL SELECT c.updated_at FROM github_pull_request_references r JOIN github_pull_requests c ON c.id=r.github_pull_request_id JOIN page ON page.id=r.message_id
        UNION ALL SELECT c.updated_at FROM twitter_post_references r JOIN twitter_posts c ON c.id=r.twitter_post_id JOIN page ON page.id=r.message_id
        UNION ALL SELECT c.updated_at FROM link_embed_references r JOIN link_embeds c ON c.id=r.link_embed_id JOIN page ON page.id=r.message_id
        UNION ALL SELECT c.updated_at FROM event_references r JOIN events c ON c.id=r.event_id JOIN page ON page.id=r.message_id
        UNION ALL SELECT c.updated_at FROM polls c JOIN page ON page.id=c.message_id
        UNION ALL SELECT m.updated_at FROM messages m JOIN message_references r ON r.referenced_message_id=m.id JOIN page ON page.id=r.message_id
        UNION ALL SELECT m.edited_at FROM messages m JOIN message_references r ON r.referenced_message_id=m.id JOIN page ON page.id=r.message_id
        UNION ALL SELECT u.updated_at FROM users u JOIN page ON page.creator_id=u.id
    )",&ids,|r|r.get::<_,Option<Timestamp>>(0))?.into_iter().next().flatten()
        .map(|t|t.jiff().strftime("%Y-%m-%d %H:%M:%S%.6f").to_string());
    let pairs = rows(
        conn,
        "SELECT message_id,id FROM message_pins WHERE message_id IN ($ids) ORDER BY message_id,id",
        &ids,
        |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)),
    )?;
    let inspect = format!(
        "[{}]",
        pairs
            .iter()
            .map(|(message, pin)| format!("[{message}, {pin}]"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    Ok((stamp, format!("{:x}", Sha256::digest(inspect))))
}

#[derive(Clone)]
pub struct AgentStep {
    pub updated_at: Timestamp,
    pub name: String,
    pub status: String,
    pub duration_ms: Option<i64>,
    pub input_summary: Option<String>,
    pub output_summary: Option<String>,
}
#[derive(Clone)]
pub struct RenderingUser {
    pub user: User,
    pub uploaded_avatar: bool,
    pub icon_name: Option<String>,
}
#[derive(Clone, Default)]
pub struct RenderingRecords {
    pub cache: HashMap<i64, CacheDetails>,
    pub pr_thread_stamps: HashMap<i64, Timestamp>,
    pub sources: HashMap<i64, Message>,
    pub quotes: HashMap<i64, Vec<(i64, i64)>>,
    pub rooms: HashMap<i64, Room>,
    pub direct_names: HashMap<i64, Vec<String>>,
    pub direct_members: HashMap<i64, Vec<User>>,
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
#[derive(Clone, Default)]
pub struct CacheDetails {
    pub cards: Vec<Timestamp>,
    pub embeds: Vec<(i64, Option<Timestamp>)>,
    pub pins: Vec<Timestamp>,
    pub has_pull_requests: bool,
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
    let sql = sql.replace("$ids", "SELECT value FROM json_each(?)");
    Ok(conn
        .prepare(&sql)?
        .query_map([serde_json::json!(ids).to_string()], map)?
        .collect::<rusqlite::Result<Vec<_>>>()?)
}
impl RenderingRecords {
    /// WS8bm2 pin excerpts consume bodies only; one bind covers the complete list.
    pub fn load_plain_text(conn: &Connection, messages: &[Message]) -> Result<Self> {
        let ids: Vec<_> = messages.iter().map(|m| m.id).collect();
        let bodies = crate::sql::query_all(conn,
            "SELECT record_id,body FROM action_text_rich_texts WHERE record_type='Message' AND name='body' AND record_id IN (SELECT value FROM json_each(?)) ORDER BY id",
            [serde_json::json!(ids).to_string()], |r| Ok((r.get(0)?,r.get(1)?)))?
            .into_iter().rev().collect();
        Ok(Self { bodies, ..Default::default() })
    }
    pub fn load(conn: &Connection, messages: &[Message]) -> Result<Self> {
        Self::load_for(conn,messages,false)
    }
    /// MessagePayloadHelper needs body, reply, author, room and Drive facts only.
    pub fn load_payload(conn: &Connection, messages: &[Message]) -> Result<Self> {
        Self::load_for(conn,messages,true)
    }
    fn load_for(conn: &Connection, messages: &[Message], payload:bool) -> Result<Self> {
        let mut data = Self::default();
        if messages.is_empty() {
            return Ok(data);
        }
        let ids: Vec<_> = messages.iter().map(|m| m.id).collect();
        if !payload {
            // WS8bm2 root cache seam. These association reads are bounded by the page,
            // and cache invalidation must include public and private provider rows alike.
            for (message, stamp) in rows(conn, "WITH page AS (SELECT id FROM messages WHERE id IN ($ids))
                SELECT r.message_id,c.updated_at FROM github_pull_request_references r JOIN github_pull_requests c ON c.id=r.github_pull_request_id JOIN page ON page.id=r.message_id
                UNION ALL SELECT r.message_id,c.updated_at FROM fizzy_card_references r JOIN fizzy_cards c ON c.id=r.fizzy_card_id JOIN page ON page.id=r.message_id
                UNION ALL SELECT r.message_id,c.updated_at FROM twitter_post_references r JOIN twitter_posts c ON c.id=r.twitter_post_id JOIN page ON page.id=r.message_id
                UNION ALL SELECT r.message_id,c.updated_at FROM event_references r JOIN events c ON c.id=r.event_id JOIN page ON page.id=r.message_id",
                &ids, |r| Ok((r.get(0)?,r.get(1)?)))? {
                data.cache.entry(message).or_default().cards.push(stamp);
            }
            for (message, reference, stamp) in rows(
                conn,
                "SELECT r.message_id,r.id,c.updated_at FROM link_embed_references r LEFT JOIN link_embeds c ON c.id=r.link_embed_id WHERE r.message_id IN ($ids) ORDER BY r.id",
                &ids,
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )? {
                data.cache
                    .entry(message)
                    .or_default()
                    .embeds
                    .push((reference, stamp));
            }
            for message in rows(conn,
                "SELECT DISTINCT message_id FROM github_pull_request_references WHERE message_id IN ($ids)",
                &ids, |r| r.get(0))? {
                data.cache.entry(message).or_default().has_pull_requests = true;
            }
            for (message, reference, source) in rows(
                conn,
                "SELECT message_id,id,referenced_message_id FROM message_references WHERE message_id IN ($ids) ORDER BY id",
                &ids,
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )? {
                data.quotes
                    .entry(message)
                    .or_default()
                    .push((reference, source));
            }
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
        if !payload {
            data.pr_thread_stamps = rows(conn,
                "SELECT room_id,MAX(updated_at) FROM github_pull_request_threads WHERE room_id IN ($ids) GROUP BY room_id",
                &room_ids, |r| Ok((r.get(0)?,r.get(1)?)))?.into_iter().collect();
        }
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
        for (id, user) in rows(
            conn,
            "SELECT memberships.room_id,users.* FROM users JOIN memberships ON users.id=memberships.user_id WHERE memberships.room_id IN ($ids)",
            &directs,
            |r| Ok((r.get(0)?, User::from_row(r)?)),
        )? {
            data.direct_names.entry(id).or_default().push(user.display_name().to_owned());
            data.direct_members.entry(id).or_default().push(user);
        }
        if !payload {
            for boost in rows(
                conn,
                "SELECT * FROM boosts WHERE message_id IN ($ids) ORDER BY created_at",
                &ids,
                Boost::from_row,
            )? {
                data.boosts.entry(boost.message_id).or_default().push(boost);
            }
            for (message, stamp) in rows(
                conn,
                "SELECT message_id,updated_at FROM message_pins WHERE message_id IN ($ids)",
                &ids,
                |r| Ok((r.get(0)?, r.get(1)?)),
            )? {
                data.pinned.insert(message);
                data.cache.entry(message).or_default().pins.push(stamp);
            }
            data.reply_counts = rows(conn,"SELECT parent_message_id,messages_count FROM channel_threads WHERE parent_message_id IN ($ids)",&ids,|r|Ok((r.get(0)?,r.get(1)?)))?.into_iter().collect();
        }
        for (id, file) in rows(
            conn,
            "SELECT message_id,file_id FROM drive_attachments WHERE message_id IN ($ids) ORDER BY id",
            &ids,
            |r| Ok((r.get(0)?, r.get(1)?)),
        )? {
            data.drive_files.entry(id).or_default().push(file);
        }
        if !payload {
            for (id, step) in rows(
                conn,
                "SELECT * FROM agent_steps WHERE message_id IN ($ids) ORDER BY position,id",
                &ids,
                |r| {
                    Ok((
                        r.get("message_id")?,
                        AgentStep {
                            updated_at: r.get("updated_at")?,
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
                &format!("SELECT poll_votes.*,{} FROM poll_votes LEFT JOIN users ON users.id=poll_votes.user_id WHERE poll_id IN ($ids) ORDER BY poll_votes.id", User::projection("users", "voter_")),
                &poll_ids,
                |r| Ok((PollVote::from_row(r)?, r.get::<_, Option<i64>>("voter_id")?.map(|_| User::from_prefixed_row(r, "voter_").map(|user| user.display_name().to_owned())).transpose()?)),
            )? {
                data.votes
                    .entry(vote.poll_id)
                    .or_default()
                    .push((vote, name));
            }
        }
        Ok(data)
    }
    /// The associations belonging to one root, without reloading the page. Cache keys
    /// must not depend on unrelated search results (or their quoted sources).
    pub fn scoped(&self, message: &Message) -> Self {
        let mut data = self.clone();
        data.quotes.retain(|id, _| *id == message.id);
        let sources: HashSet<_> = message.reply_to_message_id.into_iter()
            .chain(data.quotes.values().flatten().map(|(_, id)| *id)).collect();
        data.sources.retain(|id, _| sources.contains(id));
        let bodies: HashSet<_> = data.body_ids(std::slice::from_ref(message)).into_iter().collect();
        data.bodies.retain(|id, _| bodies.contains(id));
        let rooms: HashSet<_> = std::iter::once(message.room_id)
            .chain(data.sources.values().map(|m| m.room_id)).collect();
        data.rooms.retain(|id, _| rooms.contains(id));
        data.direct_members.retain(|id, _| rooms.contains(id));
        data.boosts.retain(|id, _| *id == message.id);
        data.polls.retain(|id, _| *id == message.id);
        let polls: HashSet<_> = data.polls.values().map(|p| p.id).collect();
        data.options.retain(|id, _| polls.contains(id));
        data.votes.retain(|id, _| polls.contains(id));
        data
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
            .chain(self.votes.values().flatten().map(|(vote, _)| vote.user_id))
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

/// Request-scoped thread facts for Message.with_payload_details. All associations are
/// batched before serialization; permissions still use the shared ChannelThread policy.
#[derive(Default)]
pub struct ThreadRenderingRecords {
    pub threads: HashMap<i64, crate::ChannelThread>,
    pub by_parent: HashMap<i64, i64>,
    pub members: HashMap<i64, crate::ThreadMembership>,
    pub member_counts: HashMap<i64, i64>,
    pub message_counts: HashMap<i64, i64>,
    pub room_members: HashSet<(i64, i64)>,
    pub posting: HashSet<(i64, i64)>,
}
impl ThreadRenderingRecords {
    pub fn load(conn: &Connection, messages: &[Message], viewer: i64) -> Result<Self> {
        let mut data = Self::default();
        if messages.is_empty() {
            return Ok(data);
        }
        let message_ids = messages.iter().map(|m| m.id).collect::<Vec<_>>();
        let thread_ids = messages
            .iter()
            .filter_map(|m| m.thread_id)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let threads = rows(
            conn,
            "SELECT * FROM channel_threads WHERE parent_message_id IN ($ids)",
            &message_ids,
            crate::ChannelThread::from_row,
        )?;
        let contexts = rows(
            conn,
            "SELECT * FROM channel_threads WHERE id IN ($ids)",
            &thread_ids,
            crate::ChannelThread::from_row,
        )?;
        for thread in threads.into_iter().chain(contexts) {
            if let Some(parent) = thread.parent_message_id {
                data.by_parent.insert(parent, thread.id);
            }
            data.threads.insert(thread.id, thread);
        }
        let ids = data.threads.keys().copied().collect::<Vec<_>>();
        let rooms = data
            .threads
            .values()
            .map(|t| t.room_id)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        for member in rows(
            conn,
            "SELECT * FROM thread_memberships WHERE thread_id IN ($ids)",
            &ids,
            crate::ThreadMembership::from_row,
        )? {
            *data.member_counts.entry(member.thread_id).or_default() += 1;
            if member.user_id == viewer {
                data.members.insert(member.thread_id, member);
            }
        }
        data.message_counts = rows(
            conn,
            "SELECT thread_id,count(*) FROM messages WHERE thread_id IN ($ids) GROUP BY thread_id",
            &ids,
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?
        .into_iter()
        .collect();
        data.room_members = rows(
            conn,
            "SELECT room_id,user_id FROM memberships WHERE room_id IN ($ids)",
            &rooms,
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?
        .into_iter()
        .collect();
        let owners = data
            .threads
            .values()
            .filter_map(|t| t.work_owner_id)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        data.posting = super::agent_access::capabilities_for_users_in_rooms(
            conn,
            &owners,
            &rooms,
            "post_messages",
        )?;
        Ok(data)
    }
    pub fn user_ids(&self, viewer: i64) -> Vec<i64> {
        std::iter::once(viewer)
            .chain(self.threads.values().map(|t| t.creator_id))
            .chain(self.threads.values().filter_map(|t| t.work_owner_id))
            .collect()
    }
}
