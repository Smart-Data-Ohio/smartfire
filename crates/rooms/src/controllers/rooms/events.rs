//! rooms/events_controller.rb and rooms/events/attendances_controller.rb.
mod input;
use crate::app::AppCtx;
use crate::concerns::{self, Before, before_actions, cast_integer, require_current_user};
use crate::controllers::presenters::page::{self, db_error as default_db_error};
use askama::Template;
use campfire_db::{CalendarEvent, Error as DbError};
use campfire_kit::{Ctx, Error, Param, Redirect, Result, StatusCode, halt};
use campfire_views::events::{Attendance, AttendanceView};

// Both Rails controllers rescue RecordNotFound with head :not_found, even for
// JSON requests; this is a callback response, before format selection.
fn db_error(error: DbError) -> Error {
    match error {
        DbError::RecordNotFound(_) => Error::Halt(Box::new(concerns::head(StatusCode::NOT_FOUND))),
        other => default_db_error(other),
    }
}

async fn set_event(c: &mut Ctx) -> Result<CalendarEvent> {
    let room = scheduled_room(c).await?;
    let user_id = require_current_user(c)?.id;
    let Some(id) = c
        .param_str("event_id")
        .or_else(|| c.param_str("id"))
        .and_then(cast_integer)
    else {
        return halt(concerns::head(StatusCode::NOT_FOUND));
    };
    c.app()
        .db
        .read(move |conn| CalendarEvent::find_visible(conn, room.id, id, user_id))
        .await
        .map_err(db_error)
}

async fn scheduled_room(c: &mut Ctx) -> Result<campfire_db::Room> {
    before_actions(c, Before::default()).await?;
    let (_, room) = match concerns::set_room(c).await {
        Err(Error::NotFound) => return halt(concerns::head(StatusCode::NOT_FOUND)),
        result => result?,
    };
    // The shared RoomScoped adapter on this base predates soft deletion.
    if room.deleted_at.is_some() {
        return halt(concerns::head(StatusCode::NOT_FOUND));
    }
    let user = require_current_user(c)?;
    if !user.is_active() || user.is_bot() {
        return halt(concerns::head(StatusCode::FORBIDDEN));
    }
    Ok(room)
}
pub async fn index(c: &mut Ctx) -> Result {
    let room = scheduled_room(c).await?;
    let user = require_current_user(c)?.clone();
    let now = c.app().db.env().now();
    let view = c
        .app()
        .db
        .read(move |conn| crate::controllers::presenters::events::index(conn, &room, &user, now))
        .await
        .map_err(db_error)?;
    page::framed_page!(c, StatusCode::OK, |ctx| {
        campfire_views::events::pages::Index { ctx, view: &view }
    })
    .await
}
pub async fn show(c: &mut Ctx) -> Result {
    let event = set_event(c).await?;
    let user = require_current_user(c)?.clone();
    let view = c
        .app()
        .db
        .read(move |conn| {
            crate::controllers::presenters::events::show(
                conn,
                &campfire_db::Room::find(conn, event.room_id)?,
                &user,
                &event,
            )
        })
        .await
        .map_err(db_error)?;
    page::framed_page!(c, StatusCode::OK, |ctx| {
        campfire_views::events::pages::Show { ctx, view: &view }
    })
    .await
}

