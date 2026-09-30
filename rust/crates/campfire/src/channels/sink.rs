//! The models' cable events, delivered on the committing thread in the order they were emitted
//! (so a sidebar removal reaches the socket before the disconnect that follows it, as the
//! `after_destroy_commit` callbacks run in Rails):
//!
//! - `Event::DisconnectUser`: `remote_connections.where(current_user:).disconnect` ([`revocation`]).
//! - `Event::Broadcast`: a model's own broadcast (`broadcast_*_to` in a model callback), looked up
//!   here by its [`Broadcast::KIND`] and sent through the cable server. A handler gets the app when
//!   it has booted, to render partials with a reader connection.
//!
//! Broadcasts that controllers and jobs make go through [`super::Broadcasts`] directly; this is
//! only for the ones the database layer emits, which can't render or reach the cable server.

use campfire_cable::turbo::{Action, Target};
use campfire_db::{Broadcast, BroadcastRequest, Event, RoomRemovalBroadcast};

use super::broadcasts::{ROOMS, dom_id};
use super::{Cable, revocation, user_gid};
use crate::app::App;

/// Delivers `event` if it belongs to the cable server; returns false for the rest.
pub fn deliver(cable: &Cable, app: Option<&App>, event: &Event) -> bool {
    match event {
        Event::DisconnectUser { user_id, reconnect } => {
            revocation::disconnect_user(cable, *user_id, *reconnect);
            true
        }
        Event::Broadcast(request) => {
            broadcast(cable, app, request);
            true
        }
        _ => false,
    }
}

/// Runs the broadcast's handler. A broadcast that fails is logged and dropped: the model
/// callbacks rescue and report it (`Rails.error.report(..., handled: true)`), so the ones after
/// it still run.
fn broadcast(cable: &Cable, app: Option<&App>, request: &BroadcastRequest) {
    let result = match request.kind {
        RoomRemovalBroadcast::KIND => {
            decode(request).map(|broadcast| room_removal(cable, &broadcast, huddle_configured(env)))
        }
        campfire_db::broadcasts::Broadcast::KIND => {
            decode(request).and_then(|broadcast| messaging(cable, &broadcast))
        }
        campfire_db::models::agent::AgentStatusChange::KIND => {
            decode(request).and_then(|broadcast| agent_status(app, &broadcast))
        }
        campfire_db::models::agent_step::StepParentChange::KIND => {
            decode(request).and_then(|broadcast| agent_steps(app, &broadcast))
        }
        kind => Err(anyhow::anyhow!("no handler for the {kind} broadcast")),
    };
    if let Err(error) = result {
        tracing::warn!(kind = request.kind, %error, "broadcast failed");
    }
}

fn agent_steps(
    app: Option<&App>,
    change: &campfire_db::models::agent_step::StepParentChange,
) -> anyhow::Result<()> {
    use askama::Template;
    let app = app.ok_or_else(|| anyhow::anyhow!("Agent step rendering requires the booted app"))?;
    if let Some(id) = change.message_id {
        let Some((room, message)) = app.db.read_blocking(|conn| {
            let Some(message) = campfire_db::Message::find_by_id(conn, id)? else {
                return Ok(None);
            };
            Ok(Some((
                campfire_db::Room::find(conn, message.room_id)?,
                message,
            )))
        })?
        else {
            return Ok(());
        };
        let Some(html) = crate::controllers::messages::rendered::domain_partial(
            app,
            &campfire_db::broadcasts::Partial::Message { message_id: id },
        )?
        else {
            return Ok(());
        };
        app.broadcasts.replace(
            &super::broadcasts::Stream::conversation(&room, &message),
            &super::broadcasts::message_dom_id(&message, None),
            &html,
        );
        return Ok(());
    }
    let Some(id) = change.thread_id else {
        return Ok(());
    };
    let steps=app.db.read_blocking(|conn| {
        if campfire_db::ChannelThread::find_by_id(conn,id)?.is_none(){return Ok(None);}
        let mut statement=conn.prepare("SELECT name,status,duration_ms,input_summary,output_summary FROM agent_steps WHERE channel_thread_id=? ORDER BY position,id")?;
        let rows=statement.query_map([id],|r|Ok(campfire_views::messages::parts::AgentStep{name:r.get(0)?,status:r.get(1)?,duration_ms:r.get(2)?,input_summary:r.get(3)?,output_summary:r.get(4)?}))?.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(Some(rows))
    })?;
    let Some(steps) = steps else {
        return Ok(());
    };
    let html = campfire_views::agents::ThreadSteps {
        thread_id: id,
        steps,
    }
    .render()?;
    app.broadcasts.replace(
        &super::broadcasts::Stream::thread_messages(id),
        &format!("agent_steps_channel_thread_{id}"),
        &html,
    );
    Ok(())
}

