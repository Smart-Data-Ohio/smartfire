//! `app/controllers/scheduled_messages_controller.rb`.
use crate::app::AppCtx;
use crate::concerns::{Before, before_actions, cast_integer, require_current_user};
use crate::controllers::{
    message_features as features,
    presenters::{Presenter, page},
};
use campfire_db::{ChannelThread, Message, NewScheduledMessage, Room, ScheduledMessage, User};
use campfire_kit::{Ctx, Error, Param, Result, StatusCode, format, permit_keys};

const BUSY: &str = "That message is sending right now; try again in a moment.";
async fn prepare(c: &mut Ctx) -> Result<()> {
    c.rescue_not_found();
    before_actions(c, Before::default()).await?;
    features::active_human(c)?;
    c.no_store();
    c.set_header("Pragma", "no-cache");
    c.start_action();
    Ok(())
}
async fn pending(c: &Ctx) -> Result<ScheduledMessage> {
    let user = require_current_user(c)?.id;
    let id = c
        .param_str("id")
        .and_then(cast_integer)
        .ok_or(Error::NotFound)?;
    c.app()
        .db
        .read(move |conn| {
            let row = ScheduledMessage::find(conn, id)?;
            if row.user_id != user || !row.pending() {
                return Err(campfire_db::Error::RecordNotFound("ScheduledMessage"));
            }
            Ok(row)
        })
        .await
        .map_err(page::db_error)
}
fn permitted(c: &Ctx, keys: &[&str]) -> Result<campfire_kit::ParamMap> {
    let raw = c.params.require("scheduled_message")?;
    if raw.as_hash().is_none() {
        return Err(Error::internal(anyhow::anyhow!(
            "scheduled_message does not support permit"
        )));
    }
    Ok(raw.permit(&permit_keys(keys)))
}
fn string(value: &Param) -> String {
    match value {
        Param::Bool(true) => "t".into(),
        Param::Bool(false) => "f".into(),
        value => value.to_s().unwrap_or_default(),
    }
}
fn integer(value: Option<&Param>) -> Option<i64> {
    value
        .filter(|value| value.is_present())
        .and_then(Param::to_s)
        .and_then(|raw| cast_integer(&raw))
}
pub async fn index(c: &mut Ctx) -> Result {
    prepare(c).await?;
    c.respond_to(&[&format::HTML])?;
    let user_id = require_current_user(c)?.id;
    let app = c.app().clone();
    let (upcoming, stranded, past) = c
        .app()
        .db
        .read(move |conn| {
            let viewer = User::find(conn, user_id)?;
            let presenter = Presenter::new(conn, &app, None);
            let mut upcoming = Vec::new();
            let mut stranded = Vec::new();
            for row in ScheduledMessage::owned_by(conn, user_id, false)? {
                let sendable = row.sendable(conn)?;
                let view = view(&presenter, conn, &viewer, &row)?;
                if sendable {
                    upcoming.push(view)
                } else {
                    stranded.push(view)
                }
            }
            let past = ScheduledMessage::owned_by(conn, user_id, true)?
                .iter()
                .map(|row| view(&presenter, conn, &viewer, row))
                .collect::<campfire_db::Result<Vec<_>>>()?;
            Ok((upcoming, stranded, past))
        })
        .await
        .map_err(page::db_error)?;
    page::framed_page!(c, StatusCode::OK, |ctx| {
        campfire_views::scheduled_messages::Index {
            ctx,
            upcoming: &upcoming,
            stranded: &stranded,
            past: &past,
        }
    })
    .await
}
pub(crate) fn view(
    presenter: &Presenter<'_>,
    conn: &campfire_db::Connection,
    viewer: &User,
    row: &ScheduledMessage,
) -> campfire_db::Result<campfire_views::scheduled_messages::Item> {
    Ok(campfire_views::scheduled_messages::Item {
        id: row.id,
        room_name: presenter.room_display_name(&Room::find(conn, row.room_id)?, Some(viewer))?,
        thread_name: row
            .thread_id
            .map(|id| ChannelThread::find(conn, id).map(|thread| thread.name))
            .transpose()?,
        body: row.markdown_source.clone(),
        send_at: row.send_at.jiff(),
        sent_at: row.sent_at.map(|at| at.jiff()),
        message_path: row
            .sent_message_id
            .map(|id| Message::find_by_id(conn, id))
            .transpose()?
            .flatten()
            .as_ref()
            .map(campfire_db::message_pin::message_path),
    })
}
pub async fn create(c: &mut Ctx) -> Result {
    prepare(c).await?;
    let room = features::room(c).await?;
    let params = permitted(
        c,
        &[
            "markdown_source",
            "send_at",
            "thread_id",
            "reply_to_message_id",
        ],
    )?;
    let raw_thread = params.get("thread_id").filter(|value| value.is_present());
    let thread_id = integer(raw_thread);
    let has_thread = raw_thread.is_some();
    let raw_reply = params
        .get("reply_to_message_id")
        .filter(|value| value.is_present());
    let reply_id = integer(raw_reply);
    let has_reply = raw_reply.is_some();
    let room_id = room.id;
    let found = c
        .app()
        .db
        .read(move |conn| {
            if has_thread && thread_id.is_none() {
                return Err(campfire_db::Error::RecordNotFound("ChannelThread"));
            }
            if let Some(thread_id) = thread_id
                && ChannelThread::find(conn, thread_id)?.room_id != room_id
            {
                return Err(campfire_db::Error::RecordNotFound("ChannelThread"));
            }
            if has_reply {
                let reply = reply_id
                    .map(|id| Message::find_by_id(conn, id))
                    .transpose()?
                    .flatten();
                if reply.is_none_or(|message| {
                    message.room_id != room_id || message.thread_id != thread_id
                }) {
                    return Ok(false);
                }
            }
            Ok(true)
        })
        .await
        .map_err(page::db_error)?;
    let source = string(params.require("markdown_source")?);
    if !found {
        return invalid(
            c,
            "Reply target is not in this conversation",
            None,
            StatusCode::UNPROCESSABLE_ENTITY,
            true,
        )
        .await;
    }
    let zone = features::user_zone(c).await?;
    let Some(send_at) = features::parse_time(
        &params
            .get("send_at")
            .and_then(Param::to_s)
            .unwrap_or_default(),
        &zone,
        c.now(),
    )
    .unwrap_or_default() else {
        return invalid(
            c,
            "Send time is invalid",
            None,
            StatusCode::UNPROCESSABLE_ENTITY,
            true,
        )
        .await;
    };
    let user_id = require_current_user(c)?.id;
    match c
        .app()
        .db
        .write(move |tx| {
            ScheduledMessage::create(
                tx,
                NewScheduledMessage {
                    user_id,
                    room_id,
                    thread_id,
                    reply_to_message_id: reply_id,
                    markdown_source: source,
                    send_at,
                },
            )
        })
        .await
    {
        Ok(row) => match c.respond_to(&[&format::HTML, &format::JSON])? {
            f if *f == format::JSON => payload(c, &row, StatusCode::CREATED).await,
            _ => features::redirect(
                c,
                &campfire_routes::scheduled_messages(),
                Some(format!(
                    "Message scheduled for {}.",
                    zone.to_fs(row.send_at.jiff(), "long")
                )),
                None,
                false,
                None,
            ),
        },
        Err(campfire_db::Error::RecordInvalid(errors)) => {
            invalid(
                c,
                &features::sentence(&errors),
                Some(features::errors_json(&errors)),
                StatusCode::UNPROCESSABLE_ENTITY,
                true,
            )
            .await
        }
        Err(error) => Err(page::db_error(error)),
    }
}
enum UpdateOutcome {
    Busy,
    InvalidParams(Error),
    InvalidTime,
    Saved(Box<ScheduledMessage>),
}
pub async fn update(c: &mut Ctx) -> Result {
    prepare(c).await?;
    let initial = pending(c).await?;
    let params = permitted(c, &["markdown_source", "send_at"]);
    let zone = features::user_zone(c).await?;
    let now = c.now();
    let result = c
        .app()
        .db
        .write(move |tx| {
            let mut row = ScheduledMessage::find(tx.conn(), initial.id)?;
            if !row.pending() || row.claimed(tx.now()) {
                return Ok(UpdateOutcome::Busy);
            }
            // Rails calls update_attributes inside with_lock, after busy?; malformed params
            // must not override a refusal for a claim/send that landed since the first lookup.
            let params = match params {
                Ok(params) => params,
                Err(error) => return Ok(UpdateOutcome::InvalidParams(error)),
            };
            let source = params
                .get("markdown_source")
                .map(string)
                .unwrap_or_else(|| row.markdown_source.clone());
            let time = if params.contains_key("send_at") {
                match features::parse_time(
                    &params
                        .get("send_at")
                        .and_then(Param::to_s)
                        .unwrap_or_default(),
                    &zone,
                    now,
                )
                .unwrap_or_default()
                {
                    Some(time) => time,
                    None => return Ok(UpdateOutcome::InvalidTime),
                }
            } else {
                row.send_at
            };
            row.update(tx, &source, time)?;
            Ok(UpdateOutcome::Saved(Box::new(row)))
        })
        .await;
    match result {
        Ok(UpdateOutcome::Busy) => invalid(c, BUSY, None, StatusCode::CONFLICT, false).await,
        Ok(UpdateOutcome::InvalidParams(error)) => Err(error),
        Ok(UpdateOutcome::InvalidTime) => {
            invalid(
                c,
                "Send time is invalid",
                None,
                StatusCode::UNPROCESSABLE_ENTITY,
                false,
            )
            .await
        }
        Ok(UpdateOutcome::Saved(row)) => match c.respond_to(&[&format::HTML, &format::JSON])? {
            f if *f == format::JSON => payload(c, &row, StatusCode::OK).await,
            _ => features::redirect(
                c,
                &campfire_routes::scheduled_messages(),
                Some("Scheduled message updated.".into()),
                None,
                false,
                None,
            ),
        },
        Err(campfire_db::Error::RecordInvalid(errors)) => {
            invalid(
                c,
                &features::sentence(&errors),
                Some(features::errors_json(&errors)),
                StatusCode::UNPROCESSABLE_ENTITY,
                false,
            )
            .await
        }
        Err(error) => Err(page::db_error(error)),
    }
}
pub async fn destroy(c: &mut Ctx) -> Result {
    prepare(c).await?;
    let initial = pending(c).await?;
    let cancelled = c
        .app()
        .db
        .write(move |tx| {
            let row = ScheduledMessage::find(tx.conn(), initial.id)?;
            if !row.pending() || row.claimed(tx.now()) {
                return Ok(false);
            }
            row.destroy(tx)?;
            Ok(true)
        })
        .await
        .map_err(page::db_error)?;
    if !cancelled {
        return invalid(c, BUSY, None, StatusCode::CONFLICT, false).await;
    }
    match c.respond_to(&[&format::HTML, &format::JSON])? {
        f if *f == format::JSON => Ok(c.head(StatusCode::NO_CONTENT)),
        _ => features::redirect(
            c,
            &campfire_routes::scheduled_messages(),
            Some("Scheduled message cancelled.".into()),
            None,
            false,
            None,
        ),
    }
}
pub async fn send_now(c: &mut Ctx) -> Result {
    prepare(c).await?;
    let initial = pending(c).await?;
    let origin = page::renderer_base_url(c);
    let (sent, row) = c
        .app()
        .db
        .write_scoped(
            move || crate::channels::message_features::origin(&origin),
            move |tx| {
                let sent = ScheduledMessage::dispatch(tx, initial.id, tx.now(), true)?;
                Ok((sent, ScheduledMessage::find(tx.conn(), initial.id)?))
            },
        )
        .await
        .map_err(page::db_error)?;
    if !sent && row.dropped() {
        let alert = row
            .drop_reason
            .as_deref()
            .filter(|reason| !reason.is_empty())
            .map(|reason| format!("The scheduled message was not sent ({reason})."))
            .unwrap_or_else(|| {
                "You no longer have access to that room, so the message was not sent.".into()
            });
        return invalid(c, &alert, None, StatusCode::UNPROCESSABLE_ENTITY, false).await;
    }
    match c.respond_to(&[&format::HTML, &format::JSON])? {
        f if *f == format::JSON => {
            payload(
                c,
                &row,
                if sent {
                    StatusCode::OK
                } else {
                    StatusCode::ACCEPTED
                },
            )
            .await
        }
        _ => features::redirect(
            c,
            &campfire_routes::scheduled_messages(),
            Some(
                if sent {
                    "Message sent."
                } else {
                    "Message is sending."
                }
                .into(),
            ),
            None,
            false,
            None,
        ),
    }
}
async fn payload(c: &mut Ctx, row: &ScheduledMessage, status: StatusCode) -> Result {
    let zone = features::user_zone(c).await?;
    let stamp = |time: campfire_db::Timestamp| {
        zone.format(
            time.jiff(),
            if zone.tz().to_offset_info(time.jiff()).abbreviation() == "UTC" {
                "%Y-%m-%dT%H:%M:%S%.3fZ"
            } else {
                "%Y-%m-%dT%H:%M:%S%.3f%:z"
            },
        )
    };
    c.json(status,&serde_json::json!({"id":row.id,"room_id":row.room_id,"thread_id":row.thread_id,"markdown_source":row.markdown_source,"send_at":stamp(row.send_at),"sent_at":row.sent_at.map(stamp),"dropped_at":row.dropped_at.map(stamp)}))
}
async fn invalid(
    c: &mut Ctx,
    alert: &str,
    value: Option<serde_json::Value>,
    status: StatusCode,
    back: bool,
) -> Result {
    match c.respond_to(&[&format::HTML, &format::JSON])? {
        f if *f == format::JSON => c.json(
            status,
            &value.unwrap_or_else(|| serde_json::json!({"error":alert})),
        ),
        _ => features::redirect(
            c,
            &campfire_routes::scheduled_messages(),
            None,
            Some(alert.into()),
            back,
            None,
        ),
    }
}
