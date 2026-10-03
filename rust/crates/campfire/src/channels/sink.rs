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
        campfire_db::models::board_automations::DigestNotes::KIND => decode(request).and_then(|notes| {
            app.map_or(Ok(()), |app| super::board_digests::deliver(cable, app, &notes).map(|_| ()))
        }),
        campfire_db::models::huddle_effects::StageEndedNote::KIND => decode::<campfire_db::models::huddle_effects::StageEndedNote>(request).and_then(|e|app.map_or(Ok(()),|app|super::huddle_effects::stage_ended_note(app,e.message_id))),
        campfire_db::models::huddle_effects::StagePanel::KIND => decode::<campfire_db::models::huddle_effects::StagePanel>(request).and_then(|e|app.map_or(Ok(()),|app|super::huddle_effects::stage_panel(app,e.room_id,e.membership_id))),
        campfire_db::models::huddle_effects::StreamChanged::KIND => decode::<campfire_db::models::huddle_effects::StreamChanged>(request).and_then(|e|app.map_or(Ok(()),|app|super::huddle_effects::stream_changed(app,e.room_id))),
        campfire_db::models::huddle_effects::StreamStopped::KIND => decode::<campfire_db::models::huddle_effects::StreamStopped>(request).and_then(|e|app.map_or(Ok(()),|app|super::huddle_effects::stream_stopped(app,e.room_id,e.user_id))),
        campfire_db::models::huddle_effects::StageRoster::KIND => decode::<campfire_db::models::huddle_effects::StageRoster>(request).and_then(|e|app.map_or(Ok(()),|app|super::huddle_effects::stage_roster(app,e.room_id))),
        campfire_db::models::huddle_effects::RoleEvent::KIND => decode::<campfire_db::models::huddle_effects::RoleEvent>(request).and_then(|e|app.map_or(Ok(()),|app|super::huddle_effects::role_event(app,e.room_id,e.membership_id))),
        campfire_db::models::huddle_effects::Presence::KIND => decode::<campfire_db::models::huddle_effects::Presence>(request).and_then(|effect| {
            app.map_or(Ok(()), |app| super::huddle_effects::presence(app, effect.room_id))
        }),
        RoomRemovalBroadcast::KIND => decode(request).map(|broadcast| room_removal(cable, &broadcast, app.map_or_else(||huddle_configured(env),|app|app.config.huddle.configured()))),
        campfire_db::broadcasts::Broadcast::KIND => decode(request).and_then(|broadcast| {
            if let Some(app) = app
                && (super::message_features::deliver(cable, app, &broadcast)?
                    || super::room_composition::deliver(app, &broadcast)?)
            {
                return Ok(());
            }
            messaging(cable, app, &broadcast)
        }),
        campfire_db::models::user_status_settings::updates::StatusBadgeBroadcast::KIND =>
            decode(request).and_then(|broadcast| status_badge(cable, broadcast)),
        campfire_db::models::user_status_settings::updates::OooNoticeBroadcast::KIND =>
            decode(request).and_then(|broadcast| ooo_notice(cable, broadcast)),
        campfire_db::models::calendar_event::CardUpdate::KIND => decode::<campfire_db::models::calendar_event::CardUpdate>(request).and_then(|event| {
            let app = app.ok_or_else(|| anyhow::anyhow!("app not booted"))?;
            super::event_cards::publish(app, event.event_id)
        }),
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
                let html = crate::controllers::presenters::page::render_detached(&copy, None, |ctx| campfire_views::messages::MessagePartial {ctx,message:&view}.render().expect("messages/_message renders"));
                copy.broadcasts.turbo(&super::broadcasts::Stream::conversation(&room, &message), Action::Replace,
                    &super::broadcasts::message_dom_id(&message, None), Some(&html), false);
                Ok(())
            })?;
            Ok(())
        }),
        crate::integrations::github::notifier::MessageCreated::KIND => decode(request).and_then(|broadcast| {
            let app = app.ok_or_else(|| anyhow::anyhow!("app has not booted"))?;
            super::github_notifier::publish(app, &broadcast)
        }),
        crate::integrations::github::pull_requests::CardUpdated::KIND => decode(request).and_then(|broadcast| {
            let app = app.ok_or_else(|| anyhow::anyhow!("app has not booted"))?;
            super::github_cards::publish(app, &broadcast)
        }),
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
/// directory partial descriptions belongs to WS8br; message/poll/pin partials use WS8b-m's seam.
fn messaging(cable: &Cable, app: Option<&App>, broadcast: &campfire_db::broadcasts::Broadcast) -> anyhow::Result<()> {
    use campfire_db::broadcasts::{Broadcast, TurboAction};
    if let Broadcast::Turbo(frame) = broadcast
        && matches!(frame.action, campfire_db::broadcasts::TurboAction::Append | campfire_db::broadcasts::TurboAction::Replace)
        && let Some(campfire_db::broadcasts::Partial::Message {message_id} | campfire_db::broadcasts::Partial::MessageReplace {message_id}) = &frame.partial
    {
        let app=app.ok_or_else(||anyhow::anyhow!("app not booted"))?;
        let copy=app.clone();let message_id=*message_id;
        let html=app.db.read_blocking(move|conn| {
            let message=campfire_db::Message::find(conn,message_id)?;
            let view=crate::controllers::presenters::Presenter::new(conn,&copy,None).message(&message)?;
            // APP_URL supplies route defaults. Without it ActionController's
            // renderer uses example.org, independent of mail's example.com fallback.
            let origin=crate::controllers::presenters::page::default_renderer_base_url(&copy);
            // Rails broadcasts render the partial directly. A stream update can
            // keep its frozen updated_at, so the collection cache would be stale.
            Ok(crate::controllers::presenters::page::render_detached_at(&copy,None,origin,|ctx|campfire_views::messages::MessagePartial {ctx,message:&view}.render().expect("messages/_message renders")))
        })?;
        if campfire_views::helpers::request_forgery::has_token_slots(&html) {
            anyhow::bail!("refusing unresolved CSRF token slots in a message replacement");
        }
        let streamables:Vec<_>=frame.streamables.iter().map(|s|s.to_param()).collect();
        let streamables:Vec<_>=streamables.iter().map(String::as_str).collect();
        let attrs:&[(&str,Option<&str>)]=if frame.maintain_scroll {&[("maintain_scroll",Some("true"))]} else {&[]};
        let action=if frame.action == campfire_db::broadcasts::TurboAction::Append {Action::Append} else {Action::Replace};
        cable.broadcast_action_to(&streamables,action,Target::Target(&frame.target),Some(&html),attrs);
        return Ok(());
    }
    if let Some((stream, payload)) = template_free_broadcast(broadcast) {
        match broadcast {
            Broadcast::Cable { .. } => { cable.broadcast(&stream, &payload); }
            Broadcast::Turbo(_) => { cable.broadcast_stream_to(&[&stream], payload.as_str().expect("Turbo frame is a string")); }
        }
        return Ok(());
    }
    let Broadcast::Turbo(frame) = broadcast else { unreachable!() };
    let app = app.ok_or_else(|| anyhow::anyhow!("app is not booted for partial rendering"))?;
    let html = match &frame.partial {
        Some(campfire_db::broadcasts::Partial::BoardRow {thread_id, column}) => {
            Some(app.db.read_blocking(|conn| {
                let thread = campfire_db::ChannelThread::find(conn, *thread_id)?;
                let room = campfire_db::Room::find(conn, thread.room_id)?;
                let presenter = crate::controllers::presenters::Presenter::new(conn, app, None);
                let rows = crate::controllers::presenters::boards::rows(&presenter, &room, &[thread])?;
                crate::controllers::presenters::page::render_detached_at(app, None, "http://example.org", |ctx| {
                    campfire_views::rooms::boards::RowPartial {ctx, row:&rows[0], column:*column}.render().map_err(|error| campfire_db::Error::Other(error.to_string()))
                })
            })?)
        }
        Some(campfire_db::broadcasts::Partial::EventCards { message_id }) => {
            Some(app.db.read_blocking(|conn| crate::controllers::presenters::events::cards(conn, *message_id))?)
        }
        Some(partial) => match super::rooms_directory::render(app, partial)? {
            Some(html) => Some(html),
            None => crate::controllers::messages::rendered::domain_partial(app, partial)?,
        },
        None => None,
    }.ok_or_else(|| anyhow::anyhow!("WS8b partial rendering is not registered: {broadcast:?}"))?;
    if campfire_views::helpers::request_forgery::has_token_slots(&html) { return Err(anyhow::anyhow!("unresolved CSRF token slot")); }
    let action = match frame.action {
        TurboAction::Append => Action::Append,
        TurboAction::Prepend => Action::Prepend,
        TurboAction::Replace => Action::Replace,
        TurboAction::Update => Action::Update,
        TurboAction::Remove => Action::Remove,
    };
    let stream = broadcast.stream_name();
    let attributes = [("maintain_scroll", frame.maintain_scroll.then_some("true"))];
    cable.broadcast_action_to(&[&stream], action, Target::Target(&frame.target), Some(&html), &attributes);
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
    crate::huddle::Config::from_lookup(env).configured()
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
    fn huddle_configuration_matches_ws13_rails_vectors() {
        let vectors: serde_json::Value = serde_json::from_str(include_str!("../huddle/protocol_vectors.json")).unwrap();
        for case in vectors["urls"].as_array().unwrap() {
            let configured = huddle_configured(|name| Some(match name {
                "LIVEKIT_URL" => case["public_url"].as_str().unwrap(),
                "LIVEKIT_INTERNAL_URL" => case["internal_url"].as_str().unwrap(),
                _ => "ws13-fixture-value",
            }.to_string()));
            assert_eq!(configured, case["configured"].as_bool().unwrap(), "{case}");
        }
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

#[cfg(test)]
mod stream_tests;