/// WS11 emits the badge followed by the directory row after commit. Render
/// synchronously on that thread, preserving Rails' callback/frame order.
fn agent_status(
    app: Option<&App>,
    change: &campfire_db::models::agent::AgentStatusChange,
) -> anyhow::Result<()> {
    use crate::controllers::presenters;
    use askama::Template;
    use campfire_db::models::agent::AgentStatusTarget;
    let app =
        app.ok_or_else(|| anyhow::anyhow!("Agent status rendering requires the booted app"))?;
    let Some(agent) = app.db.read_blocking(|conn| {
        presenters::agents::directory_agent(conn, &app.secrets, change.agent_id)
    })?
    else {
        return Ok(());
    };
    let now = app.clock.now();
    let (prefix, html) = presenters::page::render_detached(app, None, |ctx| match change.target {
        AgentStatusTarget::Badge => campfire_views::agents::StatusBadge {
            ctx,
            agent: &agent,
            now,
        }
        .render()
        .map(|html| ("status_badge", html)),
        AgentStatusTarget::DirectoryRow => campfire_views::agents::DirectoryRow {
            ctx,
            agent: &agent,
            now,
        }
        .render()
        .map(|html| ("directory_row", html)),
    })?;
    app.broadcasts.replace(
        &super::broadcasts::Stream::named(super::agents::STREAM_NAME),
        &format!("{prefix}_agent_{}", agent.id),
        &html,
    );
    Ok(())
}

/// WS8 domain frames share WS7's publisher and conservative Turbo guard. Rendering these
/// partial descriptions belongs to WS8b; template-free frames are delivered now.
fn messaging(cable: &Cable, broadcast: &campfire_db::broadcasts::Broadcast) -> anyhow::Result<()> {
    use campfire_db::broadcasts::Broadcast;
    let (stream, payload) = template_free_broadcast(broadcast).ok_or_else(|| {
        anyhow::anyhow!("WS8b partial rendering is not registered: {broadcast:?}")
    })?;
    match broadcast {
        Broadcast::Cable { .. } => {
            cable.broadcast(&stream, &payload);
        }
        Broadcast::Turbo(_) => {
            cable.broadcast_stream_to(
                &[&stream],
                payload.as_str().expect("Turbo frame is a string"),
            );
        }
    }
    Ok(())
}

fn decode<B: Broadcast>(request: &BroadcastRequest) -> anyhow::Result<B> {
    Ok(request
        .decode::<B>()
        .ok_or_else(|| anyhow::anyhow!("not a {} broadcast", B::KIND))??)
}

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok()
}

/// `Membership#broadcast_room_removal_to_user` (reference/app/models/membership.rb).
pub fn room_removal(cable: &Cable, broadcast: &RoomRemovalBroadcast, huddle_configured: bool) {
    let user = user_gid(broadcast.user_id).to_param();
    let param_key = broadcast.room_class.replace("::", "_").to_ascii_lowercase();
    if huddle_configured {
        let target = dom_id(
            &param_key,
            broadcast.room_id,
            Some("header_voice_participants"),
        );
        cable.broadcast_action_to(
            &[&user, ROOMS],
            Action::Remove,
            Target::Target(&target),
            None,
            &[],
        );
    }
    let target = dom_id(&param_key, broadcast.room_id, Some("list"));
    cable.broadcast_action_to(
        &[&user, ROOMS],
        Action::Remove,
        Target::Target(&target),
        None,
        &[],
    );
}

