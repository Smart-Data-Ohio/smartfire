//! The S5 stage endpoints: the roster and stream, roles, raised hands, and going live
//! (`campfire_api_types`' `stage` module documents each one). The writes are the classic
//! controllers' (`stage_participation`, `stage_streams`), with their policy and broadcasts.

use campfire_api_types as api;
use campfire_app::app::AppCtx;
use campfire_app::cable::huddle_sync;
use campfire_db::models::room_delete::HuddleConfig;
use campfire_db::models::stage_participation::{self, HandTarget};
use campfire_db::models::stage_streams;
use campfire_db::{Membership, Room, StageRole, Timestamp};
use campfire_kit::{Ctx, Error, Result, StatusCode};
use campfire_presentation::helpers::to_sentence;
use campfire_runtime::concerns;
use campfire_runtime::context::db_error;

use crate::dto;
use crate::error::{fail, not_found};
use crate::huddles::{before_actions, body, forbidden, invalid, set_room};

/// More hand raises than this in a minute are refused (`Rooms::Stage::HandsController`).
const HAND_RAISES_PER_MINUTE: u64 = 10;

endpoint!(
    /// `GET /api/v1/rooms/:room_id/stage`
    show => show_stage
);
endpoint!(
    /// `PATCH /api/v1/rooms/:room_id/stage/members/:membership_id`
    change_role => update_role
);
endpoint!(
    /// `POST /api/v1/rooms/:room_id/stage/hand`
    raise_hand => create_hand
);
endpoint!(
    /// `DELETE /api/v1/rooms/:room_id/stage/hand`
    lower_hand => destroy_hand
);
endpoint!(
    /// `POST /api/v1/rooms/:room_id/stage/stream`
    start_stream => create_stream
);
endpoint!(
    /// `DELETE /api/v1/rooms/:room_id/stage/stream`
    stop_stream => destroy_stream
);

/// The stage controllers' scope: signed in, a member of an alive stage room.
async fn scope(c: &mut Ctx) -> Result<(Membership, Room)> {
    before_actions(c).await?;
    match set_room(c).await? {
        (member, room) if room.stage() => Ok((member, room)),
        _ => Err(Error::NotFound),
    }
}

fn huddle_config(c: &Ctx) -> HuddleConfig {
    HuddleConfig {
        api_secret: c.app().config.huddle.api_secret.clone(),
        admin_configured: c.app().config.huddle.admin_configured(),
    }
}

/// The roster and stream after a change.
async fn state(c: &mut Ctx, room_id: i64) -> Result {
    let stage = c
        .app()
        .db
        .read(move |conn| huddle_sync::stage(conn, room_id))
        .await
        .map_err(db_error)?;
    c.json(StatusCode::OK, &stage)
}

fn participation_denial(denial: stage_participation::Denial) -> api::ApiError {
    use stage_participation::Denial;
    match denial {
        Denial::NotFound | Denial::TargetNotFound => not_found(),
        Denial::Forbidden => forbidden("Only hosts and administrators can change stage roles"),
        Denial::PlainForbidden(message) => forbidden(message),
        Denial::UnknownRole => invalid("role", "Unknown stage role"),
        Denial::ListenerOnly => invalid("role", "Only listeners can raise a hand"),
    }
}

fn stream_denial(denial: stage_streams::Denial) -> api::ApiError {
    use stage_streams::Denial;
    match denial {
        Denial::NotFound => not_found(),
        Denial::Forbidden(message) => forbidden(message),
        Denial::UnknownQuality => invalid("quality", "Unknown stream quality"),
        Denial::AlreadyLive(name) => api::ApiError::Conflict {
            message: format!("{name} is already live"),
        },
    }
}

/// The stage panel's data: the roster, the stream and every member's directory entry.
async fn show_stage(c: &mut Ctx) -> Result {
    let (_, room) = scope(c).await?;
    let (secrets, now) = (c.app().secrets.clone(), Timestamp::from_jiff(c.now()));
    let detail = c
        .app()
        .db
        .read(move |conn| {
            let stage = huddle_sync::stage(conn, room.id)?;
            let ids: Vec<i64> = stage.members.iter().map(|member| member.user_id).collect();
            Ok(api::StageDetail {
                users: dto::users(conn, &secrets, ids, now)?,
                stage,
            })
        })
        .await
        .map_err(db_error)?;
    c.json(StatusCode::OK, &detail)
}

