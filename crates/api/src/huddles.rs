//! The S5 huddle endpoints: who's in which call, joining and leaving a room's call, and call
//! moderation (`campfire_api_types`' `huddle` module documents each one). They run the classic
//! actions' writes (`HuddleGrant::issue`, `mark_out_of_call`, `call_moderation::moderate`), so
//! the grants, the LiveKit tokens, the gateway's checks and the classic broadcasts are unchanged.

use campfire_api_types as api;
use campfire_app::app::AppCtx;
use campfire_app::cable::huddle_sync;
use campfire_db::models::call_moderation::{self, Action, Denial};
use campfire_db::models::huddle_grant::{HuddleGrant, IN_CALL_WINDOW};
use campfire_db::models::room_delete::HuddleConfig;
use campfire_db::{Connection, Membership, Room, RoomType, Timestamp, User};
use campfire_kit::{Ctx, Error, Result, StatusCode};
use campfire_runtime::concerns::{self, AuthenticatedBy, Authentication, Before};
use campfire_runtime::context::db_error;
use rails_compat::jwt::livekit;
use serde::de::DeserializeOwned;

use crate::dto;
use crate::error::{fail, not_found};

/// The largest request body read; these bodies are a few fields.
const BODY_LIMIT: usize = 1 << 16;

endpoint!(
    /// `GET /api/v1/huddles`
    index => index_huddles
);
endpoint!(
    /// `GET /api/v1/rooms/:room_id/huddle`
    show => show_huddle
);
endpoint!(
    /// `POST /api/v1/rooms/:room_id/huddle`
    join => create_huddle
);
endpoint!(
    /// `POST /api/v1/rooms/:room_id/huddle/leave`
    leave => leave_huddle
);
endpoint!(
    /// `POST /api/v1/rooms/:room_id/huddle/moderation`
    moderate => moderate_huddle
);

/// The classic pages' before-actions, answering a signed-out caller with a 401.
pub(crate) async fn before_actions(c: &mut Ctx) -> Result<()> {
    concerns::before_actions(
        c,
        Before {
            authentication: Authentication::JsonUnauthorized,
            ..Before::default()
        },
    )
    .await
}

/// `set_room`, of an alive room (`RoomScoped` with `Room.alive`).
pub(crate) async fn set_room(c: &mut Ctx) -> Result<(Membership, Room)> {
    let (membership, room) = concerns::set_room(c).await?;
    if room.deleted_at.is_some() {
        return Err(Error::NotFound);
    }
    Ok((membership, room))
}

/// The JSON body as `T`; anything else is a 422.
pub(crate) async fn body<T: DeserializeOwned>(c: &mut Ctx) -> Result<T> {
    let bytes = c.read_body(BODY_LIMIT).await;
    serde_json::from_slice(&bytes).map_err(|error| {
        fail(
            c,
            api::ApiError::Validation {
                message: format!("The request body isn't valid: {error}"),
                fields: Default::default(),
            },
        )
    })
}

pub(crate) fn forbidden(message: &str) -> api::ApiError {
    api::ApiError::Forbidden {
        message: message.into(),
    }
}

/// A 422 with the classic message as it is.
pub(crate) fn invalid(field: &str, message: &str) -> api::ApiError {
    api::ApiError::Validation {
        message: message.into(),
        fields: [(field.to_string(), vec![message.to_string()])].into(),
    }
}

fn deny_bot_keys(c: &mut Ctx) -> Result<()> {
    if concerns::authenticated_by(c) == AuthenticatedBy::BotKey {
        return Err(fail(c, forbidden("Bots cannot join huddles")));
    }
    Ok(())
}

/// `Rooms::HuddlesController`'s before-actions: signed in (not with a bot key), huddles
/// configured (503 otherwise), an active human.
async fn before_huddle(c: &mut Ctx) -> Result<User> {
    concerns::before_actions_with_authentication(
        c,
        Before {
            authentication: Authentication::JsonUnauthorized,
            ..Before::default()
        },
        None,
        Some(deny_bot_keys),
    )
    .await?;
    if !c.app().config.huddle.configured() {
        let unavailable = api::ApiError::Unavailable {
            message: "Huddles are not configured".into(),
        };
        return Err(fail(c, unavailable));
    }
    let user = concerns::require_current_user(c)?.clone();
    if user.is_bot() {
        return Err(fail(c, forbidden("Bots cannot join huddles")));
    }
    if !user.is_active() {
        return Err(fail(c, forbidden("User cannot join huddles")));
    }
    Ok(user)
}

