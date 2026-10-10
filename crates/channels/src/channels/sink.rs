//! The models' cable events, delivered on the committing thread in the order they were emitted
//! (so a sidebar removal reaches the socket before the disconnect that follows it, as the
//! `after_destroy_commit` callbacks run in Rails):
//!
//! - `Event::DisconnectUser`: `remote_connections.where(current_user:).disconnect` ([`revocation`]).
//! - `Event::Broadcast`: a model's own broadcast (`broadcast_*_to` in a model callback), looked up
//!   here by its [`Broadcast::KIND`] and sent through the cable server. A handler gets the app when
//!   it has booted, to load JSON facts with a reader connection.
//!
//! Broadcasts that controllers and jobs make go through [`super::Broadcasts`] directly; this is
//! only for the ones the database layer emits, which can't reach the cable server.


use campfire_db::{Broadcast, BroadcastRequest, Event, RoomRemovalBroadcast};

use super::{Cable, revocation};
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
        campfire_db::models::board_automations::DigestNotes::KIND => decode(request).and_then(|notes| {
            app.map_or(Ok(()), |app| super::board_digests::deliver(cable, app, &notes).map(|_| ()))
        }),
        campfire_db::models::huddle_effects::StageEndedNote::KIND => decode::<campfire_db::models::huddle_effects::StageEndedNote>(request).and_then(|e|app.map_or(Ok(()),|app|super::huddle_effects::stage_ended_note(app,e.message_id))),
        campfire_db::models::huddle_effects::StreamChanged::KIND => decode::<campfire_db::models::huddle_effects::StreamChanged>(request).and_then(|e|app.map_or(Ok(()),|app|super::huddle_effects::stream_changed(app,e.room_id))),
        campfire_db::models::huddle_effects::StreamStopped::KIND => decode::<campfire_db::models::huddle_effects::StreamStopped>(request).and_then(|e|app.map_or(Ok(()),|app|super::huddle_effects::stream_stopped(app,e.room_id,e.user_id))),
        campfire_db::models::huddle_effects::StageRoster::KIND => decode::<campfire_db::models::huddle_effects::StageRoster>(request).and_then(|e|app.map_or(Ok(()),|app|super::huddle_effects::stage_roster(app,e.room_id))),
        campfire_db::models::huddle_effects::RoleEvent::KIND => decode::<campfire_db::models::huddle_effects::RoleEvent>(request).and_then(|e|app.map_or(Ok(()),|app|super::huddle_effects::role_event(app,e.room_id,e.membership_id))),
        campfire_db::models::huddle_effects::Presence::KIND => decode::<campfire_db::models::huddle_effects::Presence>(request).and_then(|effect| {
            app.map_or(Ok(()), |app| super::huddle_effects::presence(app, effect.room_id))
        }),
        campfire_db::models::scheduled_message::ScheduledMessageChange::KIND => decode(request).map(|change| {
            if let Some(app) = app { app.broadcasts.sync_scheduled(change); }
        }),
        campfire_db::models::membership::PresentRead::KIND => decode(request).map(|read: campfire_db::models::membership::PresentRead| {
            if let Some(app) = app {
                app.broadcasts.sync_read_row(read.user_id, read.room_id);
            }
        }),
        campfire_db::models::room_category::SidebarOrganized::KIND => {
            decode(request).map(|change| {
                if let Some(app) = app {
                    app.broadcasts.sync_organized(change);
                }
            })
        }
        campfire_db::models::poll::PollChanged::KIND => decode(request).map(|change| {
            if let Some(app) = app {
                app.broadcasts.sync_poll(change);
            }
        }),
        campfire_db::models::calendar_event::EventsChanged::KIND => decode(request).map(|change| {
            if let Some(app) = app {
                app.broadcasts.sync_events_changed(change);
            }
        }),
        campfire_db::models::activity_item::ActivityItemTouched::KIND => decode::<
            campfire_db::models::activity_item::ActivityItemTouched,
        >(request)
        .map(|touched| {
            if let Some(app) = app {
                app.broadcasts
                    .sync_activity_item(touched.user_id, touched.id);
            }
        }),
        campfire_db::models::activity_item::ActivityItemsRemoved::KIND => decode::<campfire_db::models::activity_item::ActivityItemsRemoved>(request).map(|removed| {
            if let Some(app) = app { app.broadcasts.sync_activity_removed(removed); }
        }),
        RoomRemovalBroadcast::KIND => decode(request).map(|broadcast| room_removal(cable, app, &broadcast, app.map_or_else(||huddle_configured(env),|app|app.config.huddle.configured()))),
        campfire_db::broadcasts::Broadcast::KIND => decode(request)
            .and_then(|change| messaging(cable, app, &change)),
        campfire_db::models::user_status_settings::updates::StatusBadgeBroadcast::KIND =>
            decode(request).and_then(|broadcast| status_badge(cable, broadcast)),
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
            messaging(cable, Some(app), &campfire_db::broadcasts::Broadcast::MessageUpdated { message_id: event.message_id })
        }),
        crate::integrations::github::notifier::MessageCreated::KIND => decode(request).and_then(|broadcast| {
            let app = app.ok_or_else(|| anyhow::anyhow!("app has not booted"))?;
            super::github_notifier::publish(app, &broadcast)
        }),
        crate::integrations::github::pull_requests::CardUpdated::KIND => decode(request).and_then(|broadcast| {
            let app = app.ok_or_else(|| anyhow::anyhow!("app has not booted"))?;
            super::github_cards::publish(app, &broadcast)
        }),
        campfire_db::models::agent_step::StepParentChange::KIND => {
            decode(request).and_then(|broadcast| agent_steps(app, &broadcast))
        }
        campfire_db::models::agent::AgentSyncChange::KIND => decode::<campfire_db::models::agent::AgentSyncChange>(request).map(|change| {
            if let Some(app) = app { app.broadcasts.sync_agent_status(change.agent_id); }
        }),
        campfire_db::models::channel_thread::ThreadWorkChange::KIND => decode::<campfire_db::models::channel_thread::ThreadWorkChange>(request).map(|change| {
            if let Some(app) = app { app.broadcasts.thread_updated(change.thread_id); }
        }),
        campfire_db::models::channel_thread::ThreadBoardCreation::KIND => decode::<campfire_db::models::channel_thread::ThreadBoardCreation>(request).map(|change| {
            if let Some(app) = app { app.broadcasts.thread_created(change.thread_id); }
        }),
        campfire_db::models::agent_approval::ApprovalChange::KIND => decode::<campfire_db::models::agent_approval::ApprovalChange>(request).map(|change| {
            if let Some(app) = app { app.broadcasts.sync_approval(change.approval_id); }
        }),
        kind => Err(anyhow::anyhow!("no handler for the {kind} broadcast")),
    };
    if let Err(error) = result {
        tracing::warn!(kind = request.kind, %error, "broadcast failed");
    }
}

