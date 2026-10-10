//! JSON twins of the classic room GitHub subscription and inbound-email forms.
//! Authorization, validation and writes are the classic controllers' domain calls.

use std::collections::BTreeMap;

use campfire_api_types as api;
use campfire_app::app::AppCtx;
use campfire_app::integrations::github::subscriptions::{
    self, DEFAULT_EVENTS, EVENT_KEYS, RepositorySubscription, SubscribeInput, SubscribeOutcome,
};
use campfire_db::{Errors, Room};
use campfire_kit::{Ctx, Error, Result, StatusCode};
use campfire_presentation::helpers::to_sentence;
use campfire_runtime::concerns::{self, cast_integer};
use campfire_runtime::context::db_error;
use serde_json::Value;

use crate::endpoints::{before_actions, body, set_room};
use crate::error::{fail, record_invalid};

/// Classic OAuth start. The callback returns to integrations or the profile, never here.
const CONNECT_PATH: &str = "/github/app/connect";

endpoint!(
    /// `GET /api/v1/rooms/:room_id/github_subscriptions`
    github_subscriptions => list_subscriptions
);
endpoint!(
    /// `POST /api/v1/rooms/:room_id/github_subscriptions`
    subscribe_repository => subscribe
);
endpoint!(
    /// `PATCH /api/v1/rooms/:room_id/github_subscriptions/:id`
    update_github_subscription => update_subscription
);
endpoint!(
    /// `DELETE /api/v1/rooms/:room_id/github_subscriptions/:id`
    unsubscribe_repository => unsubscribe
);
endpoint!(
    /// `GET /api/v1/rooms/:room_id/inbound_email`
    inbound_email => show_inbound_email
);
endpoint!(
    /// `POST /api/v1/rooms/:room_id/inbound_email`
    rotate_inbound_email => rotate_inbound
);

async fn github_room(c: &mut Ctx) -> Result<(Room, campfire_db::User)> {
    before_actions(c).await?;
    let (_, room) = set_room(c).await?;
    if room.direct() {
        return Err(Error::NotFound);
    }
    campfire_rooms::controllers::rooms::ensure_can_administer(c, &room)?;
    Ok((room, concerns::require_current_user(c)?.clone()))
}

async fn email_room(c: &mut Ctx) -> Result<Room> {
    before_actions(c).await?;
    let (_, room) = set_room(c).await?;
    if !room.emailable() {
        return Err(Error::NotFound);
    }
    campfire_rooms::controllers::rooms::ensure_can_administer(c, &room)?;
    Ok(room)
}

fn subscription_id(c: &Ctx) -> Result<i64> {
    c.param_str("id")
        .and_then(cast_integer)
        .ok_or(Error::NotFound)
}

fn event_label(key: &str) -> String {
    let mut label = key.replace('_', " ");
    if let Some(first) = label.get_mut(..1) {
        first.make_ascii_uppercase();
    }
    label
}

fn catalog() -> Vec<api::GithubEventChoice> {
    EVENT_KEYS
        .into_iter()
        .map(|key| api::GithubEventChoice {
            key: key.to_string(),
            label: event_label(key),
            selected_by_default: DEFAULT_EVENTS.contains(&key),
        })
        .collect()
}

fn subscription_dto(subscription: &RepositorySubscription) -> api::GithubSubscription {
    api::GithubSubscription {
        id: subscription.id,
        full_name: subscription.full_name(),
        events: subscription
            .events
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|event| event.as_str().map(str::to_owned))
            .collect(),
    }
}

fn events_value(events: &[String]) -> Value {
    Value::Array(events.iter().cloned().map(Value::String).collect())
}

fn connect_path(c: &Ctx) -> Option<String> {
    c.app()
        .github_app
        .configured()
        .then(|| CONNECT_PATH.to_string())
}

fn list_dto(
    subscriptions: Vec<RepositorySubscription>,
    administrator: bool,
    connect_path: Option<String>,
) -> api::GithubSubscriptionList {
    api::GithubSubscriptionList {
        subscriptions: subscriptions.iter().map(subscription_dto).collect(),
        administrator,
        connect_path,
        events: catalog(),
    }
}

fn flash(prefix: &str, errors: &Errors) -> String {
    format!(
        "{prefix}: {}.",
        to_sentence(&errors.full_messages(), " and ")
    )
}

fn invalid(c: &mut Ctx, prefix: &str, errors: &Errors) -> Error {
    let mut api_error = record_invalid(
        errors,
        &[
            ("owner", "fullName"),
            ("repo", "fullName"),
            ("room", "fullName"),
        ],
    );
    if let api::ApiError::Validation { message, .. } = &mut api_error {
        *message = flash(prefix, errors);
    }
    fail(c, api_error)
}