/// `Huddle.configured?` (reference/app/services/huddle.rb): the five LiveKit variables are set and
/// the public and internal LiveKit URLs name different endpoints. Read on every call, as Rails
/// reads `ENV` on every call. (The huddle workstream owns the rest of `Huddle`.)
pub fn huddle_configured(env: impl Fn(&str) -> Option<String>) -> bool {
    const REQUIRED: [&str; 5] = [
        "LIVEKIT_URL",
        "LIVEKIT_INTERNAL_URL",
        "LIVEKIT_API_KEY",
        "LIVEKIT_API_SECRET",
        "LIVEKIT_GATEWAY_SECRET",
    ];
    let values: Vec<String> = REQUIRED
        .iter()
        .filter_map(|name| env(name).filter(|value| !value.trim().is_empty()))
        .collect();
    if values.len() != REQUIRED.len() {
        return false;
    }
    match (endpoint_address(&values[0]), endpoint_address(&values[1])) {
        (Some(public), Some(internal)) => public != internal,
        _ => false,
    }
}

/// `Huddle.endpoint_address`: `[host.downcase, port || default_port]`; `None` is the rescued
/// `URI::InvalidURIError` (no host, or a scheme other than http, ws, https and wss).
fn endpoint_address(value: &str) -> Option<(String, u16)> {
    let uri = crate::security::ruby_uri::parse(value)?;
    let default_port = match uri.scheme.as_deref()? {
        "http" | "ws" => 80,
        "https" | "wss" => 443,
        _ => return None,
    };
    let host = uri.host.filter(|host| !host.trim().is_empty())?;
    Some((host.to_ascii_lowercase(), uri.port.unwrap_or(default_port)))
}

pub(crate) fn template_free_broadcast(
    broadcast: &campfire_db::broadcasts::Broadcast,
) -> Option<(String, serde_json::Value)> {
    use campfire_db::broadcasts::{Broadcast, TurboAction};
    match broadcast {
        Broadcast::Cable { stream, payload } => Some((stream.clone(), payload.clone())),
        Broadcast::Turbo(frame)
            if frame.action == TurboAction::Remove && frame.partial.is_none() =>
        {
            let html = campfire_cable::turbo::action_tag(
                campfire_cable::turbo::Action::Remove,
                campfire_cable::turbo::Target::Target(&frame.target),
                None,
                &[],
            );
            Some((broadcast.stream_name(), serde_json::Value::String(html)))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn configured(vars: &[(&str, &str)]) -> bool {
        huddle_configured(|name| {
            vars.iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| value.to_string())
        })
    }

    #[test]
    fn huddle_configured_needs_every_variable_and_separate_endpoints() {
        let mut vars = vec![
            ("LIVEKIT_URL", "wss://livekit.example.test"),
            ("LIVEKIT_INTERNAL_URL", "http://livekit.internal:7880"),
            ("LIVEKIT_API_KEY", "fake-key"),
            ("LIVEKIT_API_SECRET", "fake-secret-value"),
            ("LIVEKIT_GATEWAY_SECRET", "fake-gateway-value"),
        ];
        assert!(configured(&vars));
        vars[4].1 = " ";
        assert!(!configured(&vars));
        vars[4].1 = "fake-gateway-value";
        // The same host and port, spelled differently, is the same endpoint.
        vars[1].1 = "https://LIVEKIT.example.test:443/";
        assert!(!configured(&vars));
        vars[1].1 = "ftp://livekit.internal";
        assert!(!configured(&vars));
        vars[1].1 = "livekit.internal:7880";
        assert!(!configured(&vars));
    }
}
