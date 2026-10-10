//! Shared HTTP seams for the message features, kept separate from message-root ownership.

use crate::app::AppCtx;
use crate::concerns::{self, cast_integer, require_current_user};
use crate::controllers::presenters::page::db_error;
use campfire_db::{Message, Role, Room, Timestamp};
use campfire_kit::{Ctx, Error, Param, Redirect, Result, StatusCode, halt};
pub use crate::controllers::presenters::message_parts::poll_view;
pub use crate::controllers::presenters::params::param_string;

pub async fn room(c: &mut Ctx) -> Result<Room> {
    c.rescue_not_found();
    let (_, room) = concerns::set_room(c).await?;
    if room.deleted_at.is_some() {
        return Err(Error::NotFound);
    }
    Ok(room)
}

pub fn active_human(c: &Ctx) -> Result<()> {
    let user = require_current_user(c)?;
    if !user.is_active() || user.role == Role::Bot {
        return halt(concerns::head(StatusCode::FORBIDDEN));
    }
    Ok(())
}

pub async fn reachable_message(c: &mut Ctx) -> Result<Message> {
    c.rescue_not_found();
    let user_id = require_current_user(c)?.id;
    let id = c
        .param("message_id")
        .and_then(Param::to_s)
        .and_then(|raw| cast_integer(&raw))
        .ok_or(Error::NotFound)?;
    c.app()
        .db
        .read(move |conn| Message::find_reachable(conn, user_id, id))
        .await
        .map_err(db_error)
}

pub fn redirect(
    c: &mut Ctx,
    path: &str,
    notice: Option<String>,
    alert: Option<String>,
    back: bool,
    status: Option<StatusCode>,
) -> Result {
    if back {
        if let Some(notice) = notice {
            c.flash().set_notice(notice);
        }
        if let Some(alert) = alert {
            c.flash().set_alert(alert);
        }
        return c.redirect_back_or_to(path);
    }
    c.redirect_to_with(
        path,
        Redirect {
            notice,
            alert,
            status,
            ..Default::default()
        },
    )
}

pub fn errors_json(errors: &campfire_db::Errors) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    for (attribute, message) in &errors.0 {
        map.entry((*attribute).to_owned())
            .or_insert_with(|| serde_json::json!([]))
            .as_array_mut()
            .unwrap()
            .push(message.clone().into());
    }
    serde_json::json!({"errors": map})
}

pub fn sentence(errors: &campfire_db::Errors) -> String {
    campfire_presentation::helpers::to_sentence(&errors.full_messages(), " and ")
}

pub fn boolean(value: Option<&Param>) -> bool {
    !matches!(value, None | Some(Param::Null | Param::Bool(false)))
        && !matches!(
            value.and_then(Param::to_s).as_deref(),
            Some("" | "0" | "f" | "F" | "false" | "FALSE" | "off" | "OFF")
        )
}

pub async fn user_zone(c: &Ctx) -> Result<campfire_presentation::time::Zone> {
    let id = require_current_user(c)?.id;
    let name: Option<String> = c
        .app()
        .db
        .read(move |conn| {
            Ok(
                conn.query_row("SELECT time_zone FROM users WHERE id = ?", [id], |row| {
                    row.get(0)
                })?,
            )
        })
        .await
        .map_err(db_error)?;
    Ok(campfire_presentation::time::Zone::for_user(name.as_deref()))
}

/// WS8bm2 builder calendar parser; slash-relative expressions use a separate entry
/// point. Unrecognized input and Ruby's invalid-calendar exception stay distinct.
pub fn parse_time(
    raw: &str,
    zone: &campfire_presentation::time::Zone,
    now: jiff::Timestamp,
) -> Result<Option<Timestamp>> {
    parse_time_checked(raw, zone, now).map_err(db_error)
}

pub fn parse_time_checked(raw: &str, zone: &campfire_presentation::time::Zone, now: jiff::Timestamp) -> campfire_db::Result<Option<Timestamp>> {
    campfire_db::slash_commands::time_parser::parse_calendar(raw, zone.tz(), Timestamp::from_jiff(now))
}

/// Rails JSON encodes Time in UTC with millisecond precision.
pub fn json_time(time: Timestamp) -> String {
    json_time_in_zone(time, &jiff::tz::TimeZone::UTC)
}
pub fn json_time_in_zone(time: Timestamp, zone: &jiff::tz::TimeZone) -> String {
    let mut encoded = rails_compat::datetime::render(time, zone, true);
    let suffix = if encoded.ends_with('Z') {
        encoded.len() - 1
    } else {
        encoded.len() - 6
    };
    encoded.insert_str(suffix, &format!(".{:03}", time.subsec_microsecond() / 1000));
    encoded
}

#[cfg(test)]
mod relative_split_input_tests;

/// Presentation strings use WS11's shared wide-time renderer; view models never
/// narrow an accepted database timestamp through Jiff.
pub fn html_datetime(at: campfire_db::Timestamp, zone: &campfire_presentation::time::Zone) -> String {
    rails_compat::datetime::render(at, zone.tz(), true)
}
pub fn html_long(at: campfire_db::Timestamp, zone: &campfire_presentation::time::Zone) -> String {
    rails_compat::datetime::format(at, zone.tz(), "%B %d, %Y %H:%M")
}
