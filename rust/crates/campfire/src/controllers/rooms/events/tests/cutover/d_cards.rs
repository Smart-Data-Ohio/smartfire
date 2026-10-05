//! Remaining original card/timeline declarations through the real HTTP and Cable producers.
use super::super::*;
use super::support::*;
use campfire_db::{Message, NewMessage, RoomType, models::calendar_event::changes::EventChanges};
use campfire_richtext::dom::{Dom, NodeId};
use serde_json::{Value, json};

async fn create_event(
    app: &TestApp,
    title: &str,
    venue: Option<i64>,
    repeating: bool,
    ends: bool,
) -> CalendarEvent {
    let title = title.to_owned();
    app.db()
        .write(move |tx| {
            CalendarEvent::create(
                tx,
                NewCalendarEvent {
                    room_id: id("designers"),
                    organizer_id: DAVID,
                    title,
                    starts_at: Some(tx.now().since(jiff::SignedDuration::from_hours(48))),
                    ends_at: ends.then(|| tx.now().since(jiff::SignedDuration::from_hours(49))),
                    time_zone: "UTC".into(),
                    venue_room_id: venue,
                    recurrence_rule: repeating.then(|| "weekly".into()),
                    recurrence_until: repeating.then(|| "2026-10-08".parse().unwrap()),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap()
}
async fn link(app: &TestApp, eid: i64, room: i64, creator: i64, text: &str, key: &str) -> Message {
    let text = format!("{text} /rooms/{}/events/{eid}", id("designers"));
    let key = key.to_owned();
    app.db()
        .write(move |tx| {
            Message::create(
                tx,
                NewMessage {
                    room_id: room,
                    creator_id: creator,
                    markdown_source: Some(text),
                    client_message_id: Some(key),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap()
}
fn class(dom: &Dom, node: NodeId, name: &str) -> bool {
    dom.attr(node, "class")
        .is_some_and(|c| c.split_whitespace().any(|word| word == name))
}
fn by_class(dom: &Dom, root: NodeId, name: &str) -> Vec<NodeId> {
    dom.descendants(root)
        .into_iter()
        .filter(|&n| class(dom, n, name))
        .collect()
}
fn text(dom: &Dom, n: NodeId) -> String {
    dom.text_content(n)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
async fn event_ids(app: &TestApp, mid: i64) -> Vec<i64> {
    app.db()
        .read(move |c| {
            Ok(CalendarEvent::for_message_ids(c, &[mid])?
                .remove(&mid)
                .unwrap_or_default()
                .iter()
                .map(|e| e.id)
                .collect())
        })
        .await
        .unwrap()
}
async fn references(app: &TestApp, eid: i64) -> Vec<Message> {
    app.db().read(move|c|{
        let ids=c.prepare("SELECT m.id FROM messages m JOIN event_references r ON r.message_id=m.id WHERE r.event_id=? AND m.room_id=? ORDER BY m.id")?.query_map(rusqlite::params![eid,id("designers")],|r|r.get(0))?.collect::<rusqlite::Result<Vec<i64>>>()?;
        ids.into_iter().map(|mid|Message::find(c,mid)).collect()
    }).await.unwrap()
}
async fn socket(
    app: &TestApp,
    rooms: &[i64],
) -> (
    crate::channels::tests::support::Client,
    tokio::task::JoinHandle<()>,
) {
    use crate::channels::tests::support::{Client, bind_listener, identifier};
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    let cookie = app.sign_in(JASON).await.cookie_header();
    let listener = bind_listener().await;
    let address = listener.local_addr().unwrap();
    let router = app.booted.router.clone();
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let mut ws = format!("ws://{address}/cable")
        .into_client_request()
        .unwrap();
    for (key, value) in [
        ("host", "campfire.test"),
        ("origin", "http://campfire.test"),
        ("cookie", cookie.as_str()),
    ] {
        ws.headers_mut().insert(key, value.parse().unwrap());
    }
    let (socket, _) = tokio_tungstenite::connect_async(ws).await.unwrap();
    let mut client = Client { socket };
    assert_eq!(client.next_text().await, r#"{"type":"welcome"}"#);
    for &rid in rooms {
        let room = app
            .db()
            .read(move |c| campfire_db::Room::find(c, rid))
            .await
            .unwrap();
        let signed = rails_compat::turbo::signed_stream_name(
            &app.booted.app.secrets,
            &[&crate::channels::room_gid(&room).to_param(), "messages"],
        );
        client
            .confirm(&identifier(
                json!({"channel":"RoomMessagesChannel","signed_stream_name":signed}),
            ))
            .await;
    }
    app.publications();
    (client, server)
}
fn stream_name(rid: i64) -> String {
    format!(
        "{}:messages",
        campfire_views::helpers::gid_param("Rooms::Closed", rid)
    )
}
async fn delivered(
    app: &TestApp,
    client: &mut crate::channels::tests::support::Client,
    count: usize,
) -> Vec<(String, String)> {
    for _ in 0..count {
        client.next_text().await;
    }
    client.assert_silent().await;
    app.publications().take()
}

#[tokio::test]
async fn cutover_d_timeline_singleton_update_replaces_exact_two_same_room_references_and_no_cross_room()
 {
    let app = app().await;
    let e = create_event(&app, "Planning session", None, false, false).await;
    let same = link(&app, e.id, id("designers"), JASON, "see", "evt-broadcast-1").await;
    let other = link(
        &app,
        e.id,
        id("watercooler"),
        JASON,
        "also see",
        "evt-broadcast-2",
    )
    .await;
    // Rails 76
    assert_eq!(event_ids(&app, same.id).await, vec![e.id]);
    // Rails 79
    assert!(event_ids(&app, other.id).await.is_empty());
    let refs = references(&app, e.id).await;
    // Rails 86
    assert_eq!(refs.len(), 2);
    let (mut client, server) = socket(&app, &[id("designers"), id("watercooler")]).await;
    app.db()
        .write(move |tx| {
            CalendarEvent::update(
                tx,
                e.id,
                EventChanges {
                    title: Some("A new title".into()),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
    let published = delivered(&app, &mut client, refs.len()).await;
    // Rails 88
    assert_eq!(
        published
            .iter()
            .filter(|(stream, _)| stream == &stream_name(id("designers")))
            .count(),
        refs.len()
    );
    // Rails 89
    assert_eq!(
        published
            .iter()
            .filter(|(stream, _)| stream == &stream_name(id("watercooler")))
            .count(),
        0
    );
    let mut targets = vec![];
    for (_, payload) in &published {
        let html = serde_json::from_str::<Value>(payload).unwrap();
        let html = html.as_str().unwrap();
        let mut dom = Dom::new();
        let root = dom.parse_fragment(html).unwrap();
        let turbo = dom
            .descendants(root)
            .into_iter()
            .find(|&n| dom.local_name(n) == Some("turbo-stream"))
            .unwrap();
        assert_eq!(dom.attr(turbo, "action"), Some("replace"));
        assert!(html.contains("A new title"));
        targets.push(dom.attr(turbo, "target").unwrap().to_owned());
    }
    targets.sort();
    let mut expected = refs
        .iter()
        .map(|m| crate::channels::broadcasts::message_dom_id(m, Some("event_cards")))
        .collect::<Vec<_>>();
    expected.sort();
    assert_eq!(targets, expected);
    server.abort();
}

#[tokio::test]
async fn cutover_d_timeline_singleton_cancel_replaces_both_announcement_and_separate_link() {
    let app = app().await;
    let e = create_event(&app, "Planning session", None, false, false).await;
    let same = link(
        &app,
        e.id,
        id("designers"),
        JASON,
        "see",
        "evt-broadcast-cancel",
    )
    .await;
    // Rails 102
    assert_eq!(event_ids(&app, same.id).await, vec![e.id]);
    let refs = references(&app, e.id).await;
    let (mut client, server) = socket(&app, &[id("designers")]).await;
    let cancelled = app
        .db()
        .write(move |tx| CalendarEvent::cancel_with_scope(tx, e.id, "this_event", Some(DAVID)))
        .await
        .unwrap();
    // Rails 106
    assert!(cancelled);
    let published = delivered(&app, &mut client, refs.len()).await;
    // Rails 104
    assert_eq!(published.len(), refs.len());
    let mut targets = vec![];
    for (stream, payload) in &published {
        assert_eq!(stream, &stream_name(id("designers")));
        let html = serde_json::from_str::<Value>(payload).unwrap();
        let html = html.as_str().unwrap();
        let mut dom = Dom::new();
        let root = dom.parse_fragment(html).unwrap();
        let turbo = dom
            .descendants(root)
            .into_iter()
            .find(|&n| dom.local_name(n) == Some("turbo-stream"))
            .unwrap();
        assert_eq!(dom.attr(turbo, "action"), Some("replace"));
        assert!(html.contains("Cancelled"));
        targets.push(dom.attr(turbo, "target").unwrap().to_owned());
    }
    targets.sort();
    let mut expected = refs
        .iter()
        .map(|m| crate::channels::broadcasts::message_dom_id(m, Some("event_cards")))
        .collect::<Vec<_>>();
    expected.sort();
    assert_eq!(targets, expected);
    server.abort();
}

#[tokio::test]
async fn cutover_d_cards_room_http_scopes_single_card_title_time_venue_organizer_and_attendance_frame()
 {
    let app = app().await;
    let voice = venue(&app, RoomType::Voice, "Lounge", DAVID, &[DAVID, JASON]).await;
    let e = create_event(&app, "Planning session", Some(voice), false, true).await;
    let message = link(&app, e.id, id("designers"), JASON, "see", "evt-card-1").await;
    let mut david = app.sign_in(DAVID).await;
    let reply = david.get(&format!("/rooms/{}", id("designers"))).await;
    // Rails 23
    assert_eq!(reply.status, StatusCode::OK);
    let mut dom = Dom::new();
    let root = dom.parse_fragment(&reply.text()).unwrap();
    let container_id = crate::channels::broadcasts::message_dom_id(&message, Some("event_cards"));
    let containers = dom
        .descendants(root)
        .into_iter()
        .filter(|&n| dom.attr(n, "id") == Some(container_id.as_str()))
        .collect::<Vec<_>>();
    // Rails 26
    assert_eq!(containers.len(), 1);
    let container = containers[0];
    // Rails 27
    assert_eq!(by_class(&dom, container, "event-card").len(), 1);
    // Rails 28
    assert_eq!(
        by_class(&dom, container, "event-card__eyebrow")
            .iter()
            .map(|&n| text(&dom, n))
            .collect::<Vec<_>>(),
        vec!["Event"]
    );
    // Rails 29
    assert_eq!(
        by_class(&dom, container, "event-card__title")
            .iter()
            .map(|&n| text(&dom, n))
            .collect::<Vec<_>>(),
        vec!["Planning session"]
    );
    let title = by_class(&dom, container, "event-card__title")[0];
    // Rails 30
    assert_eq!(
        dom.descendants(title)
            .into_iter()
            .filter(|&n| dom.local_name(n) == Some("a")
                && dom.attr(n, "href") == Some(path(e.id).as_str()))
            .count(),
        1
    );
    let times = by_class(&dom, container, "event-card__meta")
        .into_iter()
        .flat_map(|meta| dom.descendants(meta))
        .filter(|&n| dom.local_name(n) == Some("time"))
        .collect::<std::collections::HashSet<_>>();
    // Rails 31
    assert_eq!(times.len(), 2);
    // Rails 32
    assert!(
        by_class(&dom, container, "event-card__venue")
            .iter()
            .any(|&n| text(&dom, n).contains("Lounge"))
    );
    // Rails 33
    assert!(
        by_class(&dom, container, "event-card__organizer")
            .iter()
            .any(|&n| text(&dom, n).contains("David"))
    );
    let src = format!("{}/attendance?message_id={}", path(e.id), message.id);
    // Rails 34
    assert_eq!(
        dom.descendants(container)
            .into_iter()
            .filter(|&n| dom.local_name(n) == Some("turbo-frame")
                && dom.attr(n, "src") == Some(src.as_str()))
            .count(),
        1
    );
}

#[tokio::test]
async fn cutover_d_cards_room_http_voice_venue_has_no_join_or_live_dot() {
    let app = app().await;
    let voice = venue(&app, RoomType::Voice, "Lounge", DAVID, &[DAVID]).await;
    let e = create_event(&app, "Venue meetup", Some(voice), false, false).await;
    link(&app, e.id, id("designers"), DAVID, "see", "evt-card-venue").await;
    let mut david = app.sign_in(DAVID).await;
    let reply = david.get(&format!("/rooms/{}", id("designers"))).await;
    // Rails 53
    assert_eq!(reply.status, StatusCode::OK);
    let mut dom = Dom::new();
    let root = dom.parse_fragment(&reply.text()).unwrap();
    let cards = by_class(&dom, root, "event-card");
    // Rails 54
    assert!(!cards.is_empty());
    // Rails 55
    assert!(cards.iter().any(|&n| text(&dom, n).contains("Lounge")));
    // Rails 56
    assert_eq!(
        cards
            .iter()
            .flat_map(|&n| dom.descendants(n))
            .filter(|&n| dom.local_name(n) == Some("a") && text(&dom, n) == "Join")
            .count(),
        0
    );
    // Rails 57
    assert_eq!(
        cards
            .iter()
            .map(|&n| by_class(&dom, n, "sidebar-item__icon").len())
            .sum::<usize>(),
        0
    );
}

#[tokio::test]
async fn cutover_d_cards_room_http_real_recurring_event_has_repeating_eyebrow() {
    let app = app().await;
    let e = create_event(&app, "Weekly planning", None, true, false).await;
    link(&app, e.id, id("designers"), DAVID, "see", "evt-card-series").await;
    let mut david = app.sign_in(DAVID).await;
    let reply = david.get(&format!("/rooms/{}", id("designers"))).await;
    // Rails 73
    assert_eq!(reply.status, StatusCode::OK);
    let mut dom = Dom::new();
    let root = dom.parse_fragment(&reply.text()).unwrap();
    // Rails 74
    assert!(
        by_class(&dom, root, "event-card__eyebrow")
            .iter()
            .any(|&n| text(&dom, n) == "Repeating event")
    );
}

#[tokio::test]
async fn cutover_d_cards_room_http_model_cancelled_event_renders_cancelled_card_and_state() {
    let app = app().await;
    let e = create_event(&app, "Called off", None, false, false).await;
    link(
        &app,
        e.id,
        id("designers"),
        DAVID,
        "see",
        "evt-card-cancelled",
    )
    .await;
    app.db()
        .write(move |tx| CalendarEvent::cancel_with_scope(tx, e.id, "this_event", Some(DAVID)))
        .await
        .unwrap();
    let mut david = app.sign_in(DAVID).await;
    let reply = david.get(&format!("/rooms/{}", id("designers"))).await;
    // Rails 91
    assert_eq!(reply.status, StatusCode::OK);
    let mut dom = Dom::new();
    let root = dom.parse_fragment(&reply.text()).unwrap();
    // Rails 92
    assert!(!by_class(&dom, root, "event-card--cancelled").is_empty());
    // Rails 93
    assert!(
        by_class(&dom, root, "event-card__state")
            .iter()
            .any(|&n| text(&dom, n) == "Cancelled")
    );
}

#[tokio::test]
async fn cutover_d_wide_calendar_form_http_preserves_year_and_exact_event_and_viewer_zone_periods()
{
    let app = app().await;
    app.db()
        .write(|tx| {
            tx.conn()
                .execute("UPDATE users SET time_zone='Hawaii' WHERE id=?", [DAVID])?;
            Ok(())
        })
        .await
        .unwrap();
    let mut david = app.sign_in(DAVID).await;
    // The first input is the exact datetime-local payload Chrome submitted in
    // original EventsTest:102. The summer case distinguishes the real final
    // Rails TZInfo period (EST) from the calendar proxy's extrapolated EDT.
    for (title, start, end, utc, viewer, label) in [
        (
            "Wide browser payload",
            "60310-02-02T15:30",
            "60310-02-02T16:30",
            "60310-02-02 20:30:00",
            "60310-02-02T10:30:00-10:00",
            "February 2, 60310 at 3:30 PM",
        ),
        (
            "Wide summer",
            "12026-07-04T15:30",
            "12026-07-04T16:30",
            "12026-07-04 20:30:00",
            "12026-07-04T10:30:00-10:00",
            "July 4, 12026 at 3:30 PM",
        ),
        (
            "After final Rails timezone transition",
            "2127-07-04T15:30",
            "2127-07-04T16:30",
            "2127-07-04 20:30:00",
            "2127-07-04T10:30:00-10:00",
            "July 4, 2127 at 3:30 PM",
        ),
    ] {
        let reply = david.write(json(Method::POST, &index_path(), json!({"event": {
            "title":title, "starts_at":start, "ends_at":end, "time_zone":"America/New_York",
            "description":"", "recurrence_rule":"", "recurrence_until":"", "venue_room_id":"", "meet_link_requested":"0",
        }}))).await;
        assert_eq!(reply.status, StatusCode::FOUND);
        let event = app
            .db()
            .read(move |c| {
                let eid =
                    c.query_row("SELECT id FROM events WHERE title=?", [title], |r| r.get(0))?;
                CalendarEvent::find(c, eid)
            })
            .await
            .unwrap();
        assert_eq!(event.starts_at.to_db(), utc);
        let show = david.get(&path(event.id)).await;
        assert_eq!(show.status, StatusCode::OK);
        let mut dom = Dom::new();
        let root = dom.parse_fragment(&show.text()).unwrap();
        assert!(
            dom.descendants(root)
                .into_iter()
                .any(|n| dom.local_name(n) == Some("h1") && text(&dom, n) == title)
        );
        assert!(
            dom.descendants(root)
                .into_iter()
                .any(|n| dom.local_name(n) == Some("time")
                    && dom.attr(n, "datetime") == Some(viewer)
                    && text(&dom, n) == label)
        );
        let edit = david.get(&format!("{}/edit", path(event.id))).await;
        assert_eq!(edit.status, StatusCode::OK);
        let mut dom = Dom::new();
        let root = dom.parse_fragment(&edit.text()).unwrap();
        assert!(
            dom.descendants(root)
                .into_iter()
                .any(|n| dom.attr(n, "name") == Some("event[starts_at]")
                    && dom.attr(n, "value") == Some(start))
        );
        assert!(
            dom.descendants(root)
                .into_iter()
                .any(|n| dom.attr(n, "name") == Some("event[ends_at]")
                    && dom.attr(n, "value") == Some(end))
        );
        let room = david.get(&format!("/rooms/{}", id("designers"))).await;
        assert_eq!(room.status, StatusCode::OK);
        assert!(
            cards(&room.text())
                .iter()
                .any(|card| card.contains(title) && card.contains(label))
        );
    }
}
