//! `app/controllers/rooms/polls_controller.rb`.
use crate::app::AppCtx;
use crate::concerns::{Before, before_actions, cast_integer, require_current_user};
use crate::controllers::presenters::page::db_error;
use crate::controllers::{message_features as features, messages};
use campfire_db::{Message, NewMessage, NewPoll, Poll};
use campfire_kit::{Ctx, Error, Param, Permit, Result, StatusCode, format, permit_keys};

async fn prepare(c: &mut Ctx) -> Result<campfire_db::Room> {
    before_actions(c, Before::default()).await?;
    let room = features::room(c).await?;
    features::active_human(c)?;
    Ok(room)
}

async fn set_poll(c: &Ctx, room_id: i64) -> Result<Poll> {
    let id = c
        .param_str("id")
        .and_then(cast_integer)
        .ok_or(Error::NotFound)?;
    c.app()
        .db
        .read(move |conn| Poll::find_in_room(conn, room_id, id))
        .await
        .map_err(db_error)
}

pub async fn create(c: &mut Ctx) -> Result {
    let room = prepare(c).await?;
    let result = create_poll(c, &room).await;
    let poll = match result {
        Ok(poll) => poll,
        Err(Error::ParameterMissing(name)) => {
            let error = format!("param is missing or the value is empty or invalid: {name}");
            return match c.respond_to(&[&format::HTML, &format::JSON])? {
                f if *f == format::JSON => c.json(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    &serde_json::json!({"error": error}),
                ),
                _ => features::redirect(
                    c,
                    &campfire_routes::room(room.id),
                    None,
                    Some(error),
                    false,
                    None,
                ),
            };
        }
        Err(error) => return invalid(c, room.id, error, None).await,
    };
    let id = poll.message_id;
    let message = c
        .app()
        .db
        .read(move |conn| Message::find(conn, id))
        .await
        .map_err(db_error)?;
    messages::broadcast_create(c, &room, &message).await?;
    messages::release_webhooks(c, &message).await;
    match c.respond_to(&[&format::HTML, &format::JSON])? {
        f if *f == format::JSON => payload(c, poll, StatusCode::CREATED).await,
        _ => features::redirect(
            c,
            &campfire_routes::room(room.id),
            Some("Poll posted.".into()),
            None,
            false,
            None,
        ),
    }
}

async fn create_poll(c: &Ctx, room: &campfire_db::Room) -> Result<Poll> {
    let raw = c.params.require("poll")?;
    if raw.as_hash().is_none() {
        return Err(Error::internal(anyhow::anyhow!(
            "poll parameters do not support permit"
        )));
    }
    let mut filters = permit_keys(&["question", "multiple", "anonymous", "closes_at"]);
    filters.push(Permit::ScalarArray("options".into()));
    let permitted = raw.permit(&filters);
    let question = permitted.require("question")?;
    let question = match question {
        Param::Bool(true) => "t".into(),
        Param::Bool(false) => "f".into(),
        question => question.to_s().unwrap_or_default(),
    };
    let labels = permitted
        .get("options")
        .and_then(Param::as_array)
        .unwrap_or_default()
        .iter()
        .filter_map(Param::to_s)
        .collect();
    let zone = features::user_zone(c).await?;
    let closes_at = features::parse_time(
        &permitted
            .get("closes_at")
            .and_then(Param::to_s)
            .unwrap_or_default(),
        &zone,
        c.now(),
    )?;
    let options = NewPoll {
        labels,
        multiple: features::boolean(permitted.get("multiple")),
        anonymous: features::boolean(permitted.get("anonymous")),
        closes_at,
    };
    let creator_id = require_current_user(c)?.id;
    let room = room.clone();
    c.app()
        .db
        .write(move |tx| {
            let message = Message::create(
                tx,
                NewMessage {
                    room_id: room.id,
                    creator_id,
                    markdown_source: Some(question),
                    ..Default::default()
                },
            )?;
            let poll = Poll::create_for_message(tx, &message, options)?;
            messages::deliver_webhooks_to_bots(tx, &room, &message)?;
            Ok(poll)
        })
        .await
        .map_err(db_error)
}

pub async fn show(c: &mut Ctx) -> Result {
    let room = prepare(c).await?;
    let poll = set_poll(c, room.id).await?;
    payload(c, poll, StatusCode::OK).await
}