fn agent_steps(app: Option<&App>, change: &campfire_db::models::agent_step::StepParentChange) -> anyhow::Result<()> {
    if let Some(app) = app {
        app.broadcasts.sync_agent_steps(change.message_id, change.thread_id);
    }
    Ok(())
}



fn status_badge(cable: &Cable, b: campfire_db::models::user_status_settings::updates::StatusBadgeBroadcast) -> anyhow::Result<()> {
    crate::cable::sync::status_badge(cable, b.user_id, &b.presence, b.status_text.as_deref());
    Ok(())
}



/// Domain changes leave the writer after commit in callback order. Reader work that takes
/// room locks stays deferred, so callbacks never wait for a lock while holding the writer.
pub(crate) fn messaging(cable: &Cable, app: Option<&App>, change: &campfire_db::broadcasts::Broadcast) -> anyhow::Result<()> {
    use campfire_db::broadcasts::Broadcast;
    match change {
        Broadcast::Cable { stream, payload } => {
            cable.broadcast(stream, payload);
            crate::cable::sync::cable_stream(cable, stream, payload);
            if let Some(app) = app { app.broadcasts.sync_activity_stream(stream, payload); }
        }
        Broadcast::UnreadRoom { user_id, room_id, message_id } => {
            cable.broadcast(&campfire_db::broadcasts::unread_rooms_stream_name(*user_id), &serde_json::json!({"roomId":room_id}));
            crate::cable::sync::unread_room(cable, app.map(|app| &app.db), *user_id, *room_id, *message_id);
            if let (Some(app), Some(_)) = (app, message_id) { app.broadcasts.sync_unread_rows(*room_id, vec![*user_id]); }
        }
        Broadcast::MessageCreated { message_id } | Broadcast::MessageUpdated { message_id } => {
            let app = app.ok_or_else(|| anyhow::anyhow!("app not booted"))?;
            app.db.read_blocking(|conn| {
                let message = campfire_db::Message::find(conn, *message_id)?;
                if !app.broadcasts.sync_message_checked(conn, &message, matches!(change, Broadcast::MessageCreated { .. })) {
                    return Err(campfire_db::Error::Other("message JSON publication failed".into()));
                }
                Ok(())
            })?;
        }
        Broadcast::MessageCards { message_id } | Broadcast::MessagePinned { message_id } => {
            let app = app.ok_or_else(|| anyhow::anyhow!("app not booted"))?;
            app.db.read_blocking(|conn| {
                if matches!(change, Broadcast::MessagePinned { .. }) {
                    crate::cable::sync::message_pinned(cable, conn, *message_id);
                } else if let Some(message) = campfire_db::Message::find_by_id(conn, *message_id)? {
                    app.broadcasts.sync_message_cards(conn, std::slice::from_ref(&message));
                }
                Ok(())
            })?;
        }
        Broadcast::ThreadIndicator { message_id, .. } => {
            if let Some(app) = app { app.broadcasts.sync_thread_indicator(*message_id); }
        }
        Broadcast::ThreadRemoved { thread_id, room_id } => {
            if let Some(app) = app { app.broadcasts.thread_removed(*thread_id, *room_id); }
        }
        Broadcast::MembershipChanged { membership_id } => {
            if let Some(app) = app { app.broadcasts.sync_membership_row(*membership_id); }
        }
        Broadcast::UserStatus { user_id } => {
            if let Some(app) = app {
                app.db.read_blocking(|conn| {
                    let status = crate::controllers::presenters::status_settings::profile_status(conn, &app.secrets, *user_id, *user_id, app.db.env().now())?;
                    crate::cable::sync::status_badge(cable, *user_id, &status.presence, status.status_text.as_deref());
                    Ok(())
                })?;
            }
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
pub fn room_removal(cable: &Cable, app: Option<&App>, broadcast: &RoomRemovalBroadcast, _huddle_configured: bool) {
    if let Some(app) = app && cable.sync_wanted() {
        app.broadcasts.sync_row_removed(broadcast.user_id, broadcast.room_id);
    }
}

/// `Huddle.configured?` (reference/app/services/huddle.rb): the five LiveKit variables are set and
/// the public and internal LiveKit URLs name different endpoints. Read on every call, as Rails
/// reads `ENV` on every call. (The huddle workstream owns the rest of `Huddle`.)
pub fn huddle_configured(env: impl Fn(&str) -> Option<String>) -> bool {
    crate::huddle::Config::from_lookup(env).configured()
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
        let vectors: serde_json::Value = serde_json::from_str(include_str!("../../../campfire/src/huddle/protocol_vectors.json")).unwrap();
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
