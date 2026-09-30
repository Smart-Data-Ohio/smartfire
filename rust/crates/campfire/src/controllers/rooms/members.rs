//! `app/controllers/rooms/members_controller.rb`: a live, viewer-specific JSON poll.
use campfire_db::{
    Timestamp,
    models::{room_members, workspace_presence_lease::Presence},
};
use campfire_kit::{Ctx, Result, StatusCode};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::controllers::presenters::{avatar_path, page::db_error};
use crate::{
    app::AppCtx,
    concerns::{self, Authentication, Before},
};

#[derive(Serialize)]
struct Members {
    members: Vec<Member>,
}

// Field order is observable: Rails builds this hash in precisely this order.
#[derive(Serialize)]
struct Member {
    id: i64,
    name: String,
    avatar_url: String,
    bot: bool,
    online: bool,
    presence: String,
    status: Option<String>,
    starred: bool,
}

pub async fn index(c: &mut Ctx) -> Result {
    concerns::before_actions(
        c,
        Before {
            authentication: Authentication::JsonUnauthorized,
            ..Before::default()
        },
    )
    .await?;
    // MembersController rescues RoomScoped RecordNotFound with an empty 404.
    let room = match concerns::set_room(c).await {
        Ok((_, room)) if room.deleted_at.is_none() => room,
        Ok(_) | Err(campfire_kit::Error::NotFound) => {
            return Ok(concerns::head(StatusCode::NOT_FOUND));
        }
        Err(error) => return Err(error),
    };
    let viewer = concerns::require_current_user(c)?.id;
    let now = Timestamp::from_jiff(c.now());
    let members = c
        .app()
        .db
        .read(move |conn| room_members::for_room(conn, room.id, viewer, now))
        .await
        .map_err(db_error)?;
    let members = members
        .into_iter()
        .map(|member| {
            let (presence, status) = if let Some(agent) = member.agent {
                let live = agent.suspended_at.is_none() && agent.last_seen_at.is_some();
                let status = agent
                    .working_presence_text(now)
                    .or_else(|| {
                        agent
                            .status_note
                            .as_deref()
                            .filter(|s| !campfire_richtext::ruby::is_blank(s))
                    })
                    .map(str::to_owned)
                    .unwrap_or_else(|| {
                        let mut letters = agent.status.chars();
                        letters
                            .next()
                            .map(|first| first.to_uppercase().to_string() + letters.as_str())
                            .unwrap_or_default()
                    });
                (
                    if live { "agent" } else { "offline" }.to_owned(),
                    Some(status),
                )
            } else {
                let presence = match member.settings.effective_presence(member.lease) {
                    Presence::Online => "online",
                    Presence::Idle => "idle",
                    Presence::Offline => "offline",
                    Presence::Dnd => "dnd",
                };
                (presence.into(), member.settings.status_text_display(now))
            };
            Member {
                id: member.user.id,
                name: member.user.name.clone(),
                avatar_url: c.url_for(&avatar_path(&c.app().secrets, &member.user)),
                bot: member.user.is_bot(),
                online: presence != "offline",
                presence,
                status,
                starred: member.starred,
            }
        })
        .collect();
    c.no_store();
    c.set_header("cache-control", "no-store");
    c.set_header("pragma", "no-cache");
    // Pinned Rails' render-json encoder leaves HTML entities and JS separators raw.
    let body =
        serde_json::to_string(&Members { members }).map_err(campfire_kit::Error::internal)?;
    // Rack::ETag hashes the raw JSON even with no-store. The page cache's fragment
    // digest is different. Rack::ConditionalGet then compares the exact validator.
    let digest = format!("{:x}", Sha256::digest(body.as_bytes()));
    c.set_header("etag", &format!("W/\"{}\"", &digest[..32]));
    Ok(c.render_as(StatusCode::OK, "application/json; charset=utf-8", body))
}
