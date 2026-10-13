use super::{agent_access, agent_posting, agent_service::ServiceResult};
use crate::{Agent, Message, NewMessage, NewPoll, Poll, Result, Room, Tx};

pub fn create(
    tx: &mut Tx<'_>,
    agent_id: i64,
    room_id: i64,
    question: String,
    attributes: NewPoll,
) -> Result<std::result::Result<(Poll, Room, Message), ServiceResult>> {
    let agent = Agent::find(tx.conn(), agent_id)?.ok_or(crate::Error::RecordNotFound("Agent"))?;
    let Some(room) = Room::find_for_user(tx.conn(), agent.user_id, room_id)? else {
        return Ok(Err(ServiceResult::fail("Room not found", 404)));
    };
    if !agent_access::capability_for_agent(tx.conn(), agent_id, "post_messages", Some(room_id))? {
        return Ok(Err(ServiceResult::fail(
            "Forbidden: agent lacks post_messages capability",
            403,
        )));
    }
    if let Some(denial) = agent_posting::check_budget(tx, agent_id, agent_posting::Cap::Messages)? {
        return Ok(Err(ServiceResult::budget(denial)));
    }
    let message = Message::create(
        tx,
        NewMessage {
            room_id,
            creator_id: agent.user_id,
            markdown_source: Some(question),
            ..Default::default()
        },
    )?;
    let poll = Poll::create_for_message(tx, &message, attributes)?;
    Ok(Ok((poll, room, message)))
}
