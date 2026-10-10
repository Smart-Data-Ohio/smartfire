//! test/system/events_test.rb: server interaction assertions via real HTTP and Turbo frames.
//! The shared Rails assets are unchanged; no browser pixels are used as a receipt.
use super::super::*;

use super::support::*;
async fn schedule(app: &TestApp, title: &str, description: Option<&str>, repeating: bool) -> i64 {
    let mut david = app.sign_in(DAVID).await;
    let mut input = vec![("event[title]", title), ("event[starts_at]", "2026-09-30T15:30"), ("event[time_zone]", "UTC")];
    if let Some(description) = description { input.push(("event[description]", description)); }
    if repeating {
        input.push(("event[recurrence_rule]", "weekly"));
        input.push(("event[recurrence_until]", "2026-10-14"));
    }
    let reply = david.write(Req::new(Method::POST, &index_path()).form(&input)).await;
    assert_eq!(reply.status, StatusCode::FOUND, "{}", reply.text());
    let title = title.to_owned();
    let eid = app.db().read(move |c| Ok(c.query_row("SELECT MIN(id) FROM events WHERE title=?", [title], |r| r.get::<_, i64>(0))?)).await.unwrap();
    redirected(&reply, eid);
    let event = find(app, eid).await;
    assert_eq!(event.description.as_deref(), description);
    eid
}

async fn inbox_response(app: &TestApp, eid: i64, _repeating: bool) {
    let invite = item(app, eid, JASON).await;
    assert_eq!(invite.event_type, "event_invitation");
    let mut jason = app.sign_in(JASON).await;
    let opened = jason.write(Req::new(Method::POST, &format!("/activity/{}/open", invite.id))).await;
    assert_eq!(opened.status, StatusCode::SEE_OTHER);
    assert_eq!(opened.location(), Some(format!("http://campfire.test{}", path(eid)).as_str()));
    let reply = jason.write(Req::new(Method::PATCH, &format!("{}/attendance", path(eid))).form(&[("response", "going")])).await;
    redirected(&reply, eid);
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
    let message_id = app.db().read(move |conn| {
        Ok(conn.query_row("SELECT message_id FROM event_references WHERE event_id=? ORDER BY message_id LIMIT 1", [eid], |row| row.get::<_, i64>(0))?)
    }).await.unwrap().to_string();
    let mut jason = app.sign_in(JASON).await;
    let saved = jason.write(Req::new(Method::PATCH, &format!("{}/attendance", path(eid))).header("turbo-frame", "event_attendance").form(&[("response", "going"), ("message_id", &message_id)])).await;
    assert_eq!(saved.status, StatusCode::OK);
    assert_eq!(saved.location(), None);
    assert!(saved.headers.get("turbo-location").is_none());
    assert!(saved.body.is_empty());
    assert_eq!(app.db().read(move |c| CalendarEvent::find(c, eid)?.response_for(c, Some(JASON))).await.unwrap().as_deref(), Some("going"));
}
