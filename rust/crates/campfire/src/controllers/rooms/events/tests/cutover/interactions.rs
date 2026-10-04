//! test/system/events_test.rb: server interaction assertions via real HTTP and Turbo frames.
//! The shared Rails assets are unchanged; no browser pixels are used as a receipt.
use super::super::*;
use super::support::*;
use serde_json::json;
async fn schedule(app: &TestApp, title: &str, description: Option<&str>, repeating: bool) -> i64 {
    let mut david = app.sign_in(DAVID).await;
    let room = david.get(&format!("/rooms/{}", id("designers"))).await;
    assert_eq!(room.status, StatusCode::OK);
    assert!(room.text().contains(&index_path()));
    let index = david.get(&index_path()).await;
    assert_eq!(index.status, StatusCode::OK);
    assert!(index.text().contains("New event"));
    assert!(index.text().contains(&format!("{}/new", index_path())));
    let form = david.get(&format!("{}/new", index_path())).await;
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
    let mut input = json!({"title":title,"starts_at":"2026-09-30T15:30","time_zone":"UTC"});
    if let Some(description) = description {
        input["description"] = json!(description);
    }
    if repeating {
        input["recurrence_rule"] = json!("weekly");
        input["recurrence_until"] = json!("2026-10-14");
    }
    let reply = david
        .write(json(Method::POST, &index_path(), json!({"event":input})))
        .await;
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
    let shown = david.get(&path(eid)).await;
    assert_eq!(shown.status, StatusCode::OK);
    assert!(
        shown
            .text()
            .contains(&format!(">{}</h1>", find(app, eid).await.title))
    );
    if let Some(description) = description {
        assert!(shown.text().contains(description));
    }
    assert!(visible_text(&shown.text()).contains("Currently: Going"));
    assert!(shown.text().contains("All events"));
    if repeating {
        assert!(shown.text().contains("Part of a series"));
        assert!(shown.text().contains("Next occurrence"));
    }
    let index = david.get(&index_path()).await;
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
    let itemhtml = html
        .split_once(&format!("id=\"activity_item_{}\"", invite.id))
        .unwrap()
        .1
        .split_once("</article>")
        .unwrap()
        .0;
    assert!(itemhtml.contains("Event invitation"));
    assert!(itemhtml.contains(&find(app, eid).await.title));
    if repeating {
        assert!(itemhtml.contains("repeats weekly until"));
    }
    let opened = jason
        .write(Req::new(
            Method::POST,
            &format!("/activity/{}/open", invite.id),
        ))
        .await;
    assert_eq!(opened.status, StatusCode::SEE_OTHER);
    assert_eq!(
        opened.location(),
        Some(format!("http://campfire.test{}", path(eid)).as_str())
    );
    let show = jason.get(&path(eid)).await;
    assert_eq!(show.status, StatusCode::OK);
    assert!(show.text().contains(&find(app, eid).await.title));
    let reply = jason
        .write(json(
            Method::PATCH,
            &format!("{}/attendance", path(eid)),
            json!({"response":"going"}),
        ))
        .await;
    redirected(&reply, eid);
    let show = jason.get(&path(eid)).await;
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
    let mid=app.db().read(move |c|Ok(c.query_row("SELECT message_id FROM event_references WHERE event_id=? ORDER BY message_id LIMIT 1",[eid],|r|r.get::<_,i64>(0))?)).await.unwrap();
    let mut jason = app.sign_in(JASON).await;
    let room_url = format!("/rooms/{}", id("designers"));
    let room = jason.get(&room_url).await;
    assert_eq!(room.status, StatusCode::OK);
    assert!(room.text().contains("Scheduled an event: Card session"));
    assert!(room.text().contains("event-card__title"));
    assert!(room.text().contains("Organized by David"));
    let frame = format!("response_for_message_{mid}_event_{eid}");
    let url = format!("{}/attendance?message_id={mid}", path(eid));
    assert!(room.text().contains(&url));
    let loaded = jason
        .send(Req::new(Method::GET, &url).header("turbo-frame", &frame))
        .await;
    assert_eq!(loaded.status, StatusCode::OK);
    assert!(loaded.text().contains("No response yet"));
    assert!(loaded.text().contains("value=\"going\""));
    let saved = jason
        .write(
            json(
                Method::PATCH,
                &format!("{}/attendance", path(eid)),
                json!({"response":"going","message_id":mid.to_string()}),
            )
            .header("turbo-frame", &frame),
        )
        .await;
    assert_eq!(saved.status, StatusCode::OK);
    assert_eq!(saved.location(), None);
    assert!(saved.text().contains(&format!("id=\"{frame}\"")));
    assert!(saved.text().contains("Currently: <strong>Going</strong>"));
    let room = jason.get(&room_url).await;
    assert!(room.text().contains("Scheduled an event: Card session"));
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
