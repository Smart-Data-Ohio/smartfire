//! Read-side page facts; call WS11 for approval state and Agent read capability.
use campfire_db::models::agent_delivery::AgentEvent;
use campfire_db::{
    Agent, AgentApproval, Connection, Membership, Message, Result, RichText, Room, Timestamp, User,
};
use campfire_views::agents::history::{Approval, LedgerEvent};
use std::collections::HashMap;
fn present(value: Option<String>) -> Option<String> {
    value.filter(|s| !campfire_richtext::ruby::is_blank(s))
}
struct Associations {
    rooms: HashMap<i64, String>,
    users: HashMap<i64, User>,
    messages: HashMap<i64, Message>,
}
impl Associations {
    fn load(
        conn: &Connection,
        viewer: &User,
        mut rooms: Vec<i64>,
        mut users: Vec<i64>,
        mut messages: Vec<i64>,
    ) -> Result<Self> {
        rooms.sort_unstable();
        rooms.dedup();
        users.sort_unstable();
        users.dedup();
        messages.sort_unstable();
        messages.dedup();
        Ok(Self {
            rooms: Room::for_ids(conn, &rooms)?
                .iter()
                .map(|room| {
                    Ok((
                        room.id,
                        super::super::accounts::room_display_name(conn, room, viewer)?,
                    ))
                })
                .collect::<Result<_>>()?,
            users: if users.is_empty() {
                HashMap::new()
            } else {
                User::where_ids(conn, &users)?
                    .into_iter()
                    .map(|user| (user.id, user))
                    .collect()
            },
            messages: Message::for_ids(conn, &messages)?
                .into_iter()
                .map(|message| (message.id, message))
                .collect(),
        })
    }
}

pub fn approvals(
    conn: &Connection,
    secrets: &rails_compat::Secrets,
    rows: &[AgentApproval],
    bot: &User,
    viewer: &User,
    now: Timestamp,
) -> Result<Vec<Approval>> {
    let associations = Associations::load(
        conn,
        viewer,
        rows.iter().filter_map(|a| a.room_id).collect(),
        rows.iter().filter_map(|a| a.decided_by_id).collect(),
        vec![],
    )?;
    // One owner's policy check per action category. Decision writes still recheck live authority.
    let mut policies = HashMap::new();
    rows.iter()
        .map(|approval| {
            let external = approval.github_action() || approval.fizzy_action();
            let approvable = match policies.get(&external) {
                Some(allowed) => *allowed,
                None => {
                    let allowed = approval.approvable_by(conn, viewer)?;
                    policies.insert(external, allowed);
                    allowed
                }
            };
            Ok(present_approval(
                secrets,
                approval,
                bot,
                now,
                &associations,
                approvable,
            ))
        })
        .collect()
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
    let associations = Associations::load(
        conn,
        viewer,
        approval.room_id.into_iter().collect(),
        approval.decided_by_id.into_iter().collect(),
        vec![],
    )?;
    Ok(present_approval(
        secrets,
        approval,
        &bot,
        now,
        &associations,
        approval.approvable_by(conn, viewer)?,
    ))
}
fn present_approval(
    secrets: &rails_compat::Secrets,
    approval: &AgentApproval,
    bot: &User,
    now: Timestamp,
    associations: &Associations,
    approvable: bool,
) -> Approval {
    Approval {
        id: approval.id,
        bot_name: bot.name.clone(),
        avatar_url: super::super::user_summary(secrets, bot).avatar_path,
        room_name: approval
            .room_id
            .and_then(|id| associations.rooms.get(&id))
            .cloned(),
        action: approval.action.clone(),
        summary: approval.summary.clone(),
        status: approval.effective_status(now).into(),
        expires_at: approval.expires_at.jiff(),
        decided_by: approval
            .decided_by_id
            .and_then(|id| associations.users.get(&id))
            .map(|user| user.name.clone()),
        decision_note: present(approval.decision_note.clone()),
        github_login: present(approval.github_login.clone()),
        fizzy_user_name: present(approval.fizzy_user_name.clone()),
        approvable,
    }
}
pub fn events(
    conn: &Connection,
    rows: Vec<AgentEvent>,
    agent: &Agent,
    viewer: &User,
    rich_text: &dyn RichText,
) -> Result<Vec<LedgerEvent>> {
    let associations = Associations::load(
        conn,
        viewer,
        rows.iter().filter_map(|e| e.room_id).collect(),
        rows.iter().filter_map(|e| e.actor_id).collect(),
        rows.iter().filter_map(|e| e.message_id).collect(),
    )?;
    let mut readable = HashMap::new();
    rows.into_iter()
        .map(|event| {
            present_event(
                conn,
                event,
                agent,
                viewer,
                rich_text,
                &associations,
                &mut readable,
            )
        })
        .collect()
}
fn present_event(
    conn: &Connection,
    event: AgentEvent,
    agent: &Agent,
    viewer: &User,
    rich_text: &dyn RichText,
    associations: &Associations,
    readable: &mut HashMap<i64, bool>,
) -> Result<LedgerEvent> {
    let id = event.id;
    let mut content = None;
    if let Some(message) = event
        .message_id
        .and_then(|id| associations.messages.get(&id))
    {
        let allowed = match readable.get(&message.room_id) {
            Some(allowed) => *allowed,
            None => {
                let allowed =
                    Membership::find_by_room_and_user(conn, message.room_id, agent.user_id)?
                        .is_some()
                        && agent.can(conn, "read_messages", Some(message.room_id))?
                        && (viewer.is_administrator()
                            || Membership::find_by_room_and_user(
                                conn,
                                message.room_id,
                                viewer.id,
                            )?
                            .is_some());
                readable.insert(message.room_id, allowed);
                allowed
            }
        };
        if allowed {
            content = Some(message.plain_text_body(conn, rich_text)?);
        }
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
        room_name: event
            .room_id
            .and_then(|id| associations.rooms.get(&id))
            .cloned(),
        actor_name: event
            .actor_id
            .and_then(|id| associations.users.get(&id))
            .map(|user| user.name.clone()),
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