pub async fn vote(c: &mut Ctx) -> Result {
    let room = prepare(c).await?;
    let poll = set_poll(c, room.id).await?;
    let ids = ballot(c.param("option_ids"))?;
    let user_id = require_current_user(c)?.id;
    let voted = poll.clone();
    let origin = crate::controllers::presenters::page::renderer_base_url(c);
    if let Err(error) = c
        .app()
        .db
        .write_scoped(
            move || crate::channels::message_features::origin(&origin),
            move |tx| voted.clone().cast_vote(tx, user_id, &ids),
        )
        .await
        .map_err(db_error)
    {
        return invalid(c, room.id, error, Some(poll)).await;
    }
    match c.respond_to(&[&format::TURBO_STREAM, &format::JSON, &format::HTML])? {
        f if *f == format::JSON => payload(c, poll, StatusCode::OK).await,
        f if *f == format::TURBO_STREAM => card_response(c, poll.id, None, StatusCode::OK).await,
        _ => features::redirect(c, &campfire_routes::room(room.id), None, None, false, None),
    }
}

/// `Array(option_ids).map { |id| id.to_i }.uniq`. Null is an empty ballot; numeric
/// strings accept a trailing suffix; arrays/hashes/bools raise rather than silently retract.
fn ballot(value: Option<&Param>) -> Result<Vec<i64>> {
    let values: Vec<&Param> = match value {
        None | Some(Param::Null) => vec![],
        Some(Param::Array(values)) => values.iter().collect(),
        Some(value) => vec![value],
    };
    values
        .into_iter()
        .map(|value| match value {
            Param::Str(value) => Ok(cast_integer(value).unwrap_or(0)),
            Param::Number(value) => value
                .as_i64()
                .or_else(|| value.as_f64().map(|value| value as i64))
                .ok_or_else(|| Error::internal(anyhow::anyhow!("option id has no to_i"))),
            Param::Null => Ok(0),
            _ => Err(Error::internal(anyhow::anyhow!("option id has no to_i"))),
        })
        .collect()
}

async fn payload(c: &mut Ctx, poll: Poll, status: StatusCode) -> Result {
    let app = c.app().clone();
    let viewer = require_current_user(c)?.id;
    let value = c
        .app()
        .db
        .read(move |conn| {
            Poll::find(conn, poll.id)?.results_payload(
                conn,
                &*app.db.env().rich_text,
                app.db.env().now(),
                Some(viewer),
            )
        })
        .await
        .map_err(db_error)?;
    c.json(status, &value)
}

async fn invalid(c: &mut Ctx, room_id: i64, error: Error, poll: Option<Poll>) -> Result {
    let Error::Internal(inner) = error else {
        return Err(error);
    };
    let Some(campfire_db::Error::RecordInvalid(errors)) =
        inner.downcast_ref::<campfire_db::Error>()
    else {
        return Err(Error::Internal(inner));
    };
    let errors = errors.clone();
    let message = features::sentence(&errors);
    let formats = if poll.is_some() {
        vec![&format::TURBO_STREAM, &format::JSON, &format::HTML]
    } else {
        vec![&format::HTML, &format::JSON]
    };
    match c.respond_to(&formats)? {
        f if *f == format::JSON => c.json(
            StatusCode::UNPROCESSABLE_ENTITY,
            &features::errors_json(&errors),
        ),
        f if *f == format::TURBO_STREAM => {
            card_response(
                c,
                poll.unwrap().id,
                Some(message),
                StatusCode::UNPROCESSABLE_ENTITY,
            )
            .await
        }
        _ => features::redirect(
            c,
            &campfire_routes::room(room_id),
            None,
            Some(message),
            false,
            None,
        ),
    }
}

async fn card_response(c: &mut Ctx, id: i64, error: Option<String>, status: StatusCode) -> Result {
    let app = c.app().clone();
    let view = c
        .app()
        .db
        .read(move |conn| features::poll_view(conn, &app, id, error))
        .await
        .map_err(db_error)?;
    crate::controllers::presenters::page::bare(c, status, &format::TURBO_STREAM, |ctx| {
        Ok(campfire_cable::turbo::action_tag(
            campfire_cable::turbo::Action::Replace,
            campfire_cable::turbo::Target::Target(&format!("card_poll_{id}")),
            Some(campfire_views::messages::parts::poll(ctx, &view).0.as_str()),
            &[],
        ))
    })
    .await
}
