//! rooms/events/attendances_controller.rb. Event page actions are the next slice.
use crate::app::AppCtx;
use crate::concerns::{self, Before, before_actions, cast_integer, require_current_user};
use crate::controllers::presenters::page::{self, db_error};
use askama::Template;
use campfire_db::{CalendarEvent, Error as DbError};
use campfire_kit::{Ctx, Error, Redirect, Result, StatusCode, halt};
use campfire_views::events::{Attendance, AttendanceView};

async fn set_event(c: &mut Ctx) -> Result<CalendarEvent> {
    before_actions(c, Before::default()).await?;
    let (_, room) = concerns::set_room(c).await?;
    // The shared RoomScoped adapter on this base predates soft deletion.
    if room.deleted_at.is_some() {
        return Err(Error::NotFound);
    }
    let user = require_current_user(c)?;
    if !user.is_active() || user.is_bot() {
        return halt(concerns::head(StatusCode::FORBIDDEN));
    }
    let user_id = user.id;
    let Some(id) = c.param_str("event_id").and_then(cast_integer) else {
        return Err(Error::NotFound);
    };
    c.app()
        .db
        .read(move |conn| CalendarEvent::find_visible(conn, room.id, id, user_id))
        .await
        .map_err(db_error)
}

fn present(value: Option<&str>) -> Option<&str> {
    value.filter(|s| !campfire_richtext::ruby::is_blank(s))
}
fn nested(c: &Ctx, key: &str) -> Option<String> {
    c.params
        .get("attendance")
        .and_then(|p| p.get(key))
        .and_then(|p| p.as_str())
        .map(str::to_string)
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
    let message = c.param_str("message_id").map(str::to_string);
    render_attendance(c, event, message, None).await
}

pub async fn attendance_update(c: &mut Ctx) -> Result {
    let event = set_event(c).await?;
    let response = present(c.param_str("response"))
        .map(str::to_string)
        .or_else(|| nested(c, "response"))
        .unwrap_or_default();
    let message = present(c.param_str("message_id"))
        .map(str::to_string)
        .or_else(|| nested(c, "message_id"));
    let frame = c.is_turbo_frame_request() && present(message.as_deref()).is_some();
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
            || nested(c, "apply_to_future").as_deref() == Some("1");
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
    message_id: Option<String>,
    alert: Option<String>,
) -> Result {
    let user_id = require_current_user(c)?.id;
    let view = c
        .app()
        .db
        .read(move |conn| {
            let counts = event.attendance_counts(conn)?;
            Ok(AttendanceView {
                event_id: event.id,
                room_id: event.room_id,
                message_id,
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

#[cfg(test)]
mod tests;
