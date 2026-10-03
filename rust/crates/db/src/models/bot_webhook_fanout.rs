//! `Message::BotWebhookFanout` and Agent::Delivery's legacy hop gate. Shared by the
//! message controller, slash-command posting and the scheduled-message dispatcher.
use jiff::SignedDuration;
use rusqlite::params;

use super::agent_delivery::AgentEvent;
use crate::sql::{query_all, query_one};
use crate::{Message, Result, Room, Tx, User};

pub const HOP_LIMIT: i64 = 3;
const TRIGGER_WINDOW: SignedDuration = SignedDuration::from_secs(5 * 60);

/// Thread, quiet and unfinished messages never fan out. Agent-backed bots use the ledger.
pub fn recipients(tx: &Tx<'_>, message: &Message) -> Result<Vec<User>> {
    if message.system_note || message.thread_id.is_some() || message.streaming {
        return Ok(Vec::new());
    }
    let room = Room::find(tx.conn(), message.room_id)?;
    let candidates = if room.direct() {
        room.active_bots(tx.conn())?
    } else {
        message.mentionees(tx.conn(), tx.rich_text())?
    };
    let mut bots = Vec::new();
    for bot in candidates {
        if bot.id != message.creator_id
            && bot.is_active()
            && bot.is_bot()
            && !crate::sql::exists(
                tx.conn(),
                "SELECT 1 FROM agents WHERE user_id = ?",
                [bot.id],
            )?
        {
            bots.push(bot);
        }
    }
    if bots.is_empty() || hop_for_message(tx, message)? >= HOP_LIMIT {
        return Ok(Vec::new());
    }
    Ok(bots)
}

pub fn deliver(tx: &mut Tx<'_>, message: &Message) -> Result<()> {
    for bot in recipients(tx, message)? {
        bot.deliver_webhook_later(tx, message.id)?;
    }
    Ok(())
}

/// Human messages start at zero; agents continue their latest authorized trigger across
/// rooms. A legacy bot follows its reply source or the newest plausible room trigger.
pub fn hop_for_message(tx: &Tx<'_>, message: &Message) -> Result<i64> {
    Ok(hop_and_chain_for_message(tx, message)?.0)
}

pub fn hop_and_chain_for_message(tx: &Tx<'_>, message: &Message) -> Result<(i64, Option<String>)> {
    let sender = query_one(
        tx.conn(),
        "SELECT id FROM agents WHERE user_id=?",
        [message.creator_id],
        |r| r.get(0),
    )?;
    hop_and_chain_for_message_sender(tx, message, sender)
}

pub(crate) fn hop_and_chain_for_message_sender(
    tx: &Tx<'_>,
    message: &Message,
    sender: Option<i64>,
) -> Result<(i64, Option<String>)> {
    if let Some(agent_id) = sender {
        return hop_and_chain_for_agent(tx, agent_id, Some(message.creator_id));
    }
    if !message.creator(tx.conn())?.is_bot() {
        return Ok((0, None));
    }
    let source = if let Some(source) = message.reply_to_message_id {
        Some(source)
    } else {
        // Rails where.not(id:, creator_id:) negates the conjunction, not each key.
        let ids = query_all(
            tx.conn(),
            "SELECT id FROM messages WHERE room_id = ? AND created_at >= ? AND NOT (id = ? AND creator_id = ?) ORDER BY id DESC LIMIT 25",
            params![
                message.room_id,
                tx.now().ago(TRIGGER_WINDOW),
                message.id,
                message.creator_id
            ],
            |row| row.get::<_, i64>(0),
        )?;
        let mut found = None;
        for id in ids {
            let candidate = Message::find(tx.conn(), id)?;
            let replied_to_bot = if let Some(reply) = candidate.reply_to_message_id {
                Message::find_by_id(tx.conn(), reply)?
                    .is_some_and(|reply| reply.creator_id == message.creator_id)
            } else {
                false
            };
            if replied_to_bot
                || candidate
                    .mentionees(tx.conn(), tx.rich_text())?
                    .iter()
                    .any(|user| user.id == message.creator_id)
            {
                found = Some(id);
                break;
            }
        }
        found
    };
    let Some(source) = source else {
        return Ok((0, None));
    };
    let hop: Option<AgentEvent> = query_one(
        tx.conn(),
        "SELECT * FROM agent_events WHERE message_id = ? AND outcome IN ('pending','delivered','acknowledged') ORDER BY hop DESC, id DESC LIMIT 1",
        [source],
        AgentEvent::from_row,
    )?;
    Ok(hop.map_or((0, None), |event| (event.hop() + 1, event.chain_id)))
}

/// Work assignments use the actor's server-authorized trigger, never request metadata.
pub fn hop_and_chain_for_actor(
    tx: &Tx<'_>,
    actor_id: Option<i64>,
) -> Result<(i64, Option<String>)> {
    let agent: Option<i64> = query_one(
        tx.conn(),
        "SELECT id FROM agents WHERE user_id=?",
        [actor_id],
        |r| r.get(0),
    )?;
    let Some(agent_id) = agent else {
        return Ok((0, None));
    };
    hop_and_chain_for_agent(tx, agent_id, actor_id)
}
fn hop_and_chain_for_agent(
    tx: &Tx<'_>,
    agent_id: i64,
    actor_id: Option<i64>,
) -> Result<(i64, Option<String>)> {
    let trigger = query_one(
        tx.conn(),
        "SELECT * FROM agent_events WHERE agent_id = ? AND event_type IN ('mention','direct_message','reply','work_assigned','work_unassigned','work_handed_off') AND outcome IN ('pending','delivered','acknowledged') AND created_at >= ? AND (actor_id IS NULL OR actor_id != ?) ORDER BY id DESC LIMIT 1",
        params![agent_id, tx.now().ago(TRIGGER_WINDOW), actor_id],
        AgentEvent::from_row,
    )?;
    Ok(trigger.map_or((0, None), |event| (event.hop() + 1, event.chain_id)))
}
