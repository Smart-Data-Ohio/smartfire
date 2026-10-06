//! Thin authenticated transport adapter over the shared, atomic MessagePin model.
use super::ruby_i64;
use campfire_db::models::agent_service::ServiceResult;
use campfire_db::{Agent, Message, MessagePin, Tx};
use serde_json::{Value, json};

pub(super) fn operation(
    tx: &mut Tx<'_>,
    agent_id: i64,
    op: &str,
    args: Value,
) -> campfire_db::Result<ServiceResult> {
    // pending::preflight already applied the common membership and post_messages gates.
    let agent =
        Agent::find(tx.conn(), agent_id)?.ok_or(campfire_db::Error::RecordNotFound("Agent"))?;
    let message = Message::find(tx.conn(), args.get("message_id").map_or(0, ruby_i64))?;
    let pinned = op == "pin_message";
    let status = if pinned {
        match tx.savepoint(|tx| MessagePin::pin(tx, &message, agent.user_id)) {
            Ok(Ok(_)) => 201,
            Ok(Err(cap)) => return Ok(ServiceResult::fail(cap.0, 422)),
            Err(campfire_db::Error::RecordInvalid(_)) => 200,
            Err(error) => return Err(error),
        }
    } else {
        if let Some(pin) = MessagePin::find_by_message(tx.conn(), message.id)? {
            pin.unpin(tx)?;
        }
        200
    };
    let count = MessagePin::count_for_room(tx.conn(), message.room_id)?;
    Ok(ServiceResult::ok(
        json!({"pinned":pinned,"message_id":message.id,"pin_count":count}),
        status,
    ))
}
