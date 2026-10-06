//! test/system/events_test.rb: server interaction assertions via real HTTP and Turbo frames.
//! The shared Rails assets are unchanged; no browser pixels are used as a receipt.
use super::super::*;
use super::rendered;
use super::support::*;
async fn schedule(app: &TestApp, title: &str, description: Option<&str>, repeating: bool) -> i64 {
    let mut david = app.sign_in(DAVID).await;
    let room = david.get(&format!("/rooms/{}", id("designers"))).await;
    assert_eq!(room.status, StatusCode::OK);
    let index = david.send(rendered::link(&room.text(), "Show events")).await;
    assert_eq!(index.status, StatusCode::OK);
    assert!(headings(&index.text()).iter().any(|h| h == "Events"));
    assert!(index.text().contains("New event"));
    let form = david.send(rendered::link(&index.text(), "New event")).await;
    assert_eq!(form.status, StatusCode::OK);
    for field in [
        "event[title]",
        "event[description]",
        "event[starts_at]",
        "event[recurrence_rule]",
        "event[recurrence_until]",
    ] {
        assert!(form.text().contains(&format!("name=\"{field}\"")));
    }
    assert!(form.text().contains("Schedule event"));
    let mut input = vec![
        ("event[title]", title),
        ("event[starts_at]", "2026-09-30T15:30"),
    ];
    if let Some(description) = description {
        input.push(("event[description]", description));
    }
    if repeating {
        input.push(("event[recurrence_rule]", "Weekly"));
        input.push(("event[recurrence_until]", "2026-10-14"));
    }
    let reply = david
        .send(rendered::submit(&form.text(), "Schedule event", &input))
        .await;
    let shown = rendered::follow(&mut david, &reply).await;
    let title = title.to_owned();
    let eid = app
        .db()
        .read(move |c| {
            Ok(
                c.query_row("SELECT MIN(id) FROM events WHERE title=?", [title], |r| {
                    r.get::<_, i64>(0)
                })?,
            )
        })
        .await
        .unwrap();
    redirected(&reply, eid);
    assert_eq!(shown.status, StatusCode::OK);
    assert!(headings(&shown.text()).contains(&find(app, eid).await.title));
    if let Some(description) = description {
        assert!(shown.text().contains(description));
    }
    assert!(visible_text(&shown.text()).contains("Currently: Going"));
    assert!(shown.text().contains("All events"));
    if repeating {
        assert!(shown.text().contains("Part of a series"));
        assert!(shown.text().contains("Next occurrence"));
    }
    let index = david
        .send(rendered::link(&shown.text(), "All events"))
        .await;
    assert_eq!(index.status, StatusCode::OK);
    assert!(index.text().contains(&find(app, eid).await.title));
    if repeating {
        assert!(index.text().contains("Repeats weekly"));
        assert!(index.text().contains("3 occurrences remaining"));
    }
    eid
}
async fn inbox_response(app: &TestApp, eid: i64, repeating: bool) {
    let invite = item(app, eid, JASON).await;
    assert_eq!(invite.event_type, "event_invitation");
    let mut jason = app.sign_in(JASON).await;
    let inbox = jason.get("/activity").await;
    assert_eq!(inbox.status, StatusCode::OK);
    let html = inbox.text();
    let itemhtml = rendered::article(&html, &format!("activity_item_{}", invite.id));
    assert!(itemhtml.contains("Event invitation"));
    assert!(itemhtml.contains(&find(app, eid).await.title));
    if repeating {
        assert!(itemhtml.contains("repeats weekly until"));
    }
    let opened = jason.send(rendered::submit(&itemhtml, "Open", &[])).await;
    assert_eq!(opened.status, StatusCode::SEE_OTHER);
    assert_eq!(
        opened.location(),
        Some(format!("http://campfire.test{}", path(eid)).as_str())
    );
    let show = rendered::follow(&mut jason, &opened).await;
    assert_eq!(show.status, StatusCode::OK);
    assert!(show.text().contains(&find(app, eid).await.title));
    let reply = jason
        .send(rendered::submit(&show.text(), "Going", &[]))
        .await;
    redirected(&reply, eid);
    let show = rendered::follow(&mut jason, &reply).await;
    assert_eq!(show.status, StatusCode::OK);
    assert!(visible_text(&show.text()).contains("Currently: Going"));
}
#[tokio::test]
async fn cutover_interaction_schedule_invitation_inbox_open_and_response() {
    let app = app().await;
    let eid = schedule(&app, "Launch retro", Some("Bring your notes."), false).await;
    inbox_response(&app, eid, false).await;
    assert_eq!(
        app.db()
            .read(move |c| CalendarEvent::find(c, eid)?.response_for(c, Some(JASON)))
            .await
            .unwrap()
            .as_deref(),
        Some("going")
    );
}
#[tokio::test]
async fn cutover_interaction_repeating_schedule_invites_once_and_copies_response() {
    let app = app().await;
    let eid = schedule(&app, "Weekly planning", None, true).await;
    assert_eq!(rows(&app, eid).await.len(), 3);
    assert_eq!(item_rows(&app, eid, JASON, None).await.len(), 1);
    inbox_response(&app, eid, true).await;
    assert_eq!(
        app.db()
            .read(move |c| CalendarEvent::find(c, eid)?
                .series_events(c)?
                .iter()
                .map(|e| e.response_for(c, Some(JASON)))
                .collect::<campfire_db::Result<Vec<_>>>())
            .await
            .unwrap(),
        vec![Some("going".into()); 3]
    );
}
#[tokio::test]
async fn cutover_interaction_announcement_card_response_stays_in_requested_frame() {
    let app = app().await;
    let eid = schedule(&app, "Card session", None, false).await;
    let mut jason = app.sign_in(JASON).await;
    let room_url = format!("/rooms/{}", id("designers"));
    let room = jason.get(&room_url).await;
    assert_eq!(room.status, StatusCode::OK);
    assert!(room.text().contains("Scheduled an event: Card session"));
    let matching_cards: Vec<_> = cards(&room.text())
        .into_iter()
        .filter(|card| visible_text(card).contains("Card session"))
        .collect();
    assert_eq!(matching_cards.len(), 1);
    assert!(visible_text(&matching_cards[0]).contains("Organized by David"));
    let (load, frame) = rendered::lazy_frame(&matching_cards[0]);
    let loaded = jason.send(load).await;
    assert_eq!(loaded.status, StatusCode::OK);
    assert!(loaded.text().contains("No response yet"));
    assert!(loaded.text().contains("value=\"going\""));
    let saved = jason
        .send(rendered::submit(&loaded.text(), "Going", &[]))
        .await;
    assert_eq!(saved.status, StatusCode::OK);
    assert_eq!(saved.location(), None);
    assert!(saved.headers.get("turbo-location").is_none());
    assert!(saved.text().contains(&format!("id=\"{frame}\"")));
    assert!(saved.text().contains("Currently: <strong>Going</strong>"));
    // HTTP verifies rendered frame wiring and its response, but cannot prove
    // test/system/events_test.rb:136's actual browser current path (WS14e-101).
    assert_eq!(
        app.db()
            .read(move |c| CalendarEvent::find(c, eid)?.response_for(c, Some(JASON)))
            .await
            .unwrap()
            .as_deref(),
        Some("going")
    );
}

fn visible_text(html: &str) -> String {
    regex::Regex::new("<[^>]*>")
        .unwrap()
        .replace_all(html, " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
