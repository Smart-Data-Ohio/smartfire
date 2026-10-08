//! Channel calendar pages on `/api/v1`. The classic presenters supply the read facts and
//! `rooms/events`' input parser and CalendarEvent writers supply the same validation,
//! invitations, announcements, broadcasts and calendar jobs as the classic forms.

use std::collections::HashMap;

use campfire_api_types as api;
use campfire_app::app::AppCtx;
use campfire_db::{CalendarEvent, Connection, NewCalendarEvent, Room, User};
use campfire_kit::{Ctx, Error, Param, Result, StatusCode, params::ParamMap};
use campfire_messages::controllers::message_features::active_human;
use campfire_rooms::controllers::rooms::events::{input, viewer_zone};
use campfire_views::events::{forms::FormView, pages::PageEvent};
use campfire_web::concerns::{cast_integer, require_current_user};
use campfire_web::controllers::presenters::{events as pages, page};
use serde::de::DeserializeOwned;
use serde_json::json;

use crate::dto;
use crate::endpoints::{before_actions, body, now, set_room};
use crate::error::{fail, record_invalid};

endpoint!(index => list_events);
endpoint!(new => new_event);
endpoint!(create => create_event);
endpoint!(show => show_event);
endpoint!(edit => edit_event);
endpoint!(update => update_event);
endpoint!(cancel => cancel_event);

/// `rooms/events#scheduled_room`: event URLs never join an open room, and bots and inactive
/// people cannot read the calendar or answer its attendance form.
async fn scheduled_room(c: &mut Ctx) -> Result<(Room, User)> {
    before_actions(c).await?;
    let (_, room) = set_room(c).await?;
    active_human(c)?;
    Ok((room, require_current_user(c)?.clone()))
}

pub(crate) async fn scoped_event(c: &mut Ctx) -> Result<(Room, CalendarEvent, User)> {
    let (room, viewer) = scheduled_room(c).await?;
    let event_id = c
        .param_str("event_id")
        .and_then(cast_integer)
        .ok_or(Error::NotFound)?;
    let (room_id, viewer_id) = (room.id, viewer.id);
    let event = c
        .app()
        .db
        .read(move |conn| CalendarEvent::find_visible(conn, room_id, event_id, viewer_id))
        .await
        .map_err(page::db_error)?;
    Ok((room, event, viewer))
}

fn manager(c: &mut Ctx, event: &CalendarEvent, viewer: &User, cancelling: bool) -> Result<()> {
    let allowed = if cancelling {
        event.cancellable_by(Some(viewer))
    } else {
        event.manageable_by(Some(viewer))
    };
    if allowed {
        Ok(())
    } else {
        Err(fail(
            c,
            api::ApiError::Forbidden {
                message: "Not allowed".into(),
            },
        ))
    }
}

fn recurrence(rule: &str) -> Option<api::EventRecurrenceRule> {
    Some(match rule {
        "daily" => api::EventRecurrenceRule::Daily,
        "weekly" => api::EventRecurrenceRule::Weekly,
        "biweekly" => api::EventRecurrenceRule::Biweekly,
        "monthly" => api::EventRecurrenceRule::Monthly,
        _ => return None,
    })
}

fn row(page: &PageEvent, event: &CalendarEvent) -> api::ChannelEvent {
    api::ChannelEvent {
        id: event.id,
        room_id: event.room_id,
        title: page.card.title.clone(),
        starts_at: dto::time(event.starts_at),
        ends_at: event.ends_at.map(dto::time),
        time_zone: page.card.time_zone.clone(),
        zone_label: page.zone_label(),
        organizer_name: page.card.organizer_name.clone(),
        venue: page.venue.as_ref().map(|venue| api::EventVenue {
            room_id: venue.id,
            name: venue.name.clone(),
            kind: if venue.stage {
                api::RoomKind::Stage
            } else {
                api::RoomKind::Voice
            },
            member: venue.member,
            live_user: venue.live_user.clone(),
        }),
        counts: api::EventCounts {
            going: page.going,
            maybe: page.maybe,
            declined: page.declined,
        },
        recurrence_label: page.recurrence_label.clone(),
        remaining_occurrences: page.remaining,
        cancelled: page.card.cancelled,
        series: page.card.series,
    }
}

