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
use askama::Template;

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
        RoomRemovalBroadcast::KIND => decode(request).map(|broadcast| room_removal(cable, &broadcast, huddle_configured(env))),
        crate::integrations::link_embed::store::CardUpdate::KIND => decode::<crate::integrations::link_embed::store::CardUpdate>(request)
            .and_then(|event| {
                let app = app.ok_or_else(|| anyhow::anyhow!("app not booted"))?;
                crate::controllers::presenters::link_embeds::broadcast_updates(app, event.embed_id)
            }),
        crate::integrations::fizzy::cards::CardUpdate::KIND => decode::<crate::integrations::fizzy::cards::CardUpdate>(request).and_then(|event| {
            let app = app.ok_or_else(|| anyhow::anyhow!("app not booted"))?;
            crate::controllers::presenters::fizzy_cards::broadcast_updates(app, event.card_id)
        }),
        crate::integrations::twitter::post::CardUpdate::KIND => decode::<crate::integrations::twitter::post::CardUpdate>(request).and_then(|event| {
            let app = app.ok_or_else(|| anyhow::anyhow!("app not booted"))?;
            crate::controllers::presenters::twitter_cards::broadcast_updates(app, event.post_id)
        }),
        campfire_db::models::user::lifecycle::QuietStreamFinal::KIND => decode::<campfire_db::models::user::lifecycle::QuietStreamFinal>(request).and_then(|event| {
            let app = app.ok_or_else(|| anyhow::anyhow!("app not booted"))?;
            let copy = app.clone();
            app.db.read_blocking(move |conn| {
                let message = campfire_db::Message::find(conn, event.message_id)?;
                let room = campfire_db::Room::find(conn, message.room_id)?;
                let view = crate::controllers::presenters::Presenter::new(conn, &copy, None).message(&message)?;
                let html = crate::controllers::presenters::page::render_detached(&copy, None, |ctx| campfire_views::messages::message(ctx, &view));
                copy.broadcasts.turbo(&super::broadcasts::Stream::conversation(&room, &message), Action::Replace,
                    &super::broadcasts::message_dom_id(&message, None), Some(&html), false);
                Ok(())
            })?;
            Ok(())
        }),
        campfire_db::broadcasts::Broadcast::KIND => decode(request).and_then(|broadcast| messaging(cable, app, &broadcast)),
        crate::integrations::github::notifier::MessageCreated::KIND => decode(request).and_then(|broadcast| {
            let app = app.ok_or_else(|| anyhow::anyhow!("app has not booted"))?;
            super::github_notifier::publish(app, &broadcast)
        }),
        crate::integrations::github::pull_requests::CardUpdated::KIND => decode(request).and_then(|broadcast| {
            let app = app.ok_or_else(|| anyhow::anyhow!("app has not booted"))?;
            super::github_cards::publish(app, &broadcast)
        }),
        campfire_db::models::user_status_settings::updates::StatusBadgeBroadcast::KIND => decode(request).and_then(|b| status_badge(cable, b)),
        campfire_db::models::user_status_settings::updates::OooNoticeBroadcast::KIND => decode(request).and_then(|b| ooo_notice(cable, b)),
        kind => Err(anyhow::anyhow!("no handler for the {kind} broadcast")),
    };
    if let Err(error) = result {
        tracing::warn!(kind = request.kind, %error, "broadcast failed");
    }
}

fn status_badge(cable: &Cable, b: campfire_db::models::user_status_settings::updates::StatusBadgeBroadcast) -> anyhow::Result<()> {
    let html=campfire_views::users::statuses::StatusBadge{presence:&b.presence,status_text:b.status_text.as_deref()}.render()?;
    cable.broadcast_action_to(&[&user_gid(b.user_id).to_param(),"status"],Action::Update,Target::Target(&dom_id("user",b.user_id,Some("status_badge"))),Some(&html),&[]);
    Ok(())
}

fn ooo_notice(cable: &Cable, b: campfire_db::models::user_status_settings::updates::OooNoticeBroadcast) -> anyhow::Result<()> {
    let html=campfire_views::users::statuses::OooNotice{name:&b.name,visible:b.visible,until_date:b.until_date.as_deref(),note:b.note.as_deref()}.render()?;
    cable.broadcast_action_to(&[&user_gid(b.user_id).to_param(),"ooo_notice"],Action::Update,Target::Target(&dom_id("user",b.user_id,Some("ooo_notice"))),Some(&html),&[]);
    Ok(())
}