fn room_not_found(c: &mut Ctx) -> Error {
    let error = api::ApiError::NotFound {
        message: "Room not found or inaccessible".into(),
    };
    fail(c, error)
}

/// The huddle's `set_room`: an alive room the viewer belongs to.
async fn huddle_room(c: &mut Ctx) -> Result<(Membership, Room)> {
    match set_room(c).await {
        Err(Error::NotFound) => Err(room_not_found(c)),
        scope => scope,
    }
}

fn session_id(c: &Ctx) -> Result<i64> {
    concerns::current_session(c)
        .map(|session| session.id)
        .ok_or_else(|| Error::internal(std::io::Error::other("no Current.session")))
}

/// The room's name as the viewer sees it (`room_json`).
fn room_name(conn: &Connection, room: &Room, viewer: &User) -> campfire_db::Result<String> {
    let name = if room.direct() {
        room.direct_display_name(conn, Some(viewer), None)?
    } else {
        room.name.clone()
    };
    Ok(name.unwrap_or_default())
}

/// `Users::HuddlePresenceController#show`: the live calls in the viewer's rooms.
async fn index_huddles(c: &mut Ctx) -> Result {
    let viewer = before_huddle(c).await?;
    let (secrets, now) = (c.app().secrets.clone(), Timestamp::from_jiff(c.now()));
    let list = c
        .app()
        .db
        .read(move |conn| {
            let since = now.ago(jiff::SignedDuration::from_secs(IN_CALL_WINDOW));
            let room_ids: Vec<i64> = conn
                .prepare_cached(
                    "SELECT DISTINCT room_id FROM huddle_grants WHERE revoked_at IS NULL AND last_seen_at>? \
                     AND room_id IN (SELECT room_id FROM memberships WHERE user_id=?) ORDER BY room_id",
                )?
                .query_map(rusqlite::params![since, viewer.id], |row| row.get(0))?
                .collect::<rusqlite::Result<_>>()?;
            let mut rooms = Vec::with_capacity(room_ids.len());
            for room_id in room_ids {
                let Some(room) = Room::find_by_id(conn, room_id)? else {
                    continue;
                };
                let presence = huddle_sync::presence(conn, &room, now)?;
                if !presence.participants.is_empty() {
                    rooms.push(presence);
                }
            }
            let ids = rooms
                .iter()
                .flat_map(|room| room.participants.iter().map(|person| person.user_id));
            let users = dto::users(conn, &secrets, ids.collect::<Vec<_>>(), now)?;
            Ok(api::HuddlePresenceList { rooms, users })
        })
        .await
        .map_err(db_error)?;
    c.json(StatusCode::OK, &list)
}

/// `Rooms::HuddlesController#show` and `#participants`.
async fn show_huddle(c: &mut Ctx) -> Result {
    let viewer = before_huddle(c).await?;
    let (_, room) = huddle_room(c).await?;
    let (secrets, now) = (c.app().secrets.clone(), Timestamp::from_jiff(c.now()));
    let detail = c
        .app()
        .db
        .read(move |conn| {
            let presence = huddle_sync::presence(conn, &room, now)?;
            let ids: Vec<i64> = presence.participants.iter().map(|p| p.user_id).collect();
            Ok(api::HuddleDetail {
                room_name: room_name(conn, &room, &viewer)?,
                users: dto::users(conn, &secrets, ids, now)?,
                presence,
            })
        })
        .await
        .map_err(db_error)?;
    c.json(StatusCode::OK, &detail)
}