fn detail(
    conn: &Connection,
    room: &Room,
    viewer: &User,
    event: &CalendarEvent,
) -> campfire_db::Result<api::EventDetail> {
    let view = pages::show(conn, room, viewer, event)?;
    let page = view.event;
    Ok(api::EventDetail {
        room_id: room.id,
        room_name: view.room_name,
        event: row(&page, event),
        description_html: page.description_html,
        manageable: page.manageable,
        respondable: page.respondable,
        current_response: page
            .current_response
            .as_deref()
            .and_then(crate::cards::response_of),
        head: page.head,
        can_apply_to_future: page.head || (event.series() && page.next.is_some()),
        previous_occurrence_id: page.previous,
        next_occurrence_id: page.next,
        recurrence_phrase: page.recurrence_phrase,
        recurrence_until: page.recurrence_until,
        meet_link: page.card.meet_link,
        calendar_copy: page.calendar_copy,
        attendees: page
            .attendances
            .into_iter()
            .filter_map(|attendee| {
                crate::cards::response_of(&attendee.response).map(|response| api::EventAttendee {
                    name: attendee.name,
                    response,
                })
            })
            .collect(),
    })
}

async fn list_events(c: &mut Ctx) -> Result {
    let (room, viewer) = scheduled_room(c).await?;
    let as_of = now(c);
    let list = c
        .app()
        .db
        .read(move |conn| {
            let view = pages::index(conn, &room, &viewer, as_of)?;
            // The classic presenter owns grouping and order. One extra bulk read preserves
            // the instants' subsecond precision instead of re-parsing its calendar labels.
            let events: HashMap<_, _> = CalendarEvent::for_room(conn, room.id)?
                .into_iter()
                .map(|event| (event.id, event))
                .collect();
            let rows = |pages: Vec<PageEvent>| -> campfire_db::Result<Vec<api::ChannelEvent>> {
                pages
                    .iter()
                    .map(|page| {
                        let event = events
                            .get(&page.card.id)
                            .ok_or(campfire_db::Error::RecordNotFound("Event"))?;
                        Ok(row(page, event))
                    })
                    .collect()
            };
            Ok(api::EventList {
                room_id: room.id,
                room_name: view.room_name,
                room_kind: dto::room_kind(room.room_type),
                may_create: true,
                upcoming: rows(view.upcoming)?,
                past: rows(view.past)?,
                cancelled: rows(view.cancelled)?,
            })
        })
        .await
        .map_err(page::db_error)?;
    c.json(StatusCode::OK, &list)
}

fn form(view: FormView) -> api::EventForm {
    let editing = view.id.is_some();
    let mut repeat_options = Vec::new();
    if !editing {
        repeat_options.push(api::EventRepeatOption {
            value: None,
            label: "Does not repeat".into(),
        });
    }
    for (value, label) in [
        (api::EventRecurrenceRule::Daily, "Daily"),
        (api::EventRecurrenceRule::Weekly, "Weekly"),
        (api::EventRecurrenceRule::Biweekly, "Every two weeks"),
        (api::EventRecurrenceRule::Monthly, "Monthly"),
    ] {
        repeat_options.push(api::EventRepeatOption {
            value: Some(value),
            label: label.into(),
        });
    }
    api::EventForm {
        room_id: view.room_id,
        room_name: view.room_name,
        event_id: view.id,
        meet_available: view.meet_link.is_none(),
        values: api::EventValues {
            title: view.title,
            description: view.description,
            starts_at: view.starts_at,
            ends_at: view.ends_at,
            time_zone: view.time_zone,
            venue_room_id: view.venue_room_id,
            recurrence_rule: view.recurrence_rule.as_deref().and_then(recurrence),
            recurrence_until: view.recurrence_until,
            meet_link_requested: view.meet_link_requested,
            meet_link: view.meet_link,
        },
        venues: view
            .venues
            .into_iter()
            .map(|venue| api::EventVenueOption {
                room_id: venue.id,
                name: venue.name,
                kind: if venue.stage {
                    api::RoomKind::Stage
                } else {
                    api::RoomKind::Voice
                },
            })
            .collect(),
        repeat_options,
        limits: api::EventLimits {
            title_max_length: 255,
            max_occurrences: campfire_db::models::calendar_event::recurrence::MAX_OCCURRENCES
                as i64,
            max_recurrence_years: 1,
        },
        series: view.series,
        head: view.head,
        rule_editable: !editing || view.head,
        scope_options: if editing && view.series {
            vec![
                api::EventScope::ThisEvent,
                api::EventScope::ThisAndFollowing,
            ]
        } else {
            Vec::new()
        },
    }
}