fn nested<'a>(c: &'a Ctx, key: &str) -> Result<Option<&'a Param>> {
    // params.dig(:attendance, key) stops at nil, but raises on any non-hash
    // container (including arrays, since the next key is a symbol). Keep this
    // lookup lazy: Rails' presence/|| branches can bypass it entirely.
    match c.params.get("attendance") {
        None | Some(Param::Null) => Ok(None),
        Some(Param::Hash(params)) => Ok(params.get(key)),
        Some(_) => Err(Error::internal(anyhow::anyhow!(
            "Rails attendance parameters do not support dig with a named key"
        ))),
    }
}
fn attendance_param<'a>(c: &'a Ctx, key: &str) -> Result<Option<&'a Param>> {
    match c.params.get(key).filter(|p| p.is_present()) {
        Some(value) => Ok(Some(value)),
        None => nested(c, key),
    }
}
fn message_param_text(param: &Param) -> String {
    match param {
        Param::Array(items) => format!(
            "[{}]",
            items
                .iter()
                .map(message_param_inspect)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        other => campfire_richtext::ruby::json_value_to_s(&other.to_json()),
    }
}
fn message_param_inspect(param: &Param) -> String {
    match param {
        Param::Hash(_) => format!(
            "#<ActionController::Parameters {} permitted: false>",
            campfire_richtext::ruby::json_value_inspect(&param.to_json())
        ),
        Param::Array(_) => message_param_text(param),
        other => campfire_richtext::ruby::json_value_inspect(&other.to_json()),
    }
}
fn message_attribute_values(param: &Param, values: &mut Vec<String>) {
    match param {
        Param::Array(items) => {
            for item in items {
                message_attribute_values(item, values);
            }
        }
        other => values.push(message_param_text(other)),
    }
}
fn sentence(messages: Vec<String>) -> String {
    match messages.len() {
        0 => String::new(),
        1 => messages[0].clone(),
        2 => format!("{} and {}", messages[0], messages[1]),
        _ => format!(
            "{}, and {}",
            messages[..messages.len() - 1].join(", "),
            messages.last().unwrap()
        ),
    }
}

pub async fn attendance_show(c: &mut Ctx) -> Result {
    let event = set_event(c).await?;
    let message = c.params.get("message_id").cloned();
    render_attendance(c, event, message, None).await
}

pub async fn attendance_update(c: &mut Ctx) -> Result {
    let event = set_event(c).await?;
    let response = attendance_param(c, "response")?
        .and_then(Param::as_str)
        .map(str::to_string)
        .unwrap_or_default();
    let message = attendance_param(c, "message_id")?.cloned();
    let frame = c.is_turbo_frame_request() && message.as_ref().is_some_and(Param::is_present);
    let user_id = require_current_user(c)?.id;
    let respondable = c
        .app()
        .db
        .read({
            let event = event.clone();
            move |conn| event.respondable_by(conn, Some(&campfire_db::User::find(conn, user_id)?))
        })
        .await
        .map_err(db_error)?;
    let mut alert = if !campfire_db::models::calendar_event::attendance::RESPONSES
        .contains(&response.as_str())
    {
        Some("Choose going, maybe, or declined.".into())
    } else if !respondable {
        Some("This event is no longer open for responses.".into())
    } else {
        None
    };
    if alert.is_none() {
        let apply = c.param_str("apply_to_future") == Some("1")
            || nested(c, "apply_to_future")?.and_then(Param::as_str) == Some("1");
        let event_id = event.id;
        let response = response.clone();
        match c
            .app()
            .db
            .write(move |tx| CalendarEvent::respond(tx, event_id, user_id, &response, apply))
            .await
        {
            Ok(_) => (),
            Err(DbError::RecordInvalid(errors)) => alert = Some(sentence(errors.full_messages())),
            Err(error) => return Err(db_error(error)),
        }
    }
    if frame {
        render_attendance(c, event, message, alert).await
    } else {
        let path = format!("/rooms/{}/events/{}", event.room_id, event.id);
        let url = c.url_for(&path);
        c.redirect_to_with(
            &url,
            Redirect {
                notice: if alert.is_none() {
                    Some(format!("Response saved: {response}."))
                } else {
                    None
                },
                alert,
                ..Default::default()
            },
        )
    }
}

async fn render_attendance(
    c: &mut Ctx,
    event: CalendarEvent,
    message: Option<Param>,
    alert: Option<String>,
) -> Result {
    let user_id = require_current_user(c)?.id;
    let message_id = message
        .as_ref()
        .filter(|p| !p.is_null())
        .map(message_param_text);
    // Rails interpolates an array's inspect form in the frame id, but the tag
    // builder joins array attribute values with spaces in the hidden input.
    let message_id_input = message.as_ref().and_then(Param::as_array).map(|items| {
        let mut values = Vec::new();
        for item in items {
            message_attribute_values(item, &mut values);
        }
        values.join(" ")
    });
    let view = c
        .app()
        .db
        .read(move |conn| {
            let counts = event.attendance_counts(conn)?;
            Ok(AttendanceView {
                event_id: event.id,
                room_id: event.room_id,
                message_id,
                message_id_input,
                current_response: event.response_for(conn, Some(user_id))?,
                going: *counts.get("going").unwrap_or(&0),
                maybe: *counts.get("maybe").unwrap_or(&0),
                respondable: event
                    .respondable_by(conn, Some(&campfire_db::User::find(conn, user_id)?))?,
                cancelled: event.cancelled(),
                apply_to_future: event.series_head()
                    || (event.series() && event.next_occurrence(conn)?.is_some()),
                alert,
            })
        })
        .await
        .map_err(db_error)?;
    page::content(c, StatusCode::OK, |_| Attendance { view: &view }.render()).await
}

async fn render_form(
    c: &mut Ctx,
    room: campfire_db::Room,
    a: campfire_db::NewCalendarEvent,
    persisted: Option<CalendarEvent>,
    errors: campfire_db::Errors,
    title_value: Option<String>,
) -> Result {
    let user = require_current_user(c)?.clone();
    let editing = persisted.is_some();
    let status = if errors.is_empty() {
        StatusCode::OK
    } else {
        StatusCode::UNPROCESSABLE_ENTITY
    };
    let view = c
        .app()
        .db
        .read(move |conn| {
            crate::controllers::presenters::events::form(
                conn,
                &room,
                &user,
                &a,
                persisted.as_ref(),
                &errors,
                title_value,
            )
        })
        .await
        .map_err(db_error)?;
    if editing {
        page::framed_page!(c, status, |ctx| campfire_views::events::forms::Edit {
            ctx,
            view: &view
        })
        .await
    } else {
        page::framed_page!(c, status, |ctx| campfire_views::events::forms::New {
            ctx,
            view: &view
        })
        .await
    }
}
fn ensure_manager(c: &Ctx, e: &CalendarEvent, cancel: bool) -> Result<()> {
    let u = Some(require_current_user(c)?);
    if if cancel {
        e.cancellable_by(u)
    } else {
        e.manageable_by(u)
    } {
        Ok(())
    } else {
        halt(concerns::head(StatusCode::FORBIDDEN))
    }
}
async fn event_room(c: &Ctx, e: &CalendarEvent) -> Result<campfire_db::Room> {
    let id = e.room_id;
    c.app()
        .db
        .read(move |conn| campfire_db::Room::find(conn, id))
        .await
        .map_err(db_error)
}
fn redirect_event(c: &mut Ctx, e: &CalendarEvent, notice: &str) -> Result {
    let url = c.url_for(&format!("/rooms/{}/events/{}", e.room_id, e.id));
    c.redirect_to_with(
        &url,
        Redirect {
            notice: Some(notice.into()),
            ..Default::default()
        },
    )
}
pub async fn new(c: &mut Ctx) -> Result {
    let room = scheduled_room(c).await?;
    let user = require_current_user(c)?.clone();
    let viewer_zone = viewer_zone(c, user.id).await?;
    let a = input::prefill(
        &c.params,
        &viewer_zone,
        c.app().db.env().now(),
        room.id,
        user.id,
    )?;
    let title = (!a.title.is_empty()).then(|| a.title.clone());
    render_form(c, room, a, None, campfire_db::Errors::default(), title).await
}
pub async fn create(c: &mut Ctx) -> Result {
    let room = scheduled_room(c).await?;
    let user = require_current_user(c)?.clone();
    let viewer_zone = viewer_zone(c, user.id).await?;
    let changes = input::attributes(&c.params, None, &viewer_zone, c.app().db.env().now())?;
    let title = changes.title.clone();
    let a = input::new_attributes(changes, room.id, user.id);
    match c
        .app()
        .db
        .write_scoped(move || page::enter_time_zone(campfire_views::time::Zone::for_user(Some(&viewer_zone))), {
            let a = a.clone();
            move |tx| {
                // Rails save returns false for event validation, but exceptions from the
                // subsequent after-commit writes escape to production's public 500 page.
                let errors = CalendarEvent::validate(tx.conn(), &a)?;
                if errors.is_empty() {
                    CalendarEvent::create(tx, a).map(Ok)
                } else {
                    Ok(Err(errors))
                }
            }
        })
        .await
    {
        Ok(Ok(e)) => redirect_event(
            c,
            &e,
            if e.recurrence_rule.is_some() {
                "Repeating event scheduled."
            } else {
                "Event scheduled."
            },
        ),
        Ok(Err(errors)) => render_form(c, room, a, None, errors, title).await,
        Err(e) => Err(db_error(e)),
    }
}
pub async fn edit(c: &mut Ctx) -> Result {
    let e = set_event(c).await?;
    ensure_manager(c, &e, false)?;
    let room = event_room(c, &e).await?;
    let a = input::attempted(&Default::default(), &e);
    let title = Some(a.title.clone());
    render_form(c, room, a, Some(e), campfire_db::Errors::default(), title).await
}
pub async fn update(c: &mut Ctx) -> Result {
    let e = set_event(c).await?;
    ensure_manager(c, &e, false)?;
    let user = require_current_user(c)?.clone();
    let viewer_zone = viewer_zone(c, user.id).await?;
    let changes = input::attributes(&c.params, Some(&e), &viewer_zone, c.app().db.env().now())?;
    let scope = c.param_str("update_scope").unwrap_or_default().to_string();
    let actor = user.id;
    let id = e.id;
    match c
        .app()
        .db
        .write_scoped(move || page::enter_time_zone(campfire_views::time::Zone::for_user(Some(&viewer_zone))), {
            let changes = changes.clone();
            move |tx| CalendarEvent::update_with_scope(tx, id, changes, &scope, Some(actor))
        })
        .await
    {
        Ok(_) => redirect_event(c, &e, "Event updated."),
        Err(DbError::RecordInvalid(errors)) => {
            let room = event_room(c, &e).await?;
            let a = input::attempted(&changes, &e);
            let title = Some(a.title.clone());
            render_form(c, room, a, Some(e), errors, title).await
        }
        Err(e) => Err(db_error(e)),
    }
}
pub async fn cancel(c: &mut Ctx) -> Result {
    let e = set_event(c).await?;
    ensure_manager(c, &e, true)?;
    let scope = c.param_str("cancel_scope").unwrap_or_default().to_string();
    let actor = require_current_user(c)?.id;
    let viewer_zone = viewer_zone(c, actor).await?;
    let id = e.id;
    let cancelled = c
        .app()
        .db
        .write_scoped(move || page::enter_time_zone(campfire_views::time::Zone::for_user(Some(&viewer_zone))),
            move |tx| CalendarEvent::cancel_with_scope(tx, id, &scope, Some(actor)))
        .await
        .map_err(db_error)?;
    redirect_event(
        c,
        &e,
        if cancelled {
            "Event cancelled."
        } else {
            "Event was already cancelled."
        },
    )
}

async fn viewer_zone(c: &Ctx, id: i64) -> Result<String> {
    c.app()
        .db
        .read(move |conn| {
            Ok(conn
                .query_row("SELECT time_zone FROM users WHERE id=?", [id], |r| {
                    r.get::<_, Option<String>>(0)
                })?
                .filter(|z| campfire_db::slash_commands::time_parser::known_calendar_zone(z))
                .unwrap_or_else(|| "UTC".into()))
        })
        .await
        .map_err(db_error)
}
