//! Channel event JSON reads and writes against the classic controller over identical frozen
//! databases. Real writer callbacks and durable jobs run; remote job workers are stopped.

use std::time::Duration;

use axum::http::{Method, StatusCode};
use campfire_api_types as api;
use campfire_db::{CalendarEvent, Connection, Room, User};
use campfire_views::events::pages::PageEvent;
use serde_json::{Value, json};

use super::api_tests::{ALL_PETS, Sync, get, json_body, parse, serve, tag};
use crate::controllers::presenters::test_support::{
    ALL_TALK, BENDER, BENDER_KEY, Browser, DAVID, DIRECT_DAVID_JASON, JASON, KEVIN, Reply, Req,
    TestApp,
};

const DESIGNERS: i64 = 654632876;
const LAUNCH: i64 = 390339825;
const CANCELLED: i64 = 439674037;
const VENUE: i64 = 800000001;
const PAST: i64 = 800000002;
const HEAD: i64 = 800000003;
const FOLLOWER: i64 = 800000004;
const LAST: i64 = 800000005;
const CARD_MESSAGE: i64 = 935962054;

async fn app() -> Option<TestApp> {
    Some(
        TestApp::boot_frozen_with_env(&[("SPA_ENABLED", "1")])
            .await?
            .without_job_runner()
            .await,
    )
}

async fn sql(a: &TestApp, statements: &'static str) {
    a.db()
        .write(move |tx| {
            tx.conn().execute_batch(statements)?;
            Ok(())
        })
        .await
        .unwrap();
}

fn event_body(title: &str, start: &str, end: Option<&str>) -> Value {
    json!({
        "title": title, "description": "Agenda\nBring ideas", "startsAt": start,
        "endsAt": end, "timeZone": "America/New_York", "venueRoomId": null,
        "recurrenceRule": null, "recurrenceUntil": null, "meetLinkRequested": false,
    })
}

async fn send(b: &mut Browser<'_>, method: Method, path: &str, body: &Value) -> Reply {
    b.write(json_body(method, path, body)).await
}

fn ok(reply: &Reply) -> Value {
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    parse(reply)
}

fn wire_field(classic: &str) -> &str {
    match classic {
        "starts_at" => "startsAt",
        "ends_at" => "endsAt",
        "time_zone" => "timeZone",
        "venue" => "venueRoomId",
        "recurrence_rule" => "recurrenceRule",
        "recurrence_until" => "recurrenceUntil",
        other => other,
    }
}

fn fields(reply: &Reply) -> std::collections::BTreeMap<String, Vec<String>> {
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.text()
    );
    let api::ApiError::Validation { fields, .. } = parse::<api::ApiErrorResponse>(reply).error
    else {
        panic!("{}", reply.text())
    };
    fields
}

/// Use actual classic form encoding, including its hidden false checkbox and optional scope.
fn classic_form(method: Method, path: &str, body: &Value) -> Req {
    let names = [
        ("title", "title"),
        ("description", "description"),
        ("startsAt", "starts_at"),
        ("endsAt", "ends_at"),
        ("timeZone", "time_zone"),
        ("venueRoomId", "venue_room_id"),
        ("recurrenceRule", "recurrence_rule"),
        ("recurrenceUntil", "recurrence_until"),
        ("meetLinkRequested", "meet_link_requested"),
    ];
    let mut owned = Vec::new();
    for (json_key, form_key) in names {
        if let Some(value) = body.get(json_key) {
            let text = match value {
                Value::Null => String::new(),
                Value::Bool(value) => if *value { "1" } else { "0" }.to_string(),
                Value::String(value) => value.clone(),
                other => other.to_string(),
            };
            owned.push((format!("event[{form_key}]"), text));
        }
    }
    for (json_key, form_key) in [
        ("updateScope", "update_scope"),
        ("cancelScope", "cancel_scope"),
    ] {
        if let Some(value) = body[json_key].as_str() {
            owned.push((form_key.to_string(), value.to_string()));
        }
    }
    let pairs: Vec<_> = owned
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect();
    Req::new(method, path).form(&pairs)
}