/// WS8 domain frames share WS7's publisher and conservative Turbo guard. Rendering these
/// partial descriptions belongs to WS8b; template-free frames are delivered now.
fn messaging(cable: &Cable, app: Option<&App>, broadcast: &campfire_db::broadcasts::Broadcast) -> anyhow::Result<()> {
    use campfire_db::broadcasts::Broadcast;
    if let Broadcast::Turbo(frame) = broadcast
        && let Some(campfire_db::broadcasts::Partial::Message { message_id }) = &frame.partial
    {
        let app = app.ok_or_else(|| anyhow::anyhow!("message broadcast before app boot"))?;
        let html = app.db.read_blocking(|conn| {
            let message = campfire_db::Message::find(conn, *message_id)?;
            let view = crate::controllers::presenters::Presenter::new(conn, app, None).message(&message)?;
            Ok(crate::controllers::presenters::page::render_detached_at(app, None, &app.db.env().default_url_origin, |ctx| campfire_views::messages::message(ctx, &view)))
        })?;
        let action = match frame.action {
            campfire_db::broadcasts::TurboAction::Append => Action::Append,
            campfire_db::broadcasts::TurboAction::Replace => Action::Replace,
            _ => return Err(anyhow::anyhow!("unexpected message partial action")),
        };
        let attributes = if frame.maintain_scroll { vec![("maintain_scroll", Some("true"))] } else { Vec::new() };
        cable.broadcast_action_to(&[&broadcast.stream_name()], action, Target::Target(&frame.target), Some(&html), &attributes);
        return Ok(());
    }
    if let Broadcast::Turbo(frame) = broadcast
        && let Some(campfire_db::broadcasts::Partial::EventCards { message_id }) = &frame.partial
    {
        let app = app.ok_or_else(|| anyhow::anyhow!("event card broadcast before app boot"))?;
        let html = app.db.read_blocking(|conn| crate::controllers::presenters::events::cards(conn, *message_id))?;
        let action = match frame.action {
            campfire_db::broadcasts::TurboAction::Replace => Action::Replace,
            _ => return Err(anyhow::anyhow!("unexpected event card action")),
        };
        let attributes = if frame.maintain_scroll { vec![("maintain_scroll", Some("true"))] } else { Vec::new() };
        cable.broadcast_action_to(&[&broadcast.stream_name()], action, Target::Target(&frame.target), Some(&html), &attributes);
        return Ok(());
    }
    let (stream, payload) = template_free_broadcast(broadcast)
        .ok_or_else(|| anyhow::anyhow!("WS8b partial rendering is not registered: {broadcast:?}"))?;
    match broadcast {
        Broadcast::Cable { .. } => { cable.broadcast(&stream, &payload); }
        Broadcast::Turbo(_) => { cable.broadcast_stream_to(&[&stream], payload.as_str().expect("Turbo frame is a string")); }
    }
    Ok(())
}

fn decode<B: Broadcast>(request: &BroadcastRequest) -> anyhow::Result<B> {
    Ok(request.decode::<B>().ok_or_else(|| anyhow::anyhow!("not a {} broadcast", B::KIND))??)
}

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok()
}

/// `Membership#broadcast_room_removal_to_user` (reference/app/models/membership.rb).
pub fn room_removal(cable: &Cable, broadcast: &RoomRemovalBroadcast, huddle_configured: bool) {
    let user = user_gid(broadcast.user_id).to_param();
    let param_key = broadcast.room_class.replace("::", "_").to_ascii_lowercase();
    if huddle_configured {
        let target = dom_id(&param_key, broadcast.room_id, Some("header_voice_participants"));
        cable.broadcast_action_to(&[&user, ROOMS], Action::Remove, Target::Target(&target), None, &[]);
    }
    let target = dom_id(&param_key, broadcast.room_id, Some("list"));
    cable.broadcast_action_to(&[&user, ROOMS], Action::Remove, Target::Target(&target), None, &[]);
}

/// `Huddle.configured?` (reference/app/services/huddle.rb): the five LiveKit variables are set and
/// the public and internal LiveKit URLs name different endpoints. Read on every call, as Rails
/// reads `ENV` on every call. (The huddle workstream owns the rest of `Huddle`.)
pub fn huddle_configured(env: impl Fn(&str) -> Option<String>) -> bool {
    const REQUIRED: [&str; 5] = ["LIVEKIT_URL", "LIVEKIT_INTERNAL_URL", "LIVEKIT_API_KEY", "LIVEKIT_API_SECRET", "LIVEKIT_GATEWAY_SECRET"];
    let values: Vec<String> = REQUIRED.iter().filter_map(|name| env(name).filter(|value| !value.trim().is_empty())).collect();
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
        huddle_configured(|name| vars.iter().find(|(key, _)| *key == name).map(|(_, value)| value.to_string()))
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