/// `Rooms::Stage::RolesController#update`.
async fn update_role(c: &mut Ctx) -> Result {
    let (member, room) = scope(c).await?;
    let target = c
        .param_str("membership_id")
        .and_then(concerns::cast_integer);
    let input: api::ChangeStageRole = body(c).await?;
    let role = match input.role {
        api::StageRole::Listener => StageRole::Listener,
        api::StageRole::Speaker => StageRole::Speaker,
        api::StageRole::Host => StageRole::Host,
    };
    let config = huddle_config(c);
    let (room_id, user_id) = (room.id, member.user_id);
    let result = c
        .app()
        .db
        .write(move |tx| {
            stage_participation::change_role(tx, room_id, user_id, target, role.name(), &config)
        })
        .await;
    let error = match result {
        Ok(Ok(())) => return state(c, room_id).await,
        Ok(Err(denial)) => participation_denial(denial),
        Err(campfire_db::Error::RecordInvalid(errors)) => {
            invalid("role", &to_sentence(&errors.full_messages(), " and "))
        }
        Err(error) => return Err(db_error(error)),
    };
    Err(fail(c, error))
}

/// `Rooms::Stage::HandsController#create`, with its rate limit.
async fn create_hand(c: &mut Ctx) -> Result {
    let (member, room) = scope(c).await?;
    if member.stage_role != Some(StageRole::Listener) {
        let error = participation_denial(stage_participation::Denial::ListenerOnly);
        return Err(fail(c, error));
    }
    let now = c.clock().now();
    let key = format!(
        "stage_hand_raise/{}/{}/{}",
        room.id,
        member.id,
        now.as_second() / 60
    );
    // Rails' null test store returns nil; the production process shares one expiring store.
    let count = if c.app().config.environment == "test" {
        0
    } else {
        c.kit()
            .rate_limits()
            .increment(&key, jiff::SignedDuration::from_secs(60), now)
    };
    if count > HAND_RAISES_PER_MINUTE {
        let error = api::ApiError::RateLimited {
            message: "Slow down and try again".into(),
            retry_after: u32::try_from(60 - now.as_second().rem_euclid(60)).unwrap_or(60),
        };
        return Err(fail(c, error));
    }
    let (room_id, user_id) = (room.id, member.user_id);
    let result = c
        .app()
        .db
        .write(move |tx| stage_participation::raise_hand(tx, room_id, user_id))
        .await
        .map_err(db_error)?;
    match result {
        Ok(()) => state(c, room_id).await,
        Err(denial) => Err(fail(c, participation_denial(denial))),
    }
}

/// `Rooms::Stage::HandsController#destroy`: the viewer's own hand, or with `?membershipId=`
/// someone else's.
async fn destroy_hand(c: &mut Ctx) -> Result {
    let (member, room) = scope(c).await?;
    let target = match c
        .param_str("membershipId")
        .filter(|id| !id.trim().is_empty())
    {
        Some(id) => HandTarget::Other(concerns::cast_integer(id)),
        None => HandTarget::Own,
    };
    let (room_id, user_id) = (room.id, member.user_id);
    let result = c
        .app()
        .db
        .write(move |tx| stage_participation::lower_hand(tx, room_id, user_id, target))
        .await
        .map_err(db_error)?;
    match result {
        Ok(()) => state(c, room_id).await,
        Err(denial) => Err(fail(c, participation_denial(denial))),
    }
}

/// `Rooms::Stage::StreamsController#create`: go live.
async fn create_stream(c: &mut Ctx) -> Result {
    let (member, room) = scope(c).await?;
    let input: api::StartStageStream = body(c).await?;
    let quality = huddle_sync::stream_quality_name(input.quality);
    let (room_id, user_id) = (room.id, member.user_id);
    let result = c
        .app()
        .db
        .write(move |tx| stage_streams::start(tx, room_id, user_id, quality))
        .await
        .map_err(db_error)?;
    let stream = match result {
        Ok(stream) => stream,
        Err(denial) => return Err(fail(c, stream_denial(denial))),
    };
    let live = c
        .app()
        .db
        .read(move |conn| huddle_sync::stream(conn, &stream))
        .await
        .map_err(db_error)?;
    match live {
        Some(live) => c.json(StatusCode::CREATED, &live),
        None => Err(Error::internal(std::io::Error::other(
            "a new stream has a quality the wire doesn't know",
        ))),
    }
}

/// `Rooms::Stage::StreamsController#destroy`: end the live stream, or with `?streamId=` only
/// that one.
async fn destroy_stream(c: &mut Ctx) -> Result {
    let (member, room) = scope(c).await?;
    let requested = c
        .param_str("streamId")
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(str::to_owned);
    let (room_id, user_id) = (room.id, member.user_id);
    let result = c
        .app()
        .db
        .write(move |tx| stage_streams::stop(tx, room_id, user_id, requested.as_deref()))
        .await
        .map_err(db_error)?;
    match result {
        Ok(()) => Ok(c.head(StatusCode::NO_CONTENT)),
        Err(denial) => Err(fail(c, stream_denial(denial))),
    }
}