async fn render_form(
    c: &mut Ctx,
    room: Room,
    viewer: User,
    values: NewCalendarEvent,
    event: Option<CalendarEvent>,
) -> Result {
    let view = c
        .app()
        .db
        .read(move |conn| {
            pages::form(
                conn,
                &room,
                &viewer,
                &values,
                event.as_ref(),
                &campfire_db::Errors::default(),
                Some(values.title.clone()),
            )
            .map(form)
        })
        .await
        .map_err(page::db_error)?;
    c.json(StatusCode::OK, &view)
}

async fn new_event(c: &mut Ctx) -> Result {
    let (room, viewer) = scheduled_room(c).await?;
    let zone = viewer_zone(c, viewer.id).await?;
    let mut fields = serde_json::Map::new();
    for (wire, classic) in [
        ("title", "title"),
        ("startsAt", "starts_at"),
        ("timeZone", "time_zone"),
    ] {
        if let Some(value) = c.param_str(wire) {
            fields.insert(classic.into(), json!(value));
        }
    }
    let params = if fields.is_empty() {
        ParamMap::default()
    } else {
        let Param::Hash(params) = Param::from_json(json!({"event": fields})) else {
            unreachable!()
        };
        params
    };
    let values = input::prefill(&params, &zone, now(c), room.id, viewer.id)?;
    render_form(c, room, viewer, values, None).await
}

async fn show_event(c: &mut Ctx) -> Result {
    let (room, event, viewer) = scoped_event(c).await?;
    render_detail(c, room, event.id, viewer, StatusCode::OK).await
}

async fn edit_event(c: &mut Ctx) -> Result {
    let (room, event, viewer) = scoped_event(c).await?;
    manager(c, &event, &viewer, false)?;
    let values = input::attempted(&Default::default(), &event);
    render_form(c, room, viewer, values, Some(event)).await
}

async fn event_body<T: DeserializeOwned>(c: &mut Ctx) -> Result<(T, ParamMap)> {
    let value: serde_json::Value = body(c).await?;
    let input = serde_json::from_value(value.clone()).map_err(|error| {
        fail(
            c,
            api::ApiError::Validation {
                message: format!("The request body isn't valid: {error}"),
                fields: Default::default(),
            },
        )
    })?;
    let mut fields = serde_json::Map::new();
    for (wire, classic) in [
        ("title", "title"),
        ("description", "description"),
        ("startsAt", "starts_at"),
        ("endsAt", "ends_at"),
        ("timeZone", "time_zone"),
        ("venueRoomId", "venue_room_id"),
        ("recurrenceRule", "recurrence_rule"),
        ("recurrenceUntil", "recurrence_until"),
        ("meetLinkRequested", "meet_link_requested"),
    ] {
        // A follower's classic form omits recurrence controls, and an existing Meet link
        // replaces its request checkbox. Preserve missing attributes rather than clearing.
        if let Some(value) = value.get(wire) {
            let value = if wire == "description" && value.is_null() {
                json!("")
            } else {
                value.clone()
            };
            fields.insert(classic.into(), value);
        }
    }
    let Param::Hash(params) = Param::from_json(json!({"event": fields})) else {
        unreachable!()
    };
    Ok((input, params))
}

