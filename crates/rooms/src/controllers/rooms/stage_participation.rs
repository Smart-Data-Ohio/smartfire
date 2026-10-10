use crate::controllers::presenters::page::db_error;
use crate::{
    app::AppCtx,
    concerns::{self, Before, before_actions, cast_integer},
};
use campfire_db::models::{
    room_delete::HuddleConfig,
    stage_participation::{self, Denial, HandTarget},
};
use campfire_db::{Membership, Room, StageRole};
use campfire_kit::{Ctx, Result, StatusCode};

async fn scope(
    c: &mut Ctx,
) -> Result<std::result::Result<(Membership, Room), campfire_kit::Response>> {
    before_actions(c, Before::default()).await?;
    match concerns::set_room(c).await {
        Ok((member, room)) if room.stage() => Ok(Ok((member, room))),
        Ok(_) | Err(campfire_kit::Error::NotFound) => {
            Ok(Err(concerns::head(StatusCode::NOT_FOUND)))
        }
        Err(error) => Err(error),
    }
}
pub async fn role(c: &mut Ctx) -> Result {
    let (member, room) = match scope(c).await? {
        Ok(scope) => scope,
        Err(response) => return Ok(response),
    };
    let room_id = room.id;
    let user_id = member.user_id;
    let target = c.param_str("membership_id").and_then(cast_integer);
    let role = c
        .param("stage_role")
        .and_then(|p| p.to_s())
        .unwrap_or_default();
    let config = HuddleConfig {
        api_secret: c.app().config.huddle.api_secret.clone(),
        admin_configured: c.app().config.huddle.admin_configured(),
    };
    let result = c
        .app()
        .db
        .write(move |tx| {
            stage_participation::change_role(tx, room_id, user_id, target, &role, &config)
        })
        .await;
    let result = match result {
        Err(campfire_db::Error::RecordInvalid(error)) => {
            return Ok(c.render_as(
                StatusCode::UNPROCESSABLE_ENTITY,
                "text/plain; charset=utf-8",
                campfire_views::helpers::to_sentence(&error.full_messages(), " and "),
            ));
        }
        Err(error) => return Err(db_error(error)),
        Ok(result) => result,
    };
    if let Err(denial) = result {
        return deny(c, denial);
    }
    respond(c, room_id, member.id, "roster", "stage_roster").await
}
pub async fn raise(c: &mut Ctx) -> Result {
    let (member, room) = match scope(c).await? {
        Ok(scope) => scope,
        Err(response) => return Ok(response),
    };
    if member.stage_role != Some(StageRole::Listener) {
        return deny(c, Denial::ListenerOnly);
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
    if count > 10 {
        return Ok(c.render_as(
            StatusCode::TOO_MANY_REQUESTS,
            "text/plain; charset=utf-8",
            "Slow down and try again",
        ));
    }
    let room_id = room.id;
    let user_id = member.user_id;
    if let Err(denial) = c
        .app()
        .db
        .write(move |tx| stage_participation::raise_hand(tx, room_id, user_id))
        .await
        .map_err(db_error)?
    {
        return deny(c, denial);
    }
    respond(c, room_id, member.id, "controls", "stage_controls").await
}
pub async fn lower(c: &mut Ctx) -> Result {
    let (member, room) = match scope(c).await? {
        Ok(scope) => scope,
        Err(response) => return Ok(response),
    };
    let target = match c.param("membership_id").filter(|p| !p.is_blank()) {
        Some(p) => HandTarget::Other(p.to_s().as_deref().and_then(cast_integer)),
        None => HandTarget::Own,
    };
    let room_id = room.id;
    let user_id = member.user_id;
    if let Err(denial) = c
        .app()
        .db
        .write(move |tx| stage_participation::lower_hand(tx, room_id, user_id, target))
        .await
        .map_err(db_error)?
    {
        return deny(c, denial);
    }
    respond(c, room_id, member.id, "controls", "stage_controls").await
}
fn deny(c: &mut Ctx, denial: Denial) -> Result {
    let (status, message) = match denial {
        Denial::NotFound => return Ok(concerns::head(StatusCode::NOT_FOUND)),
        Denial::TargetNotFound => return Ok(c.head(StatusCode::NOT_FOUND)),
        Denial::Forbidden => return Ok(concerns::head(StatusCode::FORBIDDEN)),
        Denial::PlainForbidden(message) => (StatusCode::FORBIDDEN, message),
        Denial::UnknownRole => (StatusCode::UNPROCESSABLE_ENTITY, "Unknown stage role"),
        Denial::ListenerOnly => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "Only listeners can raise a hand",
        ),
    };
    Ok(c.render_as(status, "text/plain; charset=utf-8", message))
}
async fn respond(c: &mut Ctx, room_id: i64, _member_id: i64, _partial: &str, _target: &str) -> Result {
    c.redirect_to(&c.url_for(&campfire_routes::room(room_id)))
}