/// `Rooms::HuddlesController#create`: a grant and its LiveKit token.
async fn create_huddle(c: &mut Ctx) -> Result {
    let viewer = before_huddle(c).await?;
    let (membership, room) = huddle_room(c).await?;
    let config = c.app().config.huddle.clone();
    let domain = HuddleConfig {
        api_secret: config.api_secret.clone(),
        admin_configured: config.admin_configured(),
    };
    let session_id = session_id(c)?;
    let room_id = room.id;
    let grant = match c
        .app()
        .db
        .write(move |tx| HuddleGrant::issue(tx, session_id, membership.id, room_id, &domain))
        .await
    {
        Ok(grant) => grant,
        Err(campfire_db::Error::Other(message)) if message == "HuddleGrant::Ineligible" => {
            return Err(room_not_found(c));
        }
        Err(error) => return Err(db_error(error)),
    };
    let can_publish = livekit::can_publish(
        grant.server_muted,
        room.room_type == RoomType::Stage,
        grant.stage_role.as_deref(),
    );
    // `configured()` checked both above.
    let token = livekit::participant_token(
        config.api_key.as_deref().unwrap_or_default(),
        config.api_secret.as_deref().unwrap_or_default(),
        &livekit::Participant {
            name: viewer.display_name(),
            identity: &grant.identity,
            room_name: &grant.room_name,
            can_publish,
        },
        c.now().as_second(),
    );
    let room_name = {
        let room = room.clone();
        c.app()
            .db
            .read(move |conn| room_name(conn, &room, &viewer))
            .await
            .map_err(db_error)?
    };
    let credentials = api::HuddleCredentials {
        url: config.public_url.clone().unwrap_or_default(),
        token,
        identity: grant.identity,
        grant_id: grant.id,
        room_id: room.id,
        room_name,
        can_publish,
    };
    c.json(StatusCode::OK, &credentials)
}

/// `Rooms::HuddlesController#leave`: this session's grants in the room leave the call.
async fn leave_huddle(c: &mut Ctx) -> Result {
    before_huddle(c).await?;
    let (_, room) = huddle_room(c).await?;
    let session_id = session_id(c)?;
    c.app()
        .db
        .write(move |tx| {
            let ids: Vec<i64> = tx
                .conn()
                .prepare_cached(
                    "SELECT id FROM huddle_grants WHERE session_id=? AND room_id=? AND revoked_at IS NULL ORDER BY id",
                )?
                .query_map(rusqlite::params![session_id, room.id], |row| row.get(0))?
                .collect::<rusqlite::Result<_>>()?;
            for id in ids {
                if let Some(mut grant) = HuddleGrant::find_by_id(tx.conn(), id)? {
                    grant.mark_out_of_call(tx, None)?;
                }
            }
            Ok(())
        })
        .await
        .map_err(db_error)?;
    Ok(c.head(StatusCode::NO_CONTENT))
}

/// `Rooms::CallModerationController#mute`, `#unmute` and `#disconnect`.
async fn moderate_huddle(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let (viewer, room) = set_room(c).await?;
    let input: api::ModerateHuddle = body(c).await?;
    let action = match input.action {
        api::HuddleModeration::Mute => Action::Mute,
        api::HuddleModeration::Unmute => Action::Unmute,
        api::HuddleModeration::Disconnect => Action::Disconnect,
    };
    let config = HuddleConfig {
        api_secret: c.app().config.huddle.api_secret.clone(),
        admin_configured: c.app().config.huddle.admin_configured(),
    };
    let (room_id, user_id, target) = (room.id, viewer.user_id, input.membership_id);
    let result = c
        .app()
        .db
        .write(move |tx| {
            call_moderation::moderate(tx, room_id, user_id, Some(target), action, &config)
        })
        .await
        .map_err(db_error)?;
    let error = match result {
        Ok(()) => return Ok(c.head(StatusCode::NO_CONTENT)),
        Err(Denial::NotFound | Denial::TargetNotFound) => not_found(),
        Err(Denial::Forbidden) => {
            forbidden("Only administrators and stage hosts can moderate calls")
        }
        Err(Denial::AdministratorRank) => {
            forbidden("Only administrators can moderate an administrator")
        }
        Err(Denial::SelfTarget) => {
            invalid("membershipId", "You cannot moderate your own call session")
        }
    };
    Err(fail(c, error))
}