fn unverified(c: &mut Ctx, full_name: &str, linked_usable: bool, administrator: bool) -> Error {
    let reason = if linked_usable {
        format!("your linked GitHub account could not confirm it can read {full_name}")
    } else {
        format!(
            "link your GitHub account on your profile so it can confirm you can read {full_name}"
        )
    };
    let override_hint = if administrator {
        " Administrators may subscribe without verifying access."
    } else {
        ""
    };
    let mut fields = BTreeMap::new();
    fields.insert(
        "github".into(),
        vec![if linked_usable {
            "unreadable".into()
        } else {
            "not_linked".into()
        }],
    );
    fail(
        c,
        api::ApiError::Validation {
            message: format!("Could not subscribe: {reason}.{override_hint}"),
            fields,
        },
    )
}

async fn list_subscriptions(c: &mut Ctx) -> Result {
    let (room, user) = github_room(c).await?;
    let room_id = room.id;
    let subscriptions = c
        .app()
        .db
        .read(move |conn| RepositorySubscription::for_room(conn, room_id))
        .await
        .map_err(db_error)?;
    c.json(
        StatusCode::OK,
        &list_dto(subscriptions, user.is_administrator(), connect_path(c)),
    )
}

async fn subscribe(c: &mut Ctx) -> Result {
    let (room, user) = github_room(c).await?;
    let request: api::SubscribeGithubRepository = body(c).await?;
    let input = SubscribeInput {
        room_id: room.id,
        user_id: user.id,
        full_name: request.full_name,
        events: events_value(&request.events),
        administrator_override: user.is_administrator() && request.skip_access_check,
    };
    match subscriptions::subscribe(&c.app().db, &c.app().github_accounts, input).await {
        Ok(SubscribeOutcome::Created(subscription)) => {
            c.json(StatusCode::CREATED, &subscription_dto(&subscription))
        }
        Ok(SubscribeOutcome::Invalid(errors)) => Err(invalid(c, "Could not subscribe", &errors)),
        Ok(SubscribeOutcome::Unverified {
            full_name,
            linked_usable,
        }) => Err(unverified(
            c,
            &full_name,
            linked_usable,
            user.is_administrator(),
        )),
        Err(error) if error.is_record_not_unique() => Err(fail(
            c,
            api::ApiError::Validation {
                message: "Could not subscribe: that repository is already subscribed in this room."
                    .into(),
                fields: BTreeMap::from([(
                    "fullName".into(),
                    vec!["has already been taken".into()],
                )]),
            },
        )),
        Err(error) => Err(db_error(error)),
    }
}

async fn load_subscription(c: &Ctx, room_id: i64) -> Result<RepositorySubscription> {
    let id = subscription_id(c)?;
    c.app()
        .db
        .read(move |conn| RepositorySubscription::find_for_room(conn, room_id, id))
        .await
        .map_err(db_error)
}

async fn update_subscription(c: &mut Ctx) -> Result {
    let (room, _) = github_room(c).await?;
    let mut subscription = load_subscription(c, room.id).await?;
    let request: api::UpdateGithubSubscription = body(c).await?;
    let events = events_value(&request.events);
    let outcome = c
        .app()
        .db
        .write(move |tx| {
            let (owner, repo, verified) = (
                subscription.owner.clone(),
                subscription.repo.clone(),
                subscription.reader_verified,
            );
            subscription.update(tx, &owner, &repo, events, verified)?;
            Ok(subscription)
        })
        .await;
    match outcome {
        Ok(subscription) => c.json(StatusCode::OK, &subscription_dto(&subscription)),
        Err(campfire_db::Error::RecordInvalid(errors)) => {
            Err(invalid(c, "Could not update", &errors))
        }
        Err(error) => Err(db_error(error)),
    }
}

async fn unsubscribe(c: &mut Ctx) -> Result {
    let (room, _) = github_room(c).await?;
    let subscription = load_subscription(c, room.id).await?;
    let dto = subscription_dto(&subscription);
    c.app()
        .db
        .write(move |tx| subscription.destroy(tx))
        .await
        .map_err(db_error)?;
    c.json(StatusCode::OK, &dto)
}

fn inbound_view(domain: Option<&str>, room: &Room) -> api::InboundEmail {
    let address = domain.and_then(|domain| {
        let token = room.inbound_email_token.as_deref()?;
        if token.chars().all(char::is_whitespace) {
            return None;
        }
        Some(format!("room-{token}@{domain}"))
    });
    api::InboundEmail {
        enabled: domain.is_some(),
        address,
    }
}

async fn show_inbound_email(c: &mut Ctx) -> Result {
    let room = email_room(c).await?;
    let domain = c.app().mail.config.domain.clone();
    c.json(StatusCode::OK, &inbound_view(domain.as_deref(), &room))
}

async fn rotate_inbound(c: &mut Ctx) -> Result {
    let room = email_room(c).await?;
    let domain = c.app().mail.config.domain.clone();
    let room = c
        .app()
        .db
        .write(move |tx| {
            let mut room = room;
            room.regenerate_inbound_email_token(tx)?;
            Ok(room)
        })
        .await
        .map_err(db_error)?;
    c.json(StatusCode::OK, &inbound_view(domain.as_deref(), &room))
}
