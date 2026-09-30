//! Agents::Streaming and Message::StreamTrailingBroadcastJob's domain half.
use super::{
    agent_access,
    agent_posting::{self, PostResult, PostingCheck},
    agent_service::{ServiceResult, invalid},
};
use crate::sql::{exists, query_all};
use crate::{
    Agent, ChannelThread, Error, Message, MessageChanges, NewMessage, Result, Timestamp, Tx,
};
use jiff::SignedDuration;
use rusqlite::params;
use serde::{Deserialize, Serialize};

pub const BROADCAST_INTERVAL: SignedDuration = SignedDuration::from_millis(250);
pub const FINALIZE_AFTER: SignedDuration = SignedDuration::from_mins(10);
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamTrailingBroadcastJob {
    pub message_id: i64,
    pub last_broadcast_at: String,
}
impl crate::Job for StreamTrailingBroadcastJob {
    const CLASS: &'static str = "Message::StreamTrailingBroadcastJob";
}
pub fn stamp(time: Timestamp) -> String {
    format!(
        "{}.{:06}Z",
        time.jiff().strftime("%Y-%m-%dT%H:%M:%S"),
        time.subsec_microsecond()
    )
}

pub fn start(tx: &mut Tx<'_>, agent_id: i64, mut a: NewMessage) -> Result<PostResult> {
    let thread = if let Some(id) = a.thread_id {
        let Some(thread) =
            ChannelThread::find_by_id(tx.conn(), id)?.filter(|t| t.room_id == a.room_id)
        else {
            return Ok(PostResult::Denied(ServiceResult::fail(
                "Thread not found",
                404,
            )));
        };
        if thread.locked_at.is_some() {
            return Ok(PostResult::Denied(ServiceResult::fail(
                "This thread is locked",
                422,
            )));
        }
        Some(thread)
    } else {
        if let Some(id) = a.reply_to_message_id
            && Message::find_by_id(tx.conn(), id)?
                .filter(|m| m.room_id == a.room_id && m.thread_id.is_none())
                .is_none()
        {
            return Err(Error::RecordNotFound("Message"));
        }
        None
    };
    match agent_posting::prepare(tx, agent_id, a.room_id, a.client_message_id.as_deref())? {
        PostingCheck::Replay(m) => return Ok(PostResult::Posted(m)),
        PostingCheck::Budget(p) => return Ok(PostResult::Denied(ServiceResult::budget(p))),
        PostingCheck::Allowed => {}
    }
    a.creator_id =
        tx.conn()
            .query_row("SELECT user_id FROM agents WHERE id=?", [agent_id], |r| {
                r.get(0)
            })?;
    a.streaming = true;
    a.drive_file_ids.clear();
    let message = match tx.savepoint(|tx| {
        if let Some(mut thread) = thread {
            thread.post_message(tx, a.creator_id, a)
        } else {
            Message::create(tx, a)
        }
    }) {
        Ok(m) => m,
        Err(Error::RecordInvalid(e)) => return Ok(PostResult::Denied(invalid(e))),
        Err(e) => return Err(e),
    };
    agent_posting::broadcast_stream_start(tx, &message)?;
    Ok(PostResult::Posted(Box::new(message)))
}
fn find_own(tx: &Tx<'_>, agent_id: i64, id: i64, streaming_only: bool) -> Result<PostResult> {
    let message = Message::find_by_id(tx.conn(), id)?;
    let agent = Agent::find(tx.conn(), agent_id)?;
    let Some(message) =
        message.filter(|m| agent.as_ref().is_some_and(|a| a.user_id == m.creator_id))
    else {
        return Ok(PostResult::Denied(ServiceResult::fail(
            "Message not found",
            404,
        )));
    };
    if !exists(
        tx.conn(),
        "SELECT 1 FROM memberships m JOIN rooms r ON r.id=m.room_id WHERE m.user_id=? AND r.id=? AND r.deleted_at IS NULL",
        params![message.creator_id, message.room_id],
    )? {
        return Ok(PostResult::Denied(ServiceResult::fail(
            "Message not found",
            404,
        )));
    }
    if !agent_access::capability_for_agent(
        tx.conn(),
        agent_id,
        "post_messages",
        Some(message.room_id),
    )? {
        return Ok(PostResult::Denied(ServiceResult::fail(
            "Forbidden: agent lacks post_messages capability",
            403,
        )));
    }
    if streaming_only && !message.streaming {
        return Ok(PostResult::Denied(ServiceResult::fail(
            "Message is not streaming",
            422,
        )));
    }
    if message.streaming
        && message
            .thread_id
            .map(|id| ChannelThread::find_by_id(tx.conn(), id))
            .transpose()?
            .flatten()
            .is_some_and(|t| t.locked_at.is_some())
    {
        return Ok(PostResult::Denied(ServiceResult::fail(
            "This thread is locked",
            422,
        )));
    }
    Ok(PostResult::Posted(Box::new(message)))
}
pub fn update(
    tx: &mut Tx<'_>,
    agent_id: i64,
    id: i64,
    append: Option<&str>,
    markdown_source: Option<&str>,
) -> Result<PostResult> {
    let result = find_own(tx, agent_id, id, true)?;
    let PostResult::Posted(mut message) = result else {
        return Ok(result);
    };
    if append.is_none() && markdown_source.is_none() {
        return Ok(PostResult::Denied(ServiceResult::fail(
            "append or markdown_source is required",
            422,
        )));
    }
    let source = append.map_or_else(
        || markdown_source.unwrap_or_default().to_owned(),
        |text| {
            format!(
                "{}{text}",
                message.markdown_source.as_deref().unwrap_or_default()
            )
        },
    );
    if let Err(error) = tx.savepoint(|tx| {
        message.update(
            tx,
            MessageChanges {
                markdown_source: Some(source),
                ..Default::default()
            },
        )
    }) {
        return match error {
            Error::RecordInvalid(e) => Ok(PostResult::Denied(invalid(e))),
            e => Err(e),
        };
    }
    broadcast_update(tx, &mut message)?;
    Ok(PostResult::Posted(message))
}
pub fn finalize(tx: &mut Tx<'_>, agent_id: i64, id: i64) -> Result<PostResult> {
    let result = find_own(tx, agent_id, id, false)?;
    let PostResult::Posted(mut message) = result else {
        return Ok(result);
    };
    if message.streaming {
        message.finalize_stream(tx)?;
    }
    message.reload(tx.conn())?;
    Ok(PostResult::Posted(message))
}
pub fn broadcast_update(tx: &mut Tx<'_>, message: &mut Message) -> Result<bool> {
    if let Some(last) = message.stream_broadcast_at
        && last > tx.now().ago(BROADCAST_INTERVAL)
    {
        tx.emit_after_commit(crate::Event::job(&StreamTrailingBroadcastJob {
            message_id: message.id,
            last_broadcast_at: stamp(last),
        }));
        return Ok(false);
    }
    tx.conn().execute(
        "UPDATE messages SET stream_broadcast_at=? WHERE id=?",
        params![tx.now(), message.id],
    )?;
    message.stream_broadcast_at = Some(tx.now());
    broadcast_final(tx, message)?;
    Ok(true)
}
pub(crate) fn broadcast_final(tx: &mut Tx<'_>, message: &Message) -> Result<()> {
    use crate::broadcasts::{Broadcast, Partial, conversation_messages, message_dom_id};
    tx.emit_after_commit(crate::Event::broadcast(&Broadcast::replace(
        conversation_messages(tx.conn(), message)?,
        message_dom_id(message, None),
        Partial::MessageReplace {
            message_id: message.id,
        },
    )));
    Ok(())
}
pub fn trailing(tx: &mut Tx<'_>, job: &StreamTrailingBroadcastJob) -> Result<bool> {
    let Some(mut message) = Message::find_by_id(tx.conn(), job.message_id)? else {
        return Ok(false);
    };
    if !message.streaming
        || message.stream_broadcast_at.map(stamp).as_deref() != Some(&job.last_broadcast_at)
    {
        return Ok(false);
    }
    broadcast_update(tx, &mut message)
}
pub fn overdue_ids(tx: &Tx<'_>, now: Timestamp) -> Result<Vec<i64>> {
    tx.conn().execute("UPDATE messages SET streaming_updated_at=created_at WHERE streaming=1 AND streaming_updated_at IS NULL",[])?;
    query_all(
        tx.conn(),
        "SELECT id FROM messages WHERE messages.streaming = 1 AND messages.streaming_updated_at < ? ORDER BY id",
        [now.ago(FINALIZE_AFTER)],
        |r| r.get(0),
    )
}
