//! Shared HTTP seams for the message features, kept separate from message-root ownership.
#[cfg(test)]
mod search_header_tests;
#[cfg(test)]
mod private_provider_tests;
#[cfg(test)]
mod provider_batch_tests;
#[cfg(test)]
mod ws12_consumer_tests;
#[cfg(test)]
mod saved_tests;
#[cfg(test)]
mod coercion_tests;
#[cfg(test)]
mod scheduled_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod review_tests;
#[cfg(test)]
mod rescue_format_tests;

use crate::app::AppCtx;
use crate::concerns::{self, cast_integer, require_current_user};
use crate::controllers::presenters::page::db_error;
use campfire_db::{Message, Role, Room, Timestamp};
use campfire_kit::{Ctx, Error, Param, Redirect, Result, StatusCode, halt};

pub(crate) async fn room(c: &mut Ctx) -> Result<Room> {
    c.rescue_not_found();
    let (_, room) = concerns::set_room(c).await?;
    if room.deleted_at.is_some() {
        return Err(Error::NotFound);
    }
    Ok(room)
}

pub(crate) fn active_human(c: &Ctx) -> Result<()> {
    let user = require_current_user(c)?;
    if !user.is_active() || user.role == Role::Bot {
        return halt(concerns::head(StatusCode::FORBIDDEN));
    }
    Ok(())
}

pub(crate) async fn reachable_message(c: &mut Ctx) -> Result<Message> {
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

pub(crate) fn redirect(
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

pub(crate) fn errors_json(errors: &campfire_db::Errors) -> serde_json::Value {
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

pub(crate) fn sentence(errors: &campfire_db::Errors) -> String {
    campfire_views::helpers::to_sentence(&errors.full_messages(), " and ")
}

/// Explicit request `to_s` sites, including Ruby Array/Parameters coercion. This does
/// not change the kit-wide parameter API or ActiveRecord lookup casting.
pub(crate) fn param_string(value:&Param)->String {
    use campfire_richtext::ruby::{json_value_to_s,json_value_inspect};
    fn inspect(value:&Param)->String {
        match value {
            Param::Hash(_)=>format!("#<ActionController::Parameters {} permitted: false>",param_string(value)),
            Param::Array(_)=>param_string(value),
            value=>json_value_inspect(&value.to_json()),
        }
    }
    match value {
        Param::Array(values)=>format!("[{}]",values.iter().map(inspect).collect::<Vec<_>>().join(", ")),
        Param::Hash(map)=>format!("{{{}}}",map.iter().map(|(k,v)|format!("{} => {}",json_value_inspect(&serde_json::Value::String(k.clone())),inspect(v))).collect::<Vec<_>>().join(", ")),
        value=>json_value_to_s(&value.to_json()),
    }
}

pub(crate) fn boolean(value: Option<&Param>) -> bool {
    !matches!(value, None | Some(Param::Null | Param::Bool(false)))
        && !matches!(
            value.and_then(Param::to_s).as_deref(),
            Some("" | "0" | "f" | "F" | "false" | "FALSE" | "off" | "OFF")
        )
}

pub(crate) async fn user_zone(c: &Ctx) -> Result<campfire_views::time::Zone> {
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
    Ok(campfire_views::time::Zone::for_user(name.as_deref()))
}

/// WS8bm2 builder calendar parser; slash-relative expressions use a separate entry
/// point. Unrecognized input and Ruby's invalid-calendar exception stay distinct.
pub(crate) fn parse_time(
    raw: &str,
    zone: &campfire_views::time::Zone,
    now: jiff::Timestamp,
) -> Result<Option<Timestamp>> {
    parse_time_checked(raw, zone, now).map_err(db_error)
}

pub(crate) fn parse_time_checked(raw: &str, zone: &campfire_views::time::Zone, now: jiff::Timestamp) -> campfire_db::Result<Option<Timestamp>> {
    campfire_db::slash_commands::time_parser::parse_calendar(raw, zone.tz(), Timestamp::from_jiff(now))
}

pub(crate) fn poll_view(
    conn: &campfire_db::Connection,
    app: &crate::app::App,
    id: i64,
    error: Option<String>,
) -> campfire_db::Result<campfire_views::messages::parts::Poll> {
    let poll = campfire_db::Poll::find(conn, id)?;
    let message = Message::find(conn, poll.message_id)?;
    let options = poll
        .options(conn)?
        .into_iter()
        .map(|option| campfire_views::messages::parts::PollOption {
            id: option.id,
            label: option.label,
        })
        .collect();
    let votes = poll
        .votes_with_names(conn)?
        .into_iter()
        .map(|(vote,name)| {
            campfire_views::messages::parts::PollVote {
                option_id: vote.poll_option_id,
                user_id: vote.user_id,
                user_name: name,
            }
        })
        .collect();
    Ok(campfire_views::messages::parts::Poll {
        id,
        room_id: message.room_id,
        anonymous: poll.anonymous,
        multiple: poll.multiple,
        closed: poll.closed(app.db.env().now()),
        closes_at: poll.closes_at.map(|time| time.jiff()),
        options,
        votes,
        vote_error: error,
    })
}

/// Rails JSON encodes Time in UTC with millisecond precision.
pub(crate) fn json_time(time: Timestamp) -> String {
    json_time_in_zone(time, &jiff::tz::TimeZone::UTC)
}
pub(crate) fn json_time_in_zone(time: Timestamp, zone: &jiff::tz::TimeZone) -> String {
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
mod slash_tests;
#[cfg(test)]
mod links_files_tests;
#[cfg(test)]
mod reminder_tests;
#[cfg(test)]
mod quote_integration_tests;
#[cfg(test)]
mod root_cache_tests;
#[cfg(test)]
mod panel_tests;
#[cfg(test)]
mod pin_poll_scaling_tests;
#[cfg(test)]
mod exceptional_input_tests;
#[cfg(test)]
mod slash_named_tests;
#[cfg(test)]
mod date_tests;

#[cfg(test)]
mod provider_tests;

#[cfg(test)]
mod composer_tests;

#[cfg(test)]
mod older_provider_tests;

#[cfg(test)]
mod bounded_provider_tests;

#[cfg(test)]
mod mapped_provider_tests;

#[cfg(test)]
mod older_owner_tests;

#[cfg(test)]
mod older_calendar_tests;
#[cfg(test)]
mod older_embed_job_tests;

#[cfg(test)]
mod older_calendar_execution_tests;
#[cfg(test)]
mod older_embed_children_tests;
#[cfg(test)]
mod older_embed_failure_tests;

#[cfg(test)]
mod comparison_support;
#[cfg(test)]
mod final_state_sibling_tests;
#[cfg(test)]
mod calendar_retry_consumer_tests;
#[cfg(test)]
mod relative_split_input_tests;
#[cfg(test)]
mod container_input_tests;

#[cfg(test)]
mod wide_html_tests;

#[cfg(test)]
mod periodic_delivery_tests;

/// Presentation strings use WS11's shared wide-time renderer; view models never
/// narrow an accepted database timestamp through Jiff.
pub(crate) fn html_datetime(at: campfire_db::Timestamp, zone: &campfire_views::time::Zone) -> String {
    rails_compat::datetime::render(at, zone.tz(), true)
}
pub(crate) fn html_long(at: campfire_db::Timestamp, zone: &campfire_views::time::Zone) -> String {
    rails_compat::datetime::format(at, zone.tz(), "%B %d, %Y %H:%M")
}