fn rows(conn: &Connection, query: &str) -> campfire_db::Result<Value> {
    let mut statement = conn.prepare(query)?;
    let columns = statement.column_count();
    let values = statement
        .query_map([], |row| {
            (0..columns)
                .map(|column| {
                    Ok(match row.get_ref(column)? {
                        rusqlite::types::ValueRef::Null => Value::Null,
                        rusqlite::types::ValueRef::Integer(value) => json!(value),
                        rusqlite::types::ValueRef::Real(value) => json!(value),
                        rusqlite::types::ValueRef::Text(value) => {
                            json!(std::str::from_utf8(value).unwrap())
                        }
                        rusqlite::types::ValueRef::Blob(_) => {
                            panic!("event parity query must not read secrets/blobs")
                        }
                    })
                })
                .collect::<rusqlite::Result<Vec<_>>>()
                .map(Value::Array)
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(Value::Array(values))
}

async fn snapshot(a: &TestApp) -> Value {
    a.db().read(|conn| {
        Ok(json!({
            "events": rows(conn, "SELECT * FROM events ORDER BY id")?,
            "attendance": rows(conn, "SELECT * FROM event_attendances ORDER BY id")?,
            "calendarCopies": rows(conn, "SELECT * FROM event_calendar_entries ORDER BY id")?,
            "activity": rows(conn, "SELECT * FROM activity_items WHERE source_type='Event' ORDER BY id")?,
            "references": rows(conn, "SELECT * FROM event_references ORDER BY id")?,
            // A normal message allocates a random client id; every persisted content field is
            // compared, and the capture replaces only that corresponding random DOM key.
            "messages": rows(conn, "SELECT id,room_id,creator_id,markdown_source,created_at,updated_at FROM messages WHERE markdown_source LIKE 'Scheduled an event:%' ORDER BY id")?,
            "richtext": rows(conn, "SELECT * FROM action_text_rich_texts WHERE record_type='Message' ORDER BY id")?,
            "jobs": rows(conn, "SELECT job_class,arguments,queue_name,payload_version,status,run_at FROM background_jobs ORDER BY id")?,
        }))
    }).await.unwrap()
}

async fn frames(
    a: &TestApp,
    capture: &campfire_cable::pubsub::PublicationCapture,
) -> Vec<(String, String)> {
    tokio::time::timeout(
        Duration::from_secs(5),
        a.booted.app.broadcasts.settle_sync(),
    )
    .await
    .expect("deferred sync readers settled");
    let keys = a.db().read(|conn| {
        Ok(conn.prepare("SELECT client_message_id FROM messages WHERE markdown_source LIKE 'Scheduled an event:%' ORDER BY id")?
            .query_map([], |r| r.get::<_, String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?)
    }).await.unwrap();
    capture
        .take()
        .into_iter()
        .map(|(stream, mut frame)| {
            for (index, key) in keys.iter().enumerate() {
                frame = frame.replace(key, &format!("announcement-{index}"));
            }
            (stream, frame)
        })
        .collect()
}

async fn same_state(
    classic: &TestApp,
    next: &TestApp,
    classic_capture: &campfire_cable::pubsub::PublicationCapture,
    next_capture: &campfire_cable::pubsub::PublicationCapture,
    step: &str,
) -> Vec<(String, String)> {
    assert_eq!(
        snapshot(classic).await,
        snapshot(next).await,
        "{step}: persisted rows and durable jobs"
    );
    let expected = frames(classic, classic_capture).await;
    assert_eq!(
        expected,
        frames(next, next_capture).await,
        "{step}: real classic broadcasts"
    );
    expected
}

async fn arrange_callbacks(a: &TestApp) {
    sql(a, "DELETE FROM background_jobs;
        UPDATE memberships SET involvement='everything' WHERE room_id=654632876;
        INSERT OR IGNORE INTO google_accounts (user_id,email,created_at,updated_at) VALUES (127326141,'organizer@example.test','2026-03-02 16:00:00','2026-03-02 16:00:00');
        INSERT OR IGNORE INTO google_accounts (user_id,email,created_at,updated_at) VALUES (712064548,'attendee@example.test','2026-03-02 16:00:00','2026-03-02 16:00:00');
        INSERT INTO rooms (id,name,type,creator_id,created_at,updated_at) VALUES (800000001,'Planning Stage','Rooms::Stage',127326141,'2026-03-02 16:00:00','2026-03-02 16:00:00');
        INSERT INTO memberships (room_id,user_id,involvement,created_at,updated_at) VALUES (800000001,127326141,'everything','2026-03-02 16:00:00','2026-03-02 16:00:00');")
        .await;
}

async fn create_pair(classic: &mut Browser<'_>, next: &mut Browser<'_>, body: &Value) -> i64 {
    let path = format!("/rooms/{DESIGNERS}/events");
    let expected = classic.write(classic_form(Method::POST, &path, body)).await;
    assert_eq!(expected.status, StatusCode::FOUND, "{}", expected.text());
    let created = send(next, Method::POST, &format!("/api/v1{path}"), body).await;
    assert_eq!(created.status, StatusCode::CREATED, "{}", created.text());
    let detail: Value = parse(&created);
    let id = detail["event"]["id"].as_i64().expect("created event id");
    assert!(
        expected
            .location()
            .unwrap()
            .ends_with(&format!("{path}/{id}"))
    );
    id
}

async fn update_pair(classic: &mut Browser<'_>, next: &mut Browser<'_>, id: i64, body: &Value) {
    let path = format!("/rooms/{DESIGNERS}/events/{id}");
    let expected = classic
        .write(classic_form(Method::PATCH, &path, body))
        .await;
    assert_eq!(expected.status, StatusCode::FOUND, "{}", expected.text());
    assert!(expected.location().unwrap().ends_with(&path));
    let detail = ok(&send(next, Method::PATCH, &format!("/api/v1{path}"), body).await);
    assert_eq!(detail["event"]["id"], id);
}

async fn respond_pair(
    classic: &mut Browser<'_>,
    next: &mut Browser<'_>,
    id: i64,
    response: &str,
    future: bool,
) {
    let path = format!("/rooms/{DESIGNERS}/events/{id}/attendance");
    let expected = classic
        .write(Req::new(Method::PATCH, &path).form(&[
            ("response", response),
            ("apply_to_future", if future { "1" } else { "0" }),
        ]))
        .await;
    assert_eq!(expected.status, StatusCode::FOUND, "{}", expected.text());
    let attendance = ok(&send(
        next,
        Method::PUT,
        &format!("/api/v1{path}"),
        &json!({"response": response, "applyToFuture": future}),
    )
    .await);
    assert_eq!(attendance["response"], response);
}

async fn cancel_pair(classic: &mut Browser<'_>, next: &mut Browser<'_>, id: i64, scope: &str) {
    let path = format!("/rooms/{DESIGNERS}/events/{id}/cancel");
    let body = json!({"cancelScope": scope});
    let expected = classic
        .write(classic_form(Method::PATCH, &path, &body))
        .await;
    assert_eq!(expected.status, StatusCode::FOUND, "{}", expected.text());
    let detail = ok(&send(next, Method::PATCH, &format!("/api/v1{path}"), &body).await);
    assert_eq!(detail["event"]["cancelled"], true);
}

#[tokio::test]
async fn spa_api_events_create_update_cancel_and_respond_match_classic_side_effects() {
    let (Some(classic), Some(next)) = (app().await, app().await) else {
        return;
    };
    arrange_callbacks(&classic).await;
    arrange_callbacks(&next).await;
    let mut old_david = classic.sign_in(DAVID).await;
    let mut new_david = next.sign_in(DAVID).await;
    let mut old_kevin = classic.sign_in(KEVIN).await;
    let mut new_kevin = next.sign_in(KEVIN).await;
    for browser in [
        &mut old_david,
        &mut new_david,
        &mut old_kevin,
        &mut new_kevin,
    ] {
        browser.authenticity_token().await;
    }
    let old_capture = classic.booted.app.cable.capture_every_publication();
    let new_capture = next.booted.app.cable.capture_every_publication();
    let mut body = event_body(
        "API parity planning",
        "2026-03-10T09:00",
        Some("2026-03-10T10:00"),
    );
    body["venueRoomId"] = json!(VENUE);
    body["meetLinkRequested"] = json!(true);
    let id = create_pair(&mut old_david, &mut new_david, &body).await;
    let publications = same_state(&classic, &next, &old_capture, &new_capture, "create").await;
    assert!(
        !publications.is_empty(),
        "creation publishes activity and channel announcement"
    );
    let state = snapshot(&next).await;
    assert!(
        state["jobs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row[0] == "Calendar::MeetLinkJob")
    );
    assert!(
        state["activity"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row.to_string().contains("event_invitation"))
    );
    assert!(
        !state["references"].as_array().unwrap().is_empty(),
        "announcement has an event card reference"
    );

    respond_pair(&mut old_kevin, &mut new_kevin, id, "maybe", false).await;
    same_state(&classic, &next, &old_capture, &new_capture, "respond").await;
    let before_repeat = snapshot(&next).await;
    respond_pair(&mut old_kevin, &mut new_kevin, id, "maybe", false).await;
    assert_eq!(
        snapshot(&next).await,
        before_repeat,
        "identical response enqueues nothing and changes no rows"
    );
    same_state(
        &classic,
        &next,
        &old_capture,
        &new_capture,
        "repeat response",
    )
    .await;

    body["title"] = json!("Revised planning");
    body["description"] = Value::Null;
    body["startsAt"] = json!("2026-03-10T11:00");
    body["endsAt"] = json!("2026-03-10T12:00");
    // Classic edits ignore supplied zone and parse local times in the event's stored zone.
    body["timeZone"] = json!("Hawaii");
    update_pair(&mut old_david, &mut new_david, id, &body).await;
    same_state(&classic, &next, &old_capture, &new_capture, "update").await;

    for a in [&classic, &next] {
        a.db().write(move |tx| {
            tx.conn().execute("INSERT INTO event_calendar_entries (event_id,user_id,google_event_id,created_at,updated_at) VALUES (?,?,'private-copy','2026-03-02 16:00:00','2026-03-02 16:00:00')", rusqlite::params![id,KEVIN])?;
            Ok(())
        }).await.unwrap();
    }
    cancel_pair(&mut old_david, &mut new_david, id, "this_event").await;
    same_state(&classic, &next, &old_capture, &new_capture, "cancel").await;
    let before_repeat = snapshot(&next).await;
    cancel_pair(&mut old_david, &mut new_david, id, "this_event").await;
    assert_eq!(
        snapshot(&next).await,
        before_repeat,
        "repeat cancellation is a no-op"
    );
    same_state(&classic, &next, &old_capture, &new_capture, "repeat cancel").await;

    let mut empty_description = event_body("Blank description", "2026-03-11T09:00", None);
    empty_description["description"] = Value::Null;
    let blank_id = create_pair(&mut old_david, &mut new_david, &empty_description).await;
    same_state(
        &classic,
        &next,
        &old_capture,
        &new_capture,
        "blank textarea create",
    )
    .await;
    let saved = next
        .db()
        .read(move |conn| Ok(CalendarEvent::find(conn, blank_id)?.description))
        .await
        .unwrap();
    assert_eq!(
        saved.as_deref(),
        Some(""),
        "JSON null has the classic blank textarea meaning"
    );
}

fn classic_row(conn: &Connection, row: &PageEvent) -> campfire_db::Result<Value> {
    let event = CalendarEvent::find(conn, row.card.id)?;
    Ok(json!({
        "id": row.card.id, "roomId": row.card.room_id, "title": row.card.title,
        "startsAt": event.starts_at.to_wire(), "endsAt": event.ends_at.map(|time|time.to_wire()),
        "timeZone": row.card.time_zone, "zoneLabel": row.zone_label(),
        "organizerName": row.card.organizer_name,
        "venue": row.venue.as_ref().map(|venue| json!({
            "roomId":venue.id,"name":venue.name,"kind":if venue.stage {"stage"} else {"voice"},
            "member":venue.member,"liveUser":venue.live_user,
        })),
        "counts":{"going":row.going,"maybe":row.maybe,"declined":row.declined},
        "recurrenceLabel":row.recurrence_label,"remainingOccurrences":row.remaining,
        "cancelled":row.card.cancelled,"series":row.card.series,
    }))
}

fn classic_detail(
    conn: &Connection,
    room: &Room,
    user: &User,
    event: &CalendarEvent,
) -> campfire_db::Result<Value> {
    let show = campfire_web::controllers::presenters::events::show(conn, room, user, event)?;
    let row = &show.event;
    Ok(json!({
        "roomId":room.id,"roomName":show.room_name,"event":classic_row(conn,row)?,
        "descriptionHtml":row.description_html,"manageable":row.manageable,"respondable":row.respondable,
        "currentResponse":row.current_response,"head":row.head,
        "canApplyToFuture":row.head || (row.card.series && row.next.is_some()),
        "previousOccurrenceId":row.previous,"nextOccurrenceId":row.next,
        "recurrencePhrase":row.recurrence_phrase,"recurrenceUntil":row.recurrence_until,
        "meetLink":row.card.meet_link,"calendarCopy":row.calendar_copy,
        "attendees":row.attendances.iter().map(|a|json!({"name":a.name,"response":a.response})).collect::<Vec<_>>(),
    }))
}

async fn arrange_reads(a: &TestApp) {
    arrange_callbacks(a).await;
    sql(a, "UPDATE events SET description='<strong>Safe</strong><script>secret()</script>\nNext line\n\nParagraph',venue_room_id=800000001,meet_link='https://meet.google.com/fixture' WHERE id=390339825;
        INSERT INTO streams (room_id,user_id,membership_id,quality,started_at,created_at,updated_at) SELECT 800000001,127326141,id,'high','2026-03-02 16:00:00','2026-03-02 16:00:00','2026-03-02 16:00:00' FROM memberships WHERE room_id=800000001 AND user_id=127326141;
        INSERT INTO events (id,room_id,organizer_id,title,starts_at,time_zone,created_at,updated_at) VALUES (800000002,654632876,127326141,'Past planning','2026-03-01 10:00:00','UTC','2026-03-02 16:00:00','2026-03-02 16:00:00');
        INSERT INTO events (id,room_id,organizer_id,title,starts_at,ends_at,time_zone,series_id,recurrence_rule,recurrence_until,created_at,updated_at) VALUES
        (800000003,654632876,127326141,'Weekly planning','2026-03-03 14:00:00','2026-03-03 15:00:00','America/New_York',800000003,'weekly','2026-03-17','2026-03-02 16:00:00','2026-03-02 16:00:00'),
        (800000004,654632876,127326141,'Weekly planning','2026-03-10 13:00:00','2026-03-10 14:00:00','America/New_York',800000003,'weekly','2026-03-17','2026-03-02 16:00:00','2026-03-02 16:00:00'),
        (800000005,654632876,127326141,'Weekly planning','2026-03-17 13:00:00','2026-03-17 14:00:00','America/New_York',800000003,'weekly','2026-03-17','2026-03-02 16:00:00','2026-03-02 16:00:00');
        INSERT INTO event_calendar_entries (event_id,user_id,google_event_id,created_at,updated_at) VALUES (390339825,127326141,'private-copy','2026-03-02 16:00:00','2026-03-02 16:00:00');")
        .await;
}

#[tokio::test]
async fn spa_api_events_list_and_details_match_every_classic_presenter_fact() {
    let Some(a) = app().await else { return };
    arrange_reads(&a).await;
    let now = a.booted.app.db.env().now();
    for user_id in [DAVID, KEVIN] {
        let mut browser = a.sign_in(user_id).await;
        let reply = browser
            .send(get(&format!("/api/v1/rooms/{DESIGNERS}/events")))
            .await;
        let list = ok(&reply);
        // Contract deserialization proves names and tagged/null shapes, not just ad-hoc JSON.
        let _: api::EventList = parse(&reply);
        let expected = a.db().read(move |conn| {
            let room = Room::find(conn, DESIGNERS)?;
            let user = User::find(conn, user_id)?;
            let view = campfire_web::controllers::presenters::events::index(conn,&room,&user,now)?;
            Ok(json!({
                "roomId":DESIGNERS,"roomName":view.room_name,"roomKind":"closed","mayCreate":true,
                "upcoming":view.upcoming.iter().map(|row|classic_row(conn,row)).collect::<campfire_db::Result<Vec<_>>>()?,
                "past":view.past.iter().map(|row|classic_row(conn,row)).collect::<campfire_db::Result<Vec<_>>>()?,
                "cancelled":view.cancelled.iter().map(|row|classic_row(conn,row)).collect::<campfire_db::Result<Vec<_>>>()?,
            }))
        }).await.unwrap();
        assert_eq!(list, expected, "list for viewer {user_id}");
        assert_eq!(
            list["upcoming"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|row| row["series"] == true)
                .count(),
            1
        );
        let series = list["upcoming"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["id"] == HEAD)
            .unwrap();
        assert_eq!(series["remainingOccurrences"], 3);
        assert_eq!(list["past"][0]["id"], PAST);
        for event_id in [LAUNCH, CANCELLED, PAST, HEAD, FOLLOWER, LAST] {
            let reply = browser
                .send(get(&format!("/api/v1/rooms/{DESIGNERS}/events/{event_id}")))
                .await;
            let detail = ok(&reply);
            let _: api::EventDetail = parse(&reply);
            let expected = a
                .db()
                .read(move |conn| {
                    classic_detail(
                        conn,
                        &Room::find(conn, DESIGNERS)?,
                        &User::find(conn, user_id)?,
                        &CalendarEvent::find(conn, event_id)?,
                    )
                })
                .await
                .unwrap();
            assert_eq!(detail, expected, "detail {event_id} for viewer {user_id}");
            if event_id == LAUNCH {
                assert!(
                    detail["descriptionHtml"]
                        .as_str()
                        .unwrap()
                        .contains("<strong>Safe</strong>")
                );
                assert!(
                    !detail["descriptionHtml"]
                        .as_str()
                        .unwrap()
                        .contains("<script>")
                );
                assert_eq!(detail["calendarCopy"], user_id == DAVID);
                assert_eq!(detail["event"]["venue"]["member"], user_id == DAVID);
                assert_eq!(detail["event"]["venue"]["liveUser"], "David");
            }
        }
    }
}

#[tokio::test]
async fn spa_api_events_form_defaults_values_and_scopes_match_classic_forms() {
    let Some(a) = app().await else { return };
    arrange_reads(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let reply = david
        .send(get(&format!("/api/v1/rooms/{DESIGNERS}/events/new")))
        .await;
    let new = ok(&reply);
    let _: api::EventForm = parse(&reply);
    assert_eq!(
        new["values"],
        json!({
            "title":"","description":null,"startsAt":null,"endsAt":null,"timeZone":"UTC",
            "venueRoomId":null,"recurrenceRule":null,"recurrenceUntil":null,"meetLinkRequested":false,"meetLink":null,
        })
    );
    assert_eq!(
        new["limits"],
        json!({"titleMaxLength":255,"maxOccurrences":52,"maxRecurrenceYears":1})
    );
    assert_eq!(
        new["repeatOptions"],
        json!([
            {"value":null,"label":"Does not repeat"},{"value":"daily","label":"Daily"},
            {"value":"weekly","label":"Weekly"},{"value":"biweekly","label":"Every two weeks"},{"value":"monthly","label":"Monthly"},
        ])
    );
    assert_eq!(
        (new["meetAvailable"].clone(), new["ruleEditable"].clone()),
        (json!(true), json!(true))
    );
    assert_eq!(new["scopeOptions"], json!([]));
    let expected_venues = a.db().read(|conn| {
        let values = campfire_db::NewCalendarEvent {room_id:DESIGNERS,organizer_id:DAVID,time_zone:"UTC".into(),..Default::default()};
        let view = campfire_web::controllers::presenters::events::form(conn,&Room::find(conn,DESIGNERS)?,&User::find(conn,DAVID)?,&values,None,&Default::default(),None)?;
        Ok(view.venues.into_iter().map(|venue|json!({"roomId":venue.id,"name":venue.name,"kind":if venue.stage {"stage"} else {"voice"}})).collect::<Vec<_>>())
    }).await.unwrap();
    assert_eq!(
        new["venues"],
        json!(expected_venues),
        "same current-user venue options and order"
    );
    let prefilled = ok(&david.send(get(&format!("/api/v1/rooms/{DESIGNERS}/events/new?title=%20Prefilled%20&startsAt=2026-03-10T09%3A00&timeZone=America%2FNew_York"))).await);
    let classic_prefill = david.classic_page(&format!("/rooms/{DESIGNERS}/events/new?event[title]=%20Prefilled%20&event[starts_at]=2026-03-10T09%3A00&event[time_zone]=America%2FNew_York")).await;
    assert_eq!(
        classic_prefill.status,
        StatusCode::OK,
        "{}",
        classic_prefill.text()
    );
    assert_eq!(prefilled["values"]["title"], "Prefilled");
    assert_eq!(prefilled["values"]["timeZone"], "America/New_York");
    let local = prefilled["values"]["startsAt"].as_str().unwrap();
    assert!(
        classic_prefill
            .text()
            .contains(&format!("value=\"{local}\"")),
        "same prefilled datetime-local value"
    );
    for event_id in [LAUNCH, HEAD, FOLLOWER] {
        let form = ok(&david
            .send(get(&format!(
                "/api/v1/rooms/{DESIGNERS}/events/{event_id}/edit"
            )))
            .await);
        let expected = a.db().read(move |conn| {
            let event=CalendarEvent::find(conn,event_id)?;
            let attributes=campfire_db::NewCalendarEvent {
                room_id:event.room_id,organizer_id:event.organizer_id,title:event.title.clone(),description:event.description.clone(),
                starts_at:Some(event.starts_at),ends_at:event.ends_at,time_zone:event.time_zone.clone(),venue_room_id:event.venue_room_id,
                recurrence_rule:event.recurrence_rule.clone(),recurrence_until:event.recurrence_until,meet_link_requested:event.meet_link_requested,
            };
            let view=campfire_web::controllers::presenters::events::form(conn,&Room::find(conn,DESIGNERS)?,&User::find(conn,DAVID)?,&attributes,Some(&event),&Default::default(),Some(event.title.clone()))?;
            Ok(json!({
                "title":view.title,"description":view.description,"startsAt":view.starts_at,"endsAt":view.ends_at,"timeZone":view.time_zone,
                "venueRoomId":view.venue_room_id,"recurrenceRule":view.recurrence_rule,"recurrenceUntil":view.recurrence_until,
                "meetLinkRequested":view.meet_link_requested,"meetLink":view.meet_link,
            }))
        }).await.unwrap();
        assert_eq!(
            form["values"], expected,
            "classic editable values for {event_id}"
        );
        assert_eq!(form["ruleEditable"], event_id == HEAD);
        assert_eq!(
            form["scopeOptions"],
            if event_id == LAUNCH {
                json!([])
            } else {
                json!(["this_event", "this_and_following"])
            }
        );
        assert_eq!(form["meetAvailable"], event_id != LAUNCH);
        if event_id == HEAD {
            assert!(
                form["repeatOptions"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|row| !row["value"].is_null()),
                "head cannot remove recurrence"
            );
        }
    }
    let mut jason = a.sign_in(JASON).await;
    let hidden_venue = ok(&jason
        .send(get(&format!(
            "/api/v1/rooms/{DESIGNERS}/events/{LAUNCH}/edit"
        )))
        .await);
    assert!(
        hidden_venue["venues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|venue| venue["roomId"] == VENUE),
        "an administrator's edit retains the stored venue outside their memberships"
    );
}

async fn series_ids(a: &TestApp, head: i64) -> Vec<i64> {
    a.db()
        .read(move |conn| {
            Ok(CalendarEvent::find(conn, head)?
                .series_events(conn)?
                .into_iter()
                .map(|event| event.id)
                .collect())
        })
        .await
        .unwrap()
}

async fn event_responses(a: &TestApp, head: i64, user: i64) -> Vec<(i64, Option<String>)> {
    a.db()
        .read(move |conn| {
            CalendarEvent::find(conn, head)?
                .series_events(conn)?
                .into_iter()
                .map(|event| Ok((event.id, event.response_for(conn, Some(user))?)))
                .collect()
        })
        .await
        .unwrap()
}

async fn editable_body(browser: &mut Browser<'_>, id: i64, scope: &str) -> Value {
    let mut values = ok(&browser
        .send(get(&format!("/api/v1/rooms/{DESIGNERS}/events/{id}/edit")))
        .await)["values"]
        .clone();
    values["updateScope"] = json!(scope);
    values
}

#[tokio::test]
async fn spa_api_events_series_scopes_and_response_propagation_match_classic() {
    let (Some(classic), Some(next)) = (app().await, app().await) else {
        return;
    };
    arrange_callbacks(&classic).await;
    arrange_callbacks(&next).await;
    let mut old_david = classic.sign_in(DAVID).await;
    let mut new_david = next.sign_in(DAVID).await;
    let mut old_kevin = classic.sign_in(KEVIN).await;
    let mut new_kevin = next.sign_in(KEVIN).await;
    for browser in [
        &mut old_david,
        &mut new_david,
        &mut old_kevin,
        &mut new_kevin,
    ] {
        browser.authenticity_token().await;
    }
    let old_capture = classic.booted.app.cable.capture_every_publication();
    let new_capture = next.booted.app.cable.capture_every_publication();
    let mut body = event_body(
        "Weekly parity",
        "2026-03-03T09:00",
        Some("2026-03-03T10:00"),
    );
    body["recurrenceRule"] = json!("weekly");
    body["recurrenceUntil"] = json!("2026-03-24");
    let head = create_pair(&mut old_david, &mut new_david, &body).await;
    same_state(
        &classic,
        &next,
        &old_capture,
        &new_capture,
        "create weekly series",
    )
    .await;
    let ids = series_ids(&next, head).await;
    assert_eq!(ids.len(), 4);
    let invitations=next.db().read(move |conn| {
        Ok(conn.query_row("SELECT COUNT(*) FROM activity_items WHERE source_type='Event' AND event_type='event_invitation' AND source_id IN (SELECT id FROM events WHERE series_id=?) AND source_id<>?",[head,head],|r|r.get::<_,i64>(0))?)
    }).await.unwrap();
    assert_eq!(
        invitations, 0,
        "a series sends invitations only for its head"
    );
    respond_pair(&mut old_kevin, &mut new_kevin, head, "maybe", false).await;
    same_state(
        &classic,
        &next,
        &old_capture,
        &new_capture,
        "head automatically responds to all future",
    )
    .await;
    assert!(
        event_responses(&next, head, KEVIN)
            .await
            .iter()
            .all(|(_, response)| response.as_deref() == Some("maybe"))
    );
    respond_pair(&mut old_kevin, &mut new_kevin, ids[1], "declined", false).await;
    same_state(
        &classic,
        &next,
        &old_capture,
        &new_capture,
        "follower responds only to itself",
    )
    .await;
    assert_eq!(
        event_responses(&next, head, KEVIN)
            .await
            .into_iter()
            .map(|(_, response)| response.unwrap())
            .collect::<Vec<_>>(),
        ["maybe", "declined", "maybe", "maybe"]
    );
    respond_pair(&mut old_kevin, &mut new_kevin, ids[2], "going", true).await;
    same_state(
        &classic,
        &next,
        &old_capture,
        &new_capture,
        "follower applies to later occurrences",
    )
    .await;
    assert_eq!(
        event_responses(&next, head, KEVIN)
            .await
            .into_iter()
            .map(|(_, response)| response.unwrap())
            .collect::<Vec<_>>(),
        ["maybe", "declined", "going", "going"]
    );

    let mut changes = editable_body(&mut new_david, ids[1], "this_event").await;
    changes["title"] = json!("One exception");
    update_pair(&mut old_david, &mut new_david, ids[1], &changes).await;
    same_state(&classic, &next, &old_capture, &new_capture, "local edit").await;
    let mut changes = editable_body(&mut new_david, ids[2], "this_and_following").await;
    changes["title"] = json!("Later planning");
    changes["startsAt"] = json!("2026-03-17T11:00");
    changes["endsAt"] = json!("2026-03-17T12:00");
    update_pair(&mut old_david, &mut new_david, ids[2], &changes).await;
    same_state(
        &classic,
        &next,
        &old_capture,
        &new_capture,
        "following edit shifts later rows",
    )
    .await;
    let earlier = ok(&new_david
        .send(get(&format!("/api/v1/rooms/{DESIGNERS}/events/{head}")))
        .await);
    assert_eq!(earlier["event"]["title"], "Weekly parity");
    let later = ok(&new_david
        .send(get(&format!("/api/v1/rooms/{DESIGNERS}/events/{}", ids[3])))
        .await);
    assert_eq!(later["event"]["title"], "Later planning");

    // Rule rematerialization encounters a local exception and differing head/follower responses.
    let mut changes = editable_body(&mut new_david, head, "this_and_following").await;
    changes["recurrenceRule"] = json!("biweekly");
    changes["recurrenceUntil"] = json!("2026-03-31");
    update_pair(&mut old_david, &mut new_david, head, &changes).await;
    same_state(
        &classic,
        &next,
        &old_capture,
        &new_capture,
        "head changes recurrence and preserves response exceptions",
    )
    .await;
    let following = series_ids(&next, head).await;
    assert!(
        following.contains(&ids[1]),
        "differently-responded follower identity survives rematerialization"
    );
    cancel_pair(&mut old_david, &mut new_david, head, "this_and_following").await;
    same_state(
        &classic,
        &next,
        &old_capture,
        &new_capture,
        "cancel following",
    )
    .await;
    let cancelled = next
        .db()
        .read(move |conn| {
            Ok(CalendarEvent::find(conn, head)?
                .series_events(conn)?
                .into_iter()
                .all(|event| event.cancelled()))
        })
        .await
        .unwrap();
    assert!(cancelled, "all series occurrences are cancelled");
}

#[tokio::test]
async fn spa_api_events_create_validation_keys_messages_and_no_writes_match_classic() {
    let (Some(classic), Some(next)) = (app().await, app().await) else {
        return;
    };
    let mut old = classic.sign_in(DAVID).await;
    let mut new = next.sign_in(DAVID).await;
    old.authenticity_token().await;
    new.authenticity_token().await;
    let old_before = snapshot(&classic).await;
    let new_before = snapshot(&next).await;
    let base = event_body("Validation", "2026-03-03T09:00", Some("2026-03-03T10:00"));
    let mut cases = Vec::new();
    for (key, value, field, message) in [
        ("title", json!("  "), "title", "can't be blank"),
        ("startsAt", json!(null), "starts_at", "can't be blank"),
        (
            "endsAt",
            json!("2026-03-03T08:00"),
            "ends_at",
            "must be after the start time",
        ),
        (
            "venueRoomId",
            json!(DESIGNERS),
            "venue",
            "must be a voice or Stage channel you belong to",
        ),
        (
            "recurrenceRule",
            json!("yearly"),
            "recurrence_rule",
            "is not included in the list",
        ),
    ] {
        let mut body = base.clone();
        body[key] = value;
        cases.push((body, field, message));
    }
    for (until, message) in [
        (Value::Null, "can't be blank"),
        (json!("2026-03-03"), "must be after the start date"),
        (
            json!("2027-03-04"),
            "must be at most one year after the start date",
        ),
    ] {
        let mut body = base.clone();
        body["recurrenceRule"] = json!("monthly");
        body["recurrenceUntil"] = until;
        cases.push((body, "recurrence_until", message));
    }
    let mut cap = base.clone();
    cap["recurrenceRule"] = json!("daily");
    cap["recurrenceUntil"] = json!("2026-04-24");
    cases.push((
        cap,
        "recurrence_until",
        "would create 53 occurrences (maximum 52); pick an earlier end date",
    ));
    for (body, field, message) in cases {
        let expected = old
            .write(classic_form(
                Method::POST,
                &format!("/rooms/{DESIGNERS}/events"),
                &body,
            ))
            .await;
        assert_eq!(
            expected.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{}",
            expected.text()
        );
        assert!(
            expected
                .text()
                .contains(&campfire_views::helpers::escape(message)),
            "classic error message {message}: {}",
            expected.text()
        );
        let actual = send(
            &mut new,
            Method::POST,
            &format!("/api/v1/rooms/{DESIGNERS}/events"),
            &body,
        )
        .await;
        let errors = fields(&actual);
        assert!(
            errors[wire_field(field)]
                .iter()
                .any(|text| text.contains(message)),
            "field {field}: {errors:?}"
        );
        assert_eq!(
            snapshot(&classic).await,
            old_before,
            "classic rejected create has no side effects"
        );
        assert_eq!(
            snapshot(&next).await,
            new_before,
            "API rejected create has no side effects"
        );
    }

    // Unknown zones still parse in the viewer zone, then the classic error form cannot
    // format those attempted times and returns500. JSON has no invalid form to render: it
    // returns the same model error as structured422, with no committed writes on either path.
    let mut unknown_zone = base.clone();
    unknown_zone["timeZone"] = json!("Not/AZone");
    let expected = old
        .write(classic_form(
            Method::POST,
            &format!("/rooms/{DESIGNERS}/events"),
            &unknown_zone,
        ))
        .await;
    assert_eq!(
        expected.status,
        StatusCode::INTERNAL_SERVER_ERROR,
        "classic invalid-zone form render"
    );
    let actual = send(
        &mut new,
        Method::POST,
        &format!("/api/v1/rooms/{DESIGNERS}/events"),
        &unknown_zone,
    )
    .await;
    assert_eq!(fields(&actual)["timeZone"], vec!["is invalid".to_string()]);
    assert_eq!(snapshot(&classic).await, old_before);
    assert_eq!(snapshot(&next).await, new_before);
}

#[tokio::test]
async fn spa_api_events_series_invalid_edits_preserve_classic_guards() {
    let (Some(classic), Some(next)) = (app().await, app().await) else {
        return;
    };
    arrange_reads(&classic).await;
    arrange_reads(&next).await;
    let mut old = classic.sign_in(DAVID).await;
    let mut new = next.sign_in(DAVID).await;
    old.authenticity_token().await;
    new.authenticity_token().await;
    let mut cases = Vec::new();
    let mut body = editable_body(&mut new, HEAD, "this_event").await;
    body["startsAt"] = json!("2026-03-03T10:00");
    cases.push((
        HEAD,
        body,
        "starts_at",
        "moves the whole series: choose This and following or the entire series",
    ));
    let mut body = editable_body(&mut new, FOLLOWER, "this_and_following").await;
    body["recurrenceRule"] = json!("biweekly");
    cases.push((
        FOLLOWER,
        body,
        "recurrence_rule",
        "can only be changed from the first event in the series using This and following",
    ));
    let mut body = editable_body(&mut new, HEAD, "this_event").await;
    body["recurrenceUntil"] = json!("2026-03-24");
    cases.push((
        HEAD,
        body,
        "recurrence_rule",
        "can only be changed from the first event in the series using This and following",
    ));
    let mut body = editable_body(&mut new, FOLLOWER, "this_event").await;
    body["startsAt"] = json!("2026-03-17T09:00");
    body["endsAt"] = json!("2026-03-17T10:00");
    cases.push((
        FOLLOWER,
        body,
        "starts_at",
        "must stay between the neighbouring occurrences in its series",
    ));
    let mut body = editable_body(&mut new, HEAD, "this_and_following").await;
    body["recurrenceRule"] = Value::Null;
    cases.push((
        HEAD,
        body,
        "recurrence_rule",
        "can't be removed from a repeating event",
    ));
    let mut body = editable_body(&mut new, LAUNCH, "this_event").await;
    body["recurrenceRule"] = json!("weekly");
    body["recurrenceUntil"] = json!("2026-03-20");
    cases.push((
        LAUNCH,
        body,
        "recurrence_rule",
        "can only be set when scheduling a new event",
    ));
    let old_before = snapshot(&classic).await;
    let new_before = snapshot(&next).await;
    for (id, body, field, message) in cases {
        let path = format!("/rooms/{DESIGNERS}/events/{id}");
        let expected = old.write(classic_form(Method::PATCH, &path, &body)).await;
        assert_eq!(
            expected.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{}",
            expected.text()
        );
        assert!(
            expected
                .text()
                .contains(&campfire_views::helpers::escape(message)),
            "classic error: {}",
            expected.text()
        );
        let actual = send(&mut new, Method::PATCH, &format!("/api/v1{path}"), &body).await;
        assert!(
            fields(&actual)[wire_field(field)]
                .iter()
                .any(|text| text.contains(message)),
            "{}",
            actual.text()
        );
        assert_eq!(
            snapshot(&classic).await,
            old_before,
            "classic invalid series write"
        );
        assert_eq!(
            snapshot(&next).await,
            new_before,
            "API invalid series write"
        );
    }

    // The pinned classic model treats nil series starts as an internal error, not422.
    let mut nil_start = editable_body(&mut new, FOLLOWER, "this_event").await;
    nil_start["startsAt"] = Value::Null;
    let path = format!("/rooms/{DESIGNERS}/events/{FOLLOWER}");
    let expected = old
        .write(classic_form(Method::PATCH, &path, &nil_start))
        .await;
    let actual = send(
        &mut new,
        Method::PATCH,
        &format!("/api/v1{path}"),
        &nil_start,
    )
    .await;
    assert_eq!(
        (expected.status, actual.status),
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            StatusCode::INTERNAL_SERVER_ERROR
        )
    );
    assert_eq!(snapshot(&classic).await, old_before);
    assert_eq!(snapshot(&next).await, new_before);
}

#[tokio::test]
async fn spa_api_events_permissions_match_classic_without_broader_access() {
    let (Some(classic), Some(next)) = (app().await, app().await) else {
        return;
    };
    let mut old = classic.sign_in(KEVIN).await;
    let mut new = next.sign_in(KEVIN).await;
    old.authenticity_token().await;
    new.authenticity_token().await;
    let body = event_body(
        "Unauthorized edit",
        "2026-03-04T11:00",
        Some("2026-03-04T12:00"),
    );
    let original = snapshot(&next).await;
    // A plain member can read/respond/create, but cannot manage another organizer's event.
    for suffix in ["/edit", ""] {
        let path = format!("/rooms/{DESIGNERS}/events/{LAUNCH}{suffix}");
        let expected = if suffix.is_empty() {
            old.write(classic_form(Method::PATCH, &path, &body)).await
        } else {
            old.classic_page(&path).await
        };
        let actual = if suffix.is_empty() {
            send(&mut new, Method::PATCH, &format!("/api/v1{path}"), &body).await
        } else {
            new.send(get(&format!("/api/v1{path}"))).await
        };
        assert_eq!(
            (expected.status, actual.status),
            (StatusCode::FORBIDDEN, StatusCode::FORBIDDEN)
        );
        assert_eq!(tag(&actual), "Forbidden");
    }
    let path = format!("/rooms/{DESIGNERS}/events/{LAUNCH}/cancel");
    assert_eq!(
        old.write(Req::new(Method::PATCH, &path).form(&[]))
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        send(
            &mut new,
            Method::PATCH,
            &format!("/api/v1{path}"),
            &json!({})
        )
        .await
        .status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        snapshot(&next).await,
        original,
        "management refusal writes nothing"
    );

    // Open-room events still require membership; wrong room/event pairs never leak details.
    for path in [
        format!("/rooms/{ALL_PETS}/events"),
        format!("/rooms/{ALL_PETS}/events/new"),
        format!("/rooms/{ALL_PETS}/events/{LAUNCH}"),
        format!("/rooms/{ALL_PETS}/events/{LAUNCH}/edit"),
        format!("/rooms/{ALL_PETS}/events/{LAUNCH}/attendance"),
        format!("/rooms/{DESIGNERS}/events/411254270"),
    ] {
        let expected = old.classic_page(&path).await;
        let actual = new.send(get(&format!("/api/v1{path}"))).await;
        assert_eq!(
            (expected.status, actual.status),
            (StatusCode::NOT_FOUND, StatusCode::NOT_FOUND),
            "{path}"
        );
    }
    for method in [Method::POST, Method::PATCH, Method::PUT] {
        let path = if method == Method::POST {
            format!("/rooms/{ALL_PETS}/events")
        } else if method == Method::PUT {
            format!("/rooms/{ALL_PETS}/events/{LAUNCH}/attendance")
        } else {
            format!("/rooms/{ALL_PETS}/events/{LAUNCH}")
        };
        let expected = old
            .write(classic_form(
                if method == Method::PUT {
                    Method::PATCH
                } else {
                    method.clone()
                },
                &path,
                &body,
            ))
            .await;
        let actual = send(&mut new, method, &format!("/api/v1{path}"), &body).await;
        assert_eq!(
            (expected.status, actual.status),
            (StatusCode::NOT_FOUND, StatusCode::NOT_FOUND)
        );
    }
    // An administrator who is a member may edit another organizer's event.
    let mut old_admin = classic.sign_in(JASON).await;
    let mut new_admin = next.sign_in(JASON).await;
    assert_eq!(
        old_admin
            .classic_page(&format!("/rooms/{DESIGNERS}/events/{LAUNCH}/edit"))
            .await
            .status,
        StatusCode::OK
    );
    assert_eq!(
        new_admin
            .send(get(&format!(
                "/api/v1/rooms/{DESIGNERS}/events/{LAUNCH}/edit"
            )))
            .await
            .status,
        StatusCode::OK
    );
    assert_eq!(
        old_admin
            .classic_page(&format!("/rooms/{DESIGNERS}/events/{CANCELLED}/edit"))
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        new_admin
            .send(get(&format!(
                "/api/v1/rooms/{DESIGNERS}/events/{CANCELLED}/edit"
            )))
            .await
            .status,
        StatusCode::FORBIDDEN
    );

    // A signed-in account changed to a bot is refused even for counts-only attendance reads.
    for a in [&classic, &next] {
        sql(a, "UPDATE users SET role=2 WHERE id=712064548;").await;
    }
    for suffix in [
        "",
        "/new",
        &format!("/{LAUNCH}"),
        &format!("/{LAUNCH}/edit"),
        &format!("/{LAUNCH}/attendance"),
    ] {
        let path = format!("/rooms/{DESIGNERS}/events{suffix}");
        assert_eq!(
            old.classic_page(&path).await.status,
            StatusCode::FORBIDDEN,
            "classic {path}"
        );
        assert_eq!(
            new.send(get(&format!("/api/v1{path}"))).await.status,
            StatusCode::FORBIDDEN,
            "API {path}"
        );
    }
    for a in [&classic, &next] {
        sql(a,"UPDATE users SET role=0 WHERE id=712064548; UPDATE rooms SET deleted_at='2026-03-02 16:00:00' WHERE id=654632876;").await;
    }
    let path = format!("/rooms/{DESIGNERS}/events");
    assert_eq!(old.classic_page(&path).await.status, StatusCode::NOT_FOUND);
    assert_eq!(
        new.send(get(&format!("/api/v1{path}"))).await.status,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn spa_api_events_invalid_responses_and_cancelled_response_match_classic_alerts() {
    let (Some(classic), Some(next)) = (app().await, app().await) else {
        return;
    };
    let mut old = classic.sign_in(KEVIN).await;
    let mut new = next.sign_in(KEVIN).await;
    old.authenticity_token().await;
    new.authenticity_token().await;
    let old_before = snapshot(&classic).await;
    let new_before = snapshot(&next).await;
    for (id, response, message) in [
        (LAUNCH, "perhaps", "Choose going, maybe, or declined."),
        (
            CANCELLED,
            "going",
            "This event is no longer open for responses.",
        ),
    ] {
        let path = format!("/rooms/{DESIGNERS}/events/{id}/attendance");
        let expected = old
            .write(Req::new(Method::PATCH, &path).form(&[("response", response)]))
            .await;
        assert_eq!(expected.status, StatusCode::FOUND, "{}", expected.text());
        assert_eq!(old.flash()["alert"], message);
        let actual = send(
            &mut new,
            Method::PUT,
            &format!("/api/v1{path}"),
            &json!({"response":response,"applyToFuture":true}),
        )
        .await;
        assert!(
            fields(&actual)["response"]
                .iter()
                .any(|text| text.contains(message)),
            "{}",
            actual.text()
        );
        assert_eq!(snapshot(&classic).await, old_before);
        assert_eq!(snapshot(&next).await, new_before);
    }
}

#[tokio::test]
async fn spa_api_events_edit_and_cancel_refresh_existing_message_cards_on_sync() {
    let Some(a) = app().await else { return };
    sql(&a,"INSERT INTO event_references (event_id,message_id,created_at,updated_at) VALUES (390339825,935962054,'2026-03-02 16:00:00','2026-03-02 16:00:00');").await;
    let (addr, server) = serve(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let kevin = a.sign_in(KEVIN).await;
    let mut sync =
        Sync::connect(addr, &kevin.cookie_header(), &[format!("room:{DESIGNERS}")]).await;
    sync.welcome().await;
    let mut body = editable_body(&mut david, LAUNCH, "this_event").await;
    body["title"] = json!("Live card revision");
    let reply = send(
        &mut david,
        Method::PATCH,
        &format!("/api/v1/rooms/{DESIGNERS}/events/{LAUNCH}"),
        &body,
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let updated=sync.until(|event|matches!(&event.payload,api::SyncPayload::MessageCards(cards) if cards.message_id==CARD_MESSAGE),|_|false).await;
    let api::SyncPayload::MessageCards(cards) = updated.payload else {
        unreachable!()
    };
    let card = cards
        .cards
        .iter()
        .find_map(|card| {
            if let api::MessageCard::Event(card) = card {
                Some(card)
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(card.title, "Live card revision");
    let reply = send(
        &mut david,
        Method::PATCH,
        &format!("/api/v1/rooms/{DESIGNERS}/events/{LAUNCH}/cancel"),
        &json!({"cancelScope":"this_event"}),
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let cancelled=sync.until(|event|matches!(&event.payload,api::SyncPayload::MessageCards(cards) if cards.message_id==CARD_MESSAGE),|_|false).await;
    let api::SyncPayload::MessageCards(cards) = cancelled.payload else {
        unreachable!()
    };
    let card = cards
        .cards
        .iter()
        .find_map(|card| {
            if let api::MessageCard::Event(card) = card {
                Some(card)
            } else {
                None
            }
        })
        .unwrap();
    assert!(card.cancelled);
    server.abort();
}

/// `events.changed` on Designers' topic.
fn designers_events_changed(event: &api::SyncEvent) -> bool {
    matches!(&event.payload, api::SyncPayload::EventsChanged(change) if change.room_id == DESIGNERS)
}

/// The next `events.changed` on Designers, from its topic.
async fn told(sync: &mut Sync, step: &str) {
    let event = sync.until(designers_events_changed, |_| false).await;
    assert_eq!(event.topic, format!("room:{DESIGNERS}"), "{step}");
}

// A series' later occurrences have no message linking them, so no `message.cards` speaks for
// them: every event write, from the API and from the classic pages alike, tells the room's topic,
// including the shortened recurrence that destroys an occurrence.
#[tokio::test]
async fn spa_api_events_tell_the_room_topic_about_every_change_classic_or_spa() {
    let Some(a) = app().await else { return };
    let (addr, server) = serve(&a).await;
    let mut david = a.sign_in(DAVID).await;
    david.authenticity_token().await;
    let kevin = a.sign_in(KEVIN).await;
    let mut sync =
        Sync::connect(addr, &kevin.cookie_header(), &[format!("room:{DESIGNERS}")]).await;
    sync.welcome().await;

    let mut body = event_body("Weekly sync", "2026-03-03T09:00", Some("2026-03-03T10:00"));
    body["recurrenceRule"] = json!("weekly");
    body["recurrenceUntil"] = json!("2026-03-24");
    let created = send(
        &mut david,
        Method::POST,
        &format!("/api/v1/rooms/{DESIGNERS}/events"),
        &body,
    )
    .await;
    assert_eq!(created.status, StatusCode::CREATED, "{}", created.text());
    let head = parse::<Value>(&created)["event"]["id"].as_i64().unwrap();
    told(&mut sync, "scheduling a series").await;
    let ids = series_ids(&a, head).await;
    assert_eq!(ids.len(), 4);
    let third = ids[2];
    let unlinked = a
        .db()
        .read(move |conn| {
            Ok(conn.query_row(
                "SELECT COUNT(*) FROM event_references WHERE event_id = ?",
                [third],
                |row| row.get::<_, i64>(0),
            )?)
        })
        .await
        .unwrap();
    assert_eq!(unlinked, 0, "no message links a later occurrence");

    let mut changes = editable_body(&mut david, ids[2], "this_event").await;
    changes["title"] = json!("Weekly sync, moved");
    let path = format!("/rooms/{DESIGNERS}/events/{}", ids[2]);
    ok(&send(
        &mut david,
        Method::PATCH,
        &format!("/api/v1{path}"),
        &changes,
    )
    .await);
    told(&mut sync, "an API edit of an unlinked occurrence").await;

    let reply = david
        .write(classic_form(
            Method::PATCH,
            &format!("{path}/cancel"),
            &json!({"cancelScope": "this_event"}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::FOUND, "{}", reply.text());
    told(&mut sync, "a classic cancel of an unlinked occurrence").await;

    // Ending the series a week after its head leaves no slot for the last occurrence.
    let mut changes = editable_body(&mut david, head, "this_and_following").await;
    changes["recurrenceUntil"] = json!("2026-03-10");
    let reply = david
        .write(classic_form(
            Method::PATCH,
            &format!("/rooms/{DESIGNERS}/events/{head}"),
            &changes,
        ))
        .await;
    assert_eq!(reply.status, StatusCode::FOUND, "{}", reply.text());
    told(&mut sync, "a classic edit that shortens the series").await;
    let remaining = series_ids(&a, head).await;
    assert!(
        !remaining.contains(&ids[3]),
        "the last occurrence is destroyed: {remaining:?}"
    );
    server.abort();
}

const WATERCOOLER: i64 = 411254270;
/// An event arranged in the open "All Pets" room, which Kevin hasn't joined.
const OPEN_ROOM_EVENT: i64 = 800000101;

/// How a refused caller authenticates.
#[derive(Clone, Copy)]
enum Caller<'a> {
    /// A signed-in browser; writes carry its authenticity token.
    Session,
    /// `?bot_key=` in the URL (already in each call's path), with no session.
    BotKey,
    /// An agent's `Authorization: Bearer` credential, with no session.
    Bearer(&'a str),
}

/// One event endpoint, as the classic pages and the SPA each send it.
struct Call {
    label: String,
    write: bool,
    classic: Req,
    api: Req,
}

/// Every event endpoint on `room`/`event`: the calendar, its form, scheduling, the event page,
/// its edit form, editing, cancelling and both attendance routes. Each write carries a valid
/// body so only the guard stands between the caller and a row.
fn event_calls(room: i64, event: i64, query: &str) -> Vec<Call> {
    let base = format!("/rooms/{room}/events");
    let mut calls: Vec<Call> = [
        String::new(),
        "/new".into(),
        format!("/{event}"),
        format!("/{event}/edit"),
        format!("/{event}/attendance"),
    ]
    .into_iter()
    .map(|suffix| {
        let path = format!("{base}{suffix}{query}");
        Call {
            label: format!("GET {base}{suffix}"),
            write: false,
            classic: Req::new(Method::GET, &path).header("accept", "*/*"),
            api: get(&format!("/api/v1{path}")),
        }
    })
    .collect();
    let body = event_body(
        "Guarded event",
        "2026-03-10T09:00",
        Some("2026-03-10T10:00"),
    );
    let create = format!("{base}{query}");
    calls.push(Call {
        label: format!("POST {base}"),
        write: true,
        classic: classic_form(Method::POST, &create, &body),
        api: json_body(Method::POST, &format!("/api/v1{create}"), &body),
    });
    let update = format!("{base}/{event}{query}");
    calls.push(Call {
        label: format!("PATCH {base}/{event}"),
        write: true,
        classic: classic_form(Method::PATCH, &update, &body),
        api: json_body(Method::PATCH, &format!("/api/v1{update}"), &body),
    });
    let cancel = format!("{base}/{event}/cancel{query}");
    calls.push(Call {
        label: format!("PATCH {base}/{event}/cancel"),
        write: true,
        classic: classic_form(
            Method::PATCH,
            &cancel,
            &json!({"cancelScope": "this_event"}),
        ),
        api: json_body(
            Method::PATCH,
            &format!("/api/v1{cancel}"),
            &json!({"cancelScope": "this_event"}),
        ),
    });
    let respond = format!("{base}/{event}/attendance{query}");
    calls.push(Call {
        label: format!("respond {base}/{event}/attendance"),
        write: true,
        classic: Req::new(Method::PATCH, &respond).form(&[("response", "going")]),
        api: json_body(
            Method::PUT,
            &format!("/api/v1{respond}"),
            &json!({"response": "going"}),
        ),
    });
    calls
}

/// Everything a guarded endpoint could write: the event snapshot plus every membership, so an
/// event URL that joined a room would show up here.
async fn guarded_state(a: &TestApp) -> Value {
    let memberships = a
        .db()
        .read(|conn| {
            rows(
                conn,
                "SELECT id,room_id,user_id,involvement FROM memberships ORDER BY id",
            )
        })
        .await
        .unwrap();
    json!({"snapshot": snapshot(a).await, "memberships": memberships})
}

/// Sends every call to the classic pages and to `/api/v1`, asserting both answer `status` and
/// neither app writes anything. Session browsers send writes with their authenticity token;
/// bot keys and agent tokens send none, as their clients do.
#[allow(clippy::too_many_arguments)]
async fn assert_refused(
    classic: &TestApp,
    next: &TestApp,
    old: &mut Browser<'_>,
    new: &mut Browser<'_>,
    calls: Vec<Call>,
    caller: Caller<'_>,
    status: StatusCode,
    who: &str,
) {
    let (old_before, new_before) = (guarded_state(classic).await, guarded_state(next).await);
    for Call {
        label,
        write,
        mut classic,
        mut api,
    } in calls
    {
        if let Caller::Bearer(authorization) = caller {
            classic = classic.header("authorization", authorization);
            api = api.header("authorization", authorization);
        }
        let (expected, actual) = if write && matches!(caller, Caller::Session) {
            (old.write(classic).await, new.write(api).await)
        } else {
            (old.send(classic).await, new.send(api).await)
        };
        assert_eq!(
            (expected.status, actual.status),
            (status, status),
            "{who}: {label}: classic {}, API {}",
            expected.text(),
            actual.text()
        );
    }
    assert_eq!(
        guarded_state(classic).await,
        old_before,
        "{who}: classic refusals write nothing"
    );
    assert_eq!(
        guarded_state(next).await,
        new_before,
        "{who}: API refusals write nothing"
    );
}

#[tokio::test]
async fn spa_api_events_refuse_non_members_of_private_rooms_like_classic() {
    let (Some(classic), Some(next)) = (app().await, app().await) else {
        return;
    };
    // Kevin isn't in the closed "All Talk" room, which holds the Watercooler event.
    let mut old = classic.sign_in(KEVIN).await;
    let mut new = next.sign_in(KEVIN).await;
    old.authenticity_token().await;
    new.authenticity_token().await;
    assert_refused(
        &classic,
        &next,
        &mut old,
        &mut new,
        event_calls(ALL_TALK, WATERCOOLER, ""),
        Caller::Session,
        StatusCode::NOT_FOUND,
        "non-member of a private room",
    )
    .await;
    // Not even an administrator outside the room sees it: the guard is membership, not role.
    for a in [&classic, &next] {
        sql(
            a,
            "DELETE FROM memberships WHERE room_id=486777696 AND user_id=149087659;",
        )
        .await;
    }
    let mut old_admin = classic.sign_in(JASON).await;
    let mut new_admin = next.sign_in(JASON).await;
    old_admin.authenticity_token().await;
    new_admin.authenticity_token().await;
    assert_refused(
        &classic,
        &next,
        &mut old_admin,
        &mut new_admin,
        event_calls(ALL_TALK, WATERCOOLER, ""),
        Caller::Session,
        StatusCode::NOT_FOUND,
        "administrator outside a private room",
    )
    .await;
}

#[tokio::test]
async fn spa_api_events_direct_rooms_serve_participants_and_refuse_others_like_classic() {
    let (Some(classic), Some(next)) = (app().await, app().await) else {
        return;
    };
    let mut old_david = classic.sign_in(DAVID).await;
    let mut new_david = next.sign_in(DAVID).await;
    old_david.authenticity_token().await;
    new_david.authenticity_token().await;
    // Classic has no room-type check: a participant lists, opens the form and schedules.
    let path = format!("/rooms/{DIRECT_DAVID_JASON}/events");
    assert_eq!(old_david.classic_page(&path).await.status, StatusCode::OK);
    let list = ok(&new_david.send(get(&format!("/api/v1{path}"))).await);
    assert_eq!(list["roomId"], DIRECT_DAVID_JASON);
    assert_eq!(list["roomKind"], "direct");
    assert_eq!(list["mayCreate"], true);
    assert_eq!(
        old_david.classic_page(&format!("{path}/new")).await.status,
        StatusCode::OK
    );
    let form = ok(&new_david.send(get(&format!("/api/v1{path}/new"))).await);
    assert_eq!(form["roomId"], DIRECT_DAVID_JASON);
    let body = event_body("Direct sync", "2026-03-10T09:00", Some("2026-03-10T10:00"));
    let expected = old_david
        .write(classic_form(Method::POST, &path, &body))
        .await;
    assert_eq!(expected.status, StatusCode::FOUND, "{}", expected.text());
    let created = send(
        &mut new_david,
        Method::POST,
        &format!("/api/v1{path}"),
        &body,
    )
    .await;
    assert_eq!(created.status, StatusCode::CREATED, "{}", created.text());
    let id = parse::<Value>(&created)["event"]["id"]
        .as_i64()
        .expect("created event id");
    assert!(
        expected
            .location()
            .unwrap()
            .ends_with(&format!("{path}/{id}"))
    );
    assert_eq!(
        snapshot(&classic).await,
        snapshot(&next).await,
        "a direct-room event writes what classic writes"
    );
    let detail = ok(&new_david.send(get(&format!("/api/v1{path}/{id}"))).await);
    assert_eq!(detail["roomId"], DIRECT_DAVID_JASON);
    assert_eq!(detail["event"]["title"], "Direct sync");
    // The other participant reads it too.
    let mut old_jason = classic.sign_in(JASON).await;
    let mut new_jason = next.sign_in(JASON).await;
    assert_eq!(
        old_jason.classic_page(&format!("{path}/{id}")).await.status,
        StatusCode::OK
    );
    ok(&new_jason.send(get(&format!("/api/v1{path}/{id}"))).await);

    // Someone outside the conversation gets classic's 404 everywhere and writes nothing.
    let mut old_kevin = classic.sign_in(KEVIN).await;
    let mut new_kevin = next.sign_in(KEVIN).await;
    old_kevin.authenticity_token().await;
    new_kevin.authenticity_token().await;
    assert_refused(
        &classic,
        &next,
        &mut old_kevin,
        &mut new_kevin,
        event_calls(DIRECT_DAVID_JASON, id, ""),
        Caller::Session,
        StatusCode::NOT_FOUND,
        "non-participant of a direct room",
    )
    .await;
}

#[tokio::test]
async fn spa_api_events_refuse_bots_on_every_endpoint_like_classic() {
    let (Some(classic), Some(next)) = (app().await, app().await) else {
        return;
    };
    // Bender is a member of "All Talk", so only the bot rule can refuse him.
    for a in [&classic, &next] {
        crate::controllers::agent_http_tests::initialize(a).await;
    }
    let bot_key = format!("?bot_key={BENDER_KEY}");
    assert_refused(
        &classic,
        &next,
        &mut classic.anonymous(),
        &mut next.anonymous(),
        event_calls(ALL_TALK, WATERCOOLER, &bot_key),
        Caller::BotKey,
        StatusCode::FORBIDDEN,
        "bot key",
    )
    .await;
    let bearer = format!("Bearer {}", crate::controllers::agent_http_tests::SECRET);
    assert_refused(
        &classic,
        &next,
        &mut classic.anonymous(),
        &mut next.anonymous(),
        event_calls(ALL_TALK, WATERCOOLER, ""),
        Caller::Bearer(&bearer),
        StatusCode::FORBIDDEN,
        "agent token",
    )
    .await;
    // A bot's own browser session: take the authenticity token as a person, then restore the
    // bot role so writes reach the bot rule rather than forgery protection.
    for a in [&classic, &next] {
        sql(a, "UPDATE users SET role=0 WHERE id=394959859;").await;
    }
    let mut old = classic.sign_in(BENDER).await;
    let mut new = next.sign_in(BENDER).await;
    old.authenticity_token().await;
    new.authenticity_token().await;
    for a in [&classic, &next] {
        sql(a, "UPDATE users SET role=2 WHERE id=394959859;").await;
    }
    assert_refused(
        &classic,
        &next,
        &mut old,
        &mut new,
        event_calls(ALL_TALK, WATERCOOLER, ""),
        Caller::Session,
        StatusCode::FORBIDDEN,
        "bot session",
    )
    .await;
}

async fn kevin_in_all_pets(a: &TestApp) -> i64 {
    a.db()
        .read(|conn| {
            Ok(conn.query_row(
                "SELECT COUNT(*) FROM memberships WHERE room_id=? AND user_id=?",
                [ALL_PETS, KEVIN],
                |row| row.get::<_, i64>(0),
            )?)
        })
        .await
        .unwrap()
}

#[tokio::test]
async fn spa_api_events_never_join_an_open_room_like_classic() {
    let (Some(classic), Some(next)) = (app().await, app().await) else {
        return;
    };
    for a in [&classic, &next] {
        sql(a, "INSERT INTO events (id,room_id,organizer_id,title,starts_at,time_zone,created_at,updated_at) VALUES (800000101,104393281,127326141,'Pets meetup','2026-03-05 16:00:00','UTC','2026-03-02 16:00:00','2026-03-02 16:00:00');").await;
    }
    assert_eq!(
        kevin_in_all_pets(&next).await,
        0,
        "Kevin hasn't joined All Pets"
    );
    let mut old = classic.sign_in(KEVIN).await;
    let mut new = next.sign_in(KEVIN).await;
    old.authenticity_token().await;
    new.authenticity_token().await;
    assert_refused(
        &classic,
        &next,
        &mut old,
        &mut new,
        event_calls(ALL_PETS, OPEN_ROOM_EVENT, ""),
        Caller::Session,
        StatusCode::NOT_FOUND,
        "open room not joined",
    )
    .await;
    assert_eq!(
        kevin_in_all_pets(&classic).await,
        0,
        "classic joined nothing"
    );
    assert_eq!(
        kevin_in_all_pets(&next).await,
        0,
        "event URLs joined nothing"
    );
}