async fn create_event(c: &mut Ctx) -> Result {
    let (room, viewer) = scheduled_room(c).await?;
    let (_, params) = event_body::<api::CreateEvent>(c).await?;
    let zone = viewer_zone(c, viewer.id).await?;
    let changes = input::attributes(&params, None, &zone, now(c))?;
    let values = input::new_attributes(changes, room.id, viewer.id);
    let outcome = c
        .app()
        .db
        .write_scoped(
            move || page::enter_time_zone(campfire_views::time::Zone::for_user(Some(&zone))),
            move |tx| {
                // Classic returns form errors only for the event itself. A failure in its
                // after-commit announcement still escapes as a server error.
                let errors = CalendarEvent::validate(tx.conn(), &values)?;
                if errors.is_empty() {
                    CalendarEvent::create(tx, values).map(Ok)
                } else {
                    Ok(Err(errors))
                }
            },
        )
        .await
        .map_err(page::db_error)?;
    match outcome {
        Ok(event) => render_detail(c, room, event.id, viewer, StatusCode::CREATED).await,
        Err(errors) => Err(fail(
            c,
            record_invalid(&errors, &[("venue", "venueRoomId")]),
        )),
    }
}

async fn render_detail(
    c: &mut Ctx,
    room: Room,
    event_id: i64,
    viewer: User,
    status: StatusCode,
) -> Result {
    let result = c
        .app()
        .db
        .read(move |conn| {
            let event = CalendarEvent::find_visible(conn, room.id, event_id, viewer.id)?;
            detail(conn, &room, &viewer, &event)
        })
        .await
        .map_err(page::db_error)?;
    c.json(status, &result)
}

fn write_error(c: &mut Ctx, error: campfire_db::Error) -> Error {
    match error {
        campfire_db::Error::RecordInvalid(errors) => {
            fail(c, record_invalid(&errors, &[("venue", "venueRoomId")]))
        }
        other => page::db_error(other),
    }
}

async fn update_event(c: &mut Ctx) -> Result {
    let (room, event, viewer) = scoped_event(c).await?;
    manager(c, &event, &viewer, false)?;
    let (input, params) = event_body::<api::UpdateEvent>(c).await?;
    let scope = input.update_scope.unwrap_or_default();
    let zone = viewer_zone(c, viewer.id).await?;
    let changes = input::attributes(&params, Some(&event), &zone, now(c))?;
    let (event_id, actor) = (event.id, viewer.id);
    c.app()
        .db
        .write_scoped(
            move || page::enter_time_zone(campfire_views::time::Zone::for_user(Some(&zone))),
            move |tx| CalendarEvent::update_with_scope(tx, event_id, changes, &scope, Some(actor)),
        )
        .await
        .map_err(|error| write_error(c, error))?;
    render_detail(c, room, event_id, viewer, StatusCode::OK).await
}

async fn cancel_event(c: &mut Ctx) -> Result {
    let (room, event, viewer) = scoped_event(c).await?;
    manager(c, &event, &viewer, true)?;
    let input: api::CancelEvent = body(c).await?;
    let scope = input.cancel_scope.unwrap_or_default();
    let zone = viewer_zone(c, viewer.id).await?;
    let (event_id, actor) = (event.id, viewer.id);
    c.app()
        .db
        .write_scoped(
            move || page::enter_time_zone(campfire_views::time::Zone::for_user(Some(&zone))),
            move |tx| CalendarEvent::cancel_with_scope(tx, event_id, &scope, Some(actor)),
        )
        .await
        .map_err(|error| write_error(c, error))?;
    render_detail(c, room, event_id, viewer, StatusCode::OK).await
}
