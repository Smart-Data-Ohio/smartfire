//! Denials before the WS12 work-write seam. Model field checks use ChannelThread::validate.
//! Replace this adapter with WS12's complete service when its write API lands.
use super::{ruby_i64, text};
use campfire_db::models::agent_service::ServiceResult;
use campfire_db::{Agent, ChannelThread, Membership, Room, Tx};
use serde_json::Value;

fn string(value: Option<&Value>) -> String {
    text(value).unwrap_or_default()
}
fn present_string(value: Option<&Value>) -> Option<String> {
    let value = string(value);
    (!campfire_richtext::ruby::is_blank(&value)).then_some(value)
}
fn denial(error: impl Into<String>) -> Option<ServiceResult> {
    Some(ServiceResult::fail(error, 422))
}

pub(super) fn check(
    tx: &Tx<'_>,
    thread: &ChannelThread,
    op: &str,
    args: &Value,
) -> campfire_db::Result<Option<ServiceResult>> {
    if op == "handoff_work" {
        let receiver = Agent::find(tx.conn(), args.get("receiver_agent_id").map_or(0, ruby_i64))?;
        let Some(receiver) = receiver else {
            return Ok(denial(
                "Receiver must be an active agent member of this room with permission to post",
            ));
        };
        if !receiver.active(tx.conn())?
            || Membership::find_by_room_and_user(tx.conn(), thread.room_id, receiver.user_id)?
                .is_none()
            || !receiver.can(tx.conn(), "post_messages", Some(thread.room_id))?
        {
            return Ok(denial(
                "Receiver must be an active agent member of this room with permission to post",
            ));
        }
        for cap in ["manage_threads", "read_messages"] {
            if !receiver.can(tx.conn(), cap, Some(thread.room_id))? {
                return Ok(denial(format!(
                    "Receiver must hold the {cap} capability in this room"
                )));
            }
        }
        if thread.work_owner_id == Some(receiver.user_id) {
            return Ok(denial("Receiver is already the owner of this work"));
        }
        // Context-package row validation and handoff writes remain with WS12.
        return Ok(None);
    }
    if op == "set_result" {
        if present_string(args.get("markdown"))
            .is_some_and(|value| value.chars().count() > campfire_db::channel_thread::RESULT_LIMIT)
        {
            return Ok(denial(
                "Result markdown is too long (maximum is 20000 characters)",
            ));
        }
        return Ok(None);
    }
    let status = args.get("work_status");
    if let Some(status) = status
        && !campfire_db::channel_thread::WORK_STATUSES.contains(&string(Some(status)).as_str())
    {
        return Ok(denial("Work status is invalid"));
    }
    if present_string(args.get("note")).is_some_and(|value| value.chars().count() > 500) {
        return Ok(denial("Note is too long (maximum is 500 characters)"));
    }
    if status.is_none() && args.get("tags").is_none() && args.get("run_url").is_none() {
        return Ok(denial("Work status is invalid"));
    }
    let mut prospective = thread.clone();
    if let Some(run) = args.get("run_url") {
        prospective.run_url = present_string(Some(run));
    }
    let tags = args.get("tags").map(|value| {
        let names: Vec<String> = if let Some(values) = value.as_array() {
            values.iter().map(|value| string(Some(value))).collect()
        } else {
            string(Some(value)).split(',').map(str::to_owned).collect()
        };
        campfire_db::channel_thread::normalize_tag_names(&names)
    });
    // Rails saves tags/run_url before its status update; retain the old status here.
    let room = Room::find(tx.conn(), thread.room_id)?;
    let errors = prospective.validate(tx.conn(), &room, tags.as_deref())?;
    if !errors.is_empty() {
        return Ok(denial(campfire_db::slash_commands::sentence(
            errors.full_messages(),
        )));
    }
    Ok(None)
}
