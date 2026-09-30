//! `Rooms::GithubSubscriptionsController`: member/creator authorization and transport only.
use crate::{
    app::AppCtx,
    concerns::{self, Before, before_actions, cast_integer},
    controllers::presenters::page::db_error,
    integrations::github::subscriptions::{
        self, RepositorySubscription, SubscribeInput, SubscribeOutcome,
    },
};
use campfire_db::{Errors, Room, User};
use campfire_kit::{
    Ctx, Error, Param, Redirect, Result, StatusCode,
    params::{ParamMap, Permit},
};
use serde_json::Value;

async fn before(c: &mut Ctx) -> Result<(Room, User)> {
    before_actions(c, Before::default()).await?;
    let (_, room) = concerns::set_room(c).await?;
    if room.direct() {
        return Err(Error::Halt(Box::new(concerns::head(StatusCode::NOT_FOUND))));
    }
    let user = concerns::require_current_user(c)?.clone();
    if !user.can_administer(Some(room.creator_id), false) {
        return Err(Error::Halt(Box::new(concerns::head(StatusCode::FORBIDDEN))));
    }
    Ok((room, user))
}
fn subscription_params(c: &Ctx) -> Result<ParamMap> {
    Ok(c.params
        .require("github_repository_subscription")?
        .permit(&[
            Permit::from("full_name"),
            Permit::from("skip_access_check"),
            Permit::ScalarArray("events".into()),
        ]))
}
fn events(params: &ParamMap) -> Value {
    let Some(array) = params.get("events").and_then(Param::as_array) else {
        return Value::Null;
    };
    let mut result = Vec::new();
    for value in array {
        if value.is_present() {
            let value = value.to_json();
            if !result.contains(&value) {
                result.push(value);
            }
        }
    }
    Value::Array(result)
}
fn sentence(errors: &Errors) -> String {
    let messages = errors.full_messages();
    match messages.len() {
        0 => String::new(),
        1 => messages[0].clone(),
        2 => messages.join(" and "),
        _ => format!(
            "{}, and {}",
            messages[..messages.len() - 1].join(", "),
            messages.last().unwrap()
        ),
    }
}
fn redirect(c: &mut Ctx, room: &Room, notice: Option<String>, alert: Option<String>) -> Result {
    let path = if room.open() {
        campfire_routes::edit_rooms_open(room.id)
    } else {
        campfire_routes::edit_rooms_closed(room.id)
    };
    c.redirect_to_with(
        &path,
        Redirect {
            notice,
            alert,
            ..Default::default()
        },
    )
}
pub async fn create(c: &mut Ctx) -> Result {
    let (room, user) = before(c).await?;
    let params = subscription_params(c)?;
    let input = SubscribeInput {
        room_id: room.id,
        user_id: user.id,
        full_name: params
            .get("full_name")
            .and_then(Param::to_s)
            .unwrap_or_default(),
        events: events(&params),
        administrator_override: user.is_administrator()
            && params.get("skip_access_check").and_then(Param::as_str) == Some("1"),
    };
    match subscriptions::subscribe(&c.app().db, &c.app().github_accounts, input).await {
        Ok(SubscribeOutcome::Created(s)) => redirect(
            c,
            &room,
            Some(format!("Subscribed to {}.", s.full_name())),
            None,
        ),
        Ok(SubscribeOutcome::Invalid(errors)) => redirect(
            c,
            &room,
            None,
            Some(format!("Could not subscribe: {}.", sentence(&errors))),
        ),
        Ok(SubscribeOutcome::Unverified {
            full_name,
            linked_usable,
        }) => {
            let reason = if linked_usable {
                format!("your linked GitHub account could not confirm it can read {full_name}")
            } else {
                format!(
                    "link your GitHub account on your profile so it can confirm you can read {full_name}"
                )
            };
            let override_hint = if user.is_administrator() {
                " Administrators may subscribe without verifying access."
            } else {
                ""
            };
            redirect(
                c,
                &room,
                None,
                Some(format!("Could not subscribe: {reason}.{override_hint}")),
            )
        }
        Err(error) if error.is_record_not_unique() => redirect(
            c,
            &room,
            None,
            Some("Could not subscribe: that repository is already subscribed in this room.".into()),
        ),
        Err(error) => Err(db_error(error)),
    }
}
async fn set_subscription(c: &Ctx, room_id: i64) -> Result<RepositorySubscription> {
    let id = c
        .param_str("id")
        .and_then(cast_integer)
        .ok_or(Error::NotFound)?;
    c.app()
        .db
        .read(move |conn| RepositorySubscription::find_for_room(conn, room_id, id))
        .await
        .map_err(db_error)
}
pub async fn update(c: &mut Ctx) -> Result {
    let (room, _) = before(c).await?;
    let mut s = set_subscription(c, room.id).await?;
    let full_name = s.full_name();
    let events = events(&subscription_params(c)?);
    let outcome = c
        .app()
        .db
        .write(move |tx| {
            let (owner, repo, verified) = (s.owner.clone(), s.repo.clone(), s.reader_verified);
            s.update(tx, &owner, &repo, events, verified)
        })
        .await;
    match outcome {
        Ok(()) => redirect(
            c,
            &room,
            Some(format!("Subscription to {full_name} updated.")),
            None,
        ),
        Err(campfire_db::Error::RecordInvalid(errors)) => redirect(
            c,
            &room,
            None,
            Some(format!("Could not update: {}.", sentence(&errors))),
        ),
        Err(error) => Err(db_error(error)),
    }
}
pub async fn destroy(c: &mut Ctx) -> Result {
    let (room, _) = before(c).await?;
    let s = set_subscription(c, room.id).await?;
    let name = s.full_name();
    c.app()
        .db
        .write(move |tx| s.destroy(tx))
        .await
        .map_err(db_error)?;
    redirect(c, &room, Some(format!("Unsubscribed from {name}.")), None)
}
