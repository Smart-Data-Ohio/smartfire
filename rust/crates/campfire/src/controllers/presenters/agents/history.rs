//! Read-side page facts; call WS11 for approval state and Agent read capability.
use campfire_db::models::agent_delivery::AgentEvent;
use campfire_db::{
    Agent, AgentApproval, Connection, Membership, Message, Result, RichText, Room, Timestamp, User,
};
use campfire_views::agents::history::{Approval, LedgerEvent};
fn present(value: Option<String>) -> Option<String> {
    value.filter(|s| !campfire_richtext::ruby::is_blank(s))
}
fn room_name(conn: &Connection, id: Option<i64>, viewer: &User) -> Result<Option<String>> {
    id.map(|id| Room::find_by_id(conn, id))
        .transpose()?
        .flatten()
        .map(|r| super::super::accounts::room_display_name(conn, &r, viewer))
        .transpose()
}
pub fn approval(
    conn: &Connection,
    secrets: &rails_compat::Secrets,
    approval: &AgentApproval,
    viewer: &User,
    now: Timestamp,
) -> Result<Approval> {
    let agent =
        Agent::find(conn, approval.agent_id)?.ok_or(campfire_db::Error::RecordNotFound("Agent"))?;
    let bot = User::find(conn, agent.user_id)?;
    Ok(Approval {
        id: approval.id,
        bot_name: bot.name.clone(),
        avatar_url: super::super::user_summary(secrets, &bot).avatar_path,
        room_name: room_name(conn, approval.room_id, viewer)?,
        action: approval.action.clone(),
        summary: approval.summary.clone(),
        status: approval.effective_status(now).into(),
        expires_at: approval.expires_at.jiff(),
        decided_by: approval
            .decided_by_id
            .map(|id| User::find_by_id(conn, id))
            .transpose()?
            .flatten()
            .map(|u| u.name),
        decision_note: present(approval.decision_note.clone()),
        github_login: present(approval.github_login.clone()),
        fizzy_user_name: present(approval.fizzy_user_name.clone()),
        approvable: approval.approvable_by(conn, viewer)?,
    })
}
pub fn event(
    conn: &Connection,
    id: i64,
    agent: &Agent,
    viewer: &User,
    rich_text: &dyn RichText,
) -> Result<LedgerEvent> {
    let event =
        AgentEvent::find(conn, id)?.ok_or(campfire_db::Error::RecordNotFound("AgentEvent"))?;
    let mut content = None;
    if let Some(message) = event
        .message_id
        .map(|id| Message::find_by_id(conn, id))
        .transpose()?
        .flatten()
        && Membership::find_by_room_and_user(conn, message.room_id, agent.user_id)?.is_some()
        && agent.can(conn, "read_messages", Some(message.room_id))?
        && (viewer.is_administrator()
            || Membership::find_by_room_and_user(conn, message.room_id, viewer.id)?.is_some())
    {
        content = Some(message.plain_text_body(conn, rich_text)?);
    }
    let metadata_text = |key: &str| {
        event
            .metadata
            .get(key)
            .filter(|v| !v.is_null())
            .map(ruby_text)
    };
    let handoff = if event.event_type == "work_handed_off" {
        event
            .metadata
            .get("handoff")
            .filter(|v| v.is_object())
            .map(|v| v.get("summary").map(ruby_text).unwrap_or_default())
    } else {
        None
    };
    Ok(LedgerEvent {
        id,
        event_type: event.event_type.clone(),
        outcome: event.outcome.clone(),
        created_at: event.created_at.jiff(),
        room_name: room_name(conn, event.room_id, viewer)?,
        actor_name: event
            .actor_id
            .map(|id| User::find_by_id(conn, id))
            .transpose()?
            .flatten()
            .map(|u| u.name),
        message_id: event.message_id,
        hop: event.hop(),
        detail: present(event.detail),
        webhook_status: event.webhook_status,
        webhook_attempts: event.webhook_attempts,
        webhook_last_error: present(event.webhook_last_error),
        external_metadata: event.metadata.is_object(),
        external_action: metadata_text("action"),
        external_status: metadata_text("status"),
        external_message: present(metadata_text("message")),
        handoff_summary: handoff,
        content,
    })
}
fn ruby_text(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Null => String::new(),
        v => v.to_string(),
    }
}
