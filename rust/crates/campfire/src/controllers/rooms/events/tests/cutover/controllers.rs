//! Open assertions from test/controllers/rooms/events_controller_test.rb, in declaration order.
use super::super::*;
use super::support::*;
use campfire_db::{RoomType, models::calendar_event::changes::EventChanges};
use serde_json::json;
#[tokio::test]
async fn cutover_events_index_separates_upcoming_past_and_cancelled() {
    let app = app().await;
    app.db()
        .write(|tx| {
            CalendarEvent::create(
                tx,
                NewCalendarEvent {
                    room_id: id("designers"),
                    organizer_id: DAVID,
                    title: "Old kickoff".into(),
                    starts_at: Timestamp::parse_db("2026-09-20 12:00:00"),
                    time_zone: "UTC".into(),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
    let mut david = app.sign_in(DAVID).await;
    let reply = david.get(&index_path()).await;
    assert_eq!(reply.status, StatusCode::OK);
    let html = reply.text();
    let (upcoming, rest) = html.split_once("id=\"past-events\"").unwrap();
    let (past, cancelled) = rest.split_once("id=\"cancelled-events\"").unwrap();
    assert!(upcoming.contains("Launch party planning"));
    assert!(!upcoming.contains("Old kickoff"));
    assert!(past.contains("Old kickoff"));
    assert!(cancelled.contains("Sprint retro"));
}
#[tokio::test]
async fn cutover_events_member_create_invites_and_parses_posted_zone() {
    let app = app().await;
    let n = count(&app).await;
    let mut david = app.sign_in(DAVID).await;
    let reply=david.write(json(Method::POST,&index_path(),json!({"event":{"title":"Demo day","description":"Show and tell","starts_at":"2026-09-25T15:30","time_zone":"America/New_York"}}))).await;
    assert_eq!(count(&app).await, n + 1);
    let e = app
        .db()
        .read(|c| {
            let eid = c.query_row("SELECT id FROM events WHERE title='Demo day'", [], |r| {
                r.get(0)
            })?;
            CalendarEvent::find(c, eid)
        })
        .await
        .unwrap();
    redirected(&reply, e.id);
    assert_eq!(e.organizer_id, DAVID);
    assert_eq!(
        e.starts_at,
        Timestamp::parse_db("2026-09-25 19:30:00").unwrap()
    );
    assert_eq!(item(&app, e.id, JASON).await.event_type, "event_invitation");
}
#[tokio::test]
async fn cutover_events_series_create_invites_once_per_member() {
    let app = app().await;
    let n = count(&app).await;
    let mut david = app.sign_in(DAVID).await;
    let reply=david.write(json(Method::POST,&index_path(),json!({"event":{"title":"Weekly planning","starts_at":"2026-09-25T15:30","time_zone":"America/New_York","recurrence_rule":"weekly","recurrence_until":"2026-10-09"}}))).await;
    assert_eq!(count(&app).await, n + 3);
    let head = app
        .db()
        .read(|c| {
            let eid = c.query_row(
                "SELECT MIN(id) FROM events WHERE title='Weekly planning'",
                [],
                |r| r.get(0),
            )?;
            CalendarEvent::find(c, eid)
        })
        .await
        .unwrap();
    redirected(&reply, head.id);
    assert_eq!(head.series_id, Some(head.id));
    assert_eq!(rows(&app, head.id).await.len(), 3);
    for who in [JASON, id("jz"), KEVIN] {
        assert_eq!(
            item_rows(&app, head.id, who, None).await,
            vec![(head.id, "event_invitation".into())]
        );
    }
}
#[tokio::test]
async fn cutover_events_create_above_cap_renders_error_without_writes() {
    let app = app().await;
    let n = count(&app).await;
    let mut david = app.sign_in(DAVID).await;
    let reply=david.write(json(Method::POST,&index_path(),json!({"event":{"title":"Too long","starts_at":"2026-09-25T15:30","time_zone":"UTC","recurrence_rule":"daily","recurrence_until":"2026-12-01"}}))).await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(reply.text().contains("pick an earlier end date"));
    assert_eq!(count(&app).await, n);
}
#[tokio::test]
async fn cutover_events_index_lists_each_past_occurrence() {
    let app = app().await;
    let head = app
        .db()
        .write(|tx| {
            CalendarEvent::create(
                tx,
                NewCalendarEvent {
                    room_id: id("designers"),
                    organizer_id: DAVID,
                    title: "Old planning".into(),
                    starts_at: Timestamp::parse_db("2026-09-12 12:00:00"),
                    time_zone: "UTC".into(),
                    recurrence_rule: Some("daily".into()),
                    recurrence_until: Some("2026-09-14".parse().unwrap()),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
    let rs = rows(&app, head.id).await;
    assert_eq!(rs.len(), 3);
    let mut david = app.sign_in(DAVID).await;
    let reply = david.get(&index_path()).await;
    assert_eq!(reply.status, StatusCode::OK);
    let html = reply.text();
    let rest = html.split_once("id=\"past-events\"").unwrap().1;
    for e in rs {
        assert!(rest.contains(&path(e.id)));
    }
}
#[tokio::test]
async fn cutover_events_following_shift_notifies_once_per_attendee() {
    let app = app().await;
    let head = series_at(
        &app,
        "Weekly planning",
        DAVID,
        "2026-09-25 15:30:00",
        "2026-09-25 16:30:00",
        "2026-10-09",
    )
    .await;
    respond(&app, head.id, JASON, "going").await;
    let mut david = app.sign_in(DAVID).await;
    let reply=david.write(json(Method::PATCH,&path(head.id),json!({"update_scope":"this_and_following","event":{"title":"Weekly planning","starts_at":"2026-09-25T16:30","ends_at":"2026-09-25T17:30","time_zone":"UTC"}}))).await;
    redirected(&reply, head.id);
    let rs = rows(&app, head.id).await;
    assert_eq!(
        rs[1].starts_at,
        Timestamp::parse_db("2026-10-02 16:30:00").unwrap()
    );
    assert_eq!(
        rs[2].starts_at,
        Timestamp::parse_db("2026-10-09 16:30:00").unwrap()
    );
    assert_eq!(
        item_rows(&app, head.id, JASON, None).await,
        vec![(head.id, "event_update".into())]
    );
}
#[tokio::test]
async fn cutover_events_default_update_leaves_other_occurrences_untouched() {
    let app = app().await;
    let head = series(&app, "Weekly planning", DAVID).await;
    let rs = rows(&app, head.id).await;
    let eid = rs[1].id;
    let mut david = app.sign_in(DAVID).await;
    let reply=david.write(json(Method::PATCH,&path(eid),json!({"event":{"title":"Renamed","starts_at":"2026-10-01T16:30","ends_at":"2026-10-01T17:30","time_zone":"UTC"}}))).await;
    redirected(&reply, eid);
    let saved = rows(&app, head.id).await;
    assert_eq!(saved[0].title, "Weekly planning");
    assert_eq!(saved[2].title, "Weekly planning");
    assert_eq!(saved[0].starts_at, rs[0].starts_at);
    assert_eq!(saved[2].starts_at, rs[2].starts_at);
}
#[tokio::test]
async fn cutover_events_follower_rule_change_is_rejected() {
    let app = app().await;
    let head = series(&app, "Weekly planning", DAVID).await;
    let rs = rows(&app, head.id).await;
    let mut david = app.sign_in(DAVID).await;
    let reply=david.write(json(Method::PATCH,&path(rs[1].id),json!({"update_scope":"this_and_following","event":{"title":"Weekly planning","recurrence_rule":"daily"}}))).await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(reply.text().contains("first event"));
    assert_eq!(
        find(&app, head.id).await.recurrence_rule.as_deref(),
        Some("weekly")
    );
}
#[tokio::test]
async fn cutover_events_admin_following_edit_succeeds_and_member_edit_is_denied() {
    let app = app().await;
    let head = series(&app, "Weekly planning", id("jz")).await;
    let rs = rows(&app, head.id).await;
    let eid = rs[1].id;
    let mut jason = app.sign_in(JASON).await;
    let reply=jason.write(json(Method::PATCH,&path(eid),json!({"update_scope":"this_and_following","event":{"title":"Renamed","starts_at":"2026-10-01T15:30","ends_at":"2026-10-01T16:30","time_zone":"UTC"}}))).await;
    redirected(&reply, eid);
    let saved = rows(&app, head.id).await;
    assert_eq!(
        saved.iter().map(|e| e.title.as_str()).collect::<Vec<_>>(),
        vec!["Weekly planning", "Renamed", "Renamed"]
    );
    let mut kevin = app.sign_in(KEVIN).await;
    let denied = kevin
        .write(json(
            Method::PATCH,
            &path(eid),
            json!({"update_scope":"this_and_following","event":{"title":"Hijacked"}}),
        ))
        .await;
    assert_eq!(denied.status, StatusCode::FORBIDDEN);
    assert_eq!(find(&app, eid).await.title, "Renamed");
}
#[tokio::test]
async fn cutover_events_following_cancel_cancels_later_and_notifies_once() {
    let app = app().await;
    let head = series(&app, "Weekly planning", DAVID).await;
    respond(&app, head.id, JASON, "going").await;
    let rs = rows(&app, head.id).await;
    let mut david = app.sign_in(DAVID).await;
    let reply = david
        .write(json(
            Method::PATCH,
            &format!("{}/cancel", path(rs[1].id)),
            json!({"cancel_scope":"this_and_following"}),
        ))
        .await;
    redirected(&reply, rs[1].id);
    let saved = rows(&app, head.id).await;
    assert_eq!(
        saved.iter().map(|e| e.cancelled()).collect::<Vec<_>>(),
        vec![false, true, true]
    );
    assert_eq!(
        item_rows(&app, head.id, JASON, Some("event_cancelled")).await,
        vec![(rs[1].id, "event_cancelled".into())]
    );
}
async fn local_cancel(scope: Option<&str>) {
    let app = app().await;
    let head = series(&app, "Weekly planning", DAVID).await;
    let rs = rows(&app, head.id).await;
    let mut david = app.sign_in(DAVID).await;
    let req = if let Some(scope) = scope {
        json(
            Method::PATCH,
            &format!("{}/cancel", path(rs[1].id)),
            json!({"cancel_scope":scope}),
        )
    } else {
        Req::new(Method::PATCH, &format!("{}/cancel", path(rs[1].id)))
    };
    redirected(&david.write(req).await, rs[1].id);
    assert_eq!(
        rows(&app, head.id)
            .await
            .iter()
            .map(|e| e.cancelled())
            .collect::<Vec<_>>(),
        vec![false, true, false]
    );
}
#[tokio::test]
async fn cutover_events_default_cancel_is_local() {
    local_cancel(None).await;
}
#[tokio::test]
async fn cutover_events_explicit_this_event_cancel_is_local() {
    local_cancel(Some("this_event")).await;
}
#[tokio::test]
async fn cutover_events_show_cancel_scope_inputs_only_for_series() {
    let app = app().await;
    let head = series(&app, "Weekly planning", DAVID).await;
    let rs = rows(&app, head.id).await;
    let mut david = app.sign_in(DAVID).await;
    let reply = david.get(&path(rs[1].id)).await;
    assert_eq!(reply.status, StatusCode::OK);
    for value in ["this_event", "this_and_following"] {
        let inputs = regex::Regex::new("<input\\b[^>]*>").unwrap();
        let html = reply.text();
        assert_eq!(
            inputs
                .find_iter(&html)
                .filter(|m| m.as_str().contains("name=\"cancel_scope\"")
                    && m.as_str().contains(&format!("value=\"{value}\"")))
                .count(),
            1
        );
    }
    let reply = david.get(&path(id("launch_party"))).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert!(!reply.text().contains("name=\"cancel_scope\""));
    assert!(reply.text().contains("Cancel event"));
}
#[tokio::test]
async fn cutover_events_index_query_count_is_independent_of_occurrence_count() {
    let app = app().await;
    let mut heads = Vec::new();
    for _ in 0..2 {
        heads.push(series(&app, "Weekly planning", DAVID).await);
    }
    assert_eq!(rows(&app, heads[0].id).await.len(), 3);
    let mut david = app.sign_in(DAVID).await;
    assert_eq!(david.get(&index_path()).await.status, StatusCode::OK);
    let small = query_count(&app, &mut david, &index_path()).await;
    for head in heads {
        app.db()
            .write(move |tx| {
                CalendarEvent::update_with_scope(
                    tx,
                    head.id,
                    EventChanges {
                        recurrence_until: Some(Some("2026-11-12".parse().unwrap())),
                        ..Default::default()
                    },
                    "this_and_following",
                    Some(DAVID),
                )
            })
            .await
            .unwrap();
        assert_eq!(rows(&app, head.id).await.len(), 8);
    }
    let large = query_count(&app, &mut david, &index_path()).await;
    assert_eq!(small, large, "event index SQL must stay bounded");
}
#[tokio::test]
async fn cutover_events_signed_out_index_redirects_to_sign_in() {
    let app = app().await;
    let mut guest = app.anonymous();
    let reply = guest.get(&index_path()).await;
    assert_eq!(reply.status, StatusCode::FOUND);
    assert_eq!(reply.location(), Some("http://campfire.test/session/new"));
}
#[tokio::test]
async fn cutover_events_update_uses_existing_zone_and_notifies_attendees() {
    let app = app().await;
    let eid = id("launch_party");
    respond(&app, eid, KEVIN, "going").await;
    let mut david = app.sign_in(DAVID).await;
    let reply=david.write(json(Method::PATCH,&path(eid),json!({"event":{"title":"Launch party planning","starts_at":"2026-09-26T15:30","ends_at":"","time_zone":"UTC"}}))).await;
    redirected(&reply, eid);
    let e = find(&app, eid).await;
    assert_eq!(
        e.starts_at,
        Timestamp::parse_db("2026-09-26 19:30:00").unwrap()
    );
    assert_eq!(e.time_zone, "America/New_York");
    assert_eq!(item(&app, eid, KEVIN).await.event_type, "event_update");
}
#[tokio::test]
async fn cutover_events_create_with_venue_persists_the_room() {
    let app = app().await;
    let vid = venue(&app, RoomType::Voice, "Lounge", DAVID, &[DAVID, JASON]).await;
    let mut david = app.sign_in(DAVID).await;
    let reply=david.write(json(Method::POST,&index_path(),json!({"event":{"title":"Voice social","starts_at":"2026-09-25T15:30","time_zone":"UTC","venue_room_id":vid}}))).await;
    let e = app
        .db()
        .read(|c| {
            let eid = c.query_row(
                "SELECT id FROM events WHERE title='Voice social'",
                [],
                |r| r.get(0),
            )?;
            CalendarEvent::find(c, eid)
        })
        .await
        .unwrap();
    redirected(&reply, e.id);
    assert_eq!(e.venue_room_id, Some(vid));
}
#[tokio::test]
async fn cutover_events_update_sets_and_clears_venue() {
    let app = app().await;
    let eid = id("launch_party");
    let vid = venue(&app, RoomType::Voice, "Lounge", DAVID, &[DAVID, JASON]).await;
    let e = find(&app, eid).await;
    let start = campfire_views::time::Zone::for_user(Some(&e.time_zone))
        .format(e.starts_at.jiff(), "%Y-%m-%dT%H:%M");
    let end = campfire_views::time::Zone::for_user(Some(&e.time_zone))
        .format(e.ends_at.unwrap().jiff(), "%Y-%m-%dT%H:%M");
    let mut david = app.sign_in(DAVID).await;
    for v in [json!(vid), json!("")] {
        let reply = david
            .write(json(
                Method::PATCH,
                &path(eid),
                json!({"event":{"starts_at":start,"ends_at":end,"venue_room_id":v}}),
            ))
            .await;
        redirected(&reply, eid);
        assert_eq!(
            find(&app, eid).await.venue_room_id,
            if v == json!("") { None } else { Some(vid) }
        );
    }
}
async fn rejected_create(outsider: bool) {
    let app = app().await;
    let vid = if outsider {
        venue(&app, RoomType::Voice, "Outsiders", JASON, &[JASON]).await
    } else {
        id("designers")
    };
    let n = count(&app).await;
    let mut david = app.sign_in(DAVID).await;
    let reply=david.write(json(Method::POST,&index_path(),json!({"event":{"title":"Bad venue","starts_at":"2026-09-25T15:30","time_zone":"UTC","venue_room_id":vid}}))).await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        reply
            .text()
            .contains("must be a voice or Stage channel you belong to")
    );
    assert_eq!(count(&app).await, n);
}
#[tokio::test]
async fn cutover_events_create_rejects_text_venue_without_writes() {
    rejected_create(false).await;
}
#[tokio::test]
async fn cutover_events_create_rejects_nonmember_venue_without_writes() {
    rejected_create(true).await;
}
#[tokio::test]
async fn cutover_events_update_rejects_nonmember_venue_and_keeps_original() {
    let app = app().await;
    let eid = id("launch_party");
    let vid = venue(&app, RoomType::Voice, "Outsiders", JASON, &[JASON]).await;
    let original = find(&app, eid).await;
    let mut david = app.sign_in(DAVID).await;
    let reply = david
        .write(json(
            Method::PATCH,
            &path(eid),
            json!({"event":{"venue_room_id":vid}}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        reply
            .text()
            .contains("must be a voice or Stage channel you belong to")
    );
    let e = find(&app, eid).await;
    assert_eq!(e.venue_room_id, None);
    assert_eq!(e.starts_at, original.starts_at);
}
#[tokio::test]
async fn cutover_events_editor_keeps_hidden_venue_during_unrelated_edit() {
    let app = app().await;
    let eid = id("launch_party");
    let vid = venue(&app, RoomType::Voice, "Design sync", DAVID, &[DAVID]).await;
    set_venue(&app, eid, vid).await;
    let mut jason = app.sign_in(JASON).await;
    let shown = jason.get(&format!("{}/edit", path(eid))).await;
    assert_eq!(shown.status, StatusCode::OK);
    let html = shown.text();
    let select = html
        .split_once("name=\"event[venue_room_id]\"")
        .unwrap()
        .1
        .split_once("</select>")
        .unwrap()
        .0;
    assert!(select.contains("label=\"Voice\""));
    let voice = select
        .split_once("<optgroup label=\"Voice\">")
        .unwrap()
        .1
        .split_once("</optgroup>")
        .unwrap()
        .0;
    let option = regex::Regex::new("<option[^>]*>Design sync</option>").unwrap();
    let option = option.find(voice).unwrap().as_str();
    assert!(option.contains(&format!("value=\"{vid}\"")));
    assert!(option.contains("selected=\"selected\""));
    let e = find(&app, eid).await;
    let reply=jason.write(json(Method::PATCH,&path(eid),json!({"event":{"starts_at":e.starts_at.jiff().checked_add(jiff::SignedDuration::from_hours(1)).unwrap().to_string(),"ends_at":e.ends_at.unwrap().jiff().checked_add(jiff::SignedDuration::from_hours(1)).unwrap().to_string(),"venue_room_id":vid}}))).await;
    redirected(&reply, eid);
    assert_eq!(find(&app, eid).await.venue_room_id, Some(vid));
}
#[tokio::test]
async fn cutover_events_shared_venue_index_query_count_is_independent_of_event_count() {
    let app = app().await;
    let vid = venue(&app, RoomType::Stage, "Town Hall", DAVID, &[DAVID]).await;
    live(&app, vid).await;
    set_venue(&app, id("launch_party"), vid).await;
    for n in 0..1 {
        app.db()
            .write(move |tx| {
                CalendarEvent::create(
                    tx,
                    NewCalendarEvent {
                        room_id: id("designers"),
                        organizer_id: DAVID,
                        title: format!("Session {n}"),
                        starts_at: Timestamp::parse_db("2026-09-25 12:00:00"),
                        time_zone: "UTC".into(),
                        venue_room_id: Some(vid),
                        ..Default::default()
                    },
                )
            })
            .await
            .unwrap();
    }
    let mut david = app.sign_in(DAVID).await;
    assert_eq!(david.get(&index_path()).await.status, StatusCode::OK);
    let small = query_count(&app, &mut david, &index_path()).await;
    for n in 0..4 {
        app.db()
            .write(move |tx| {
                CalendarEvent::create(
                    tx,
                    NewCalendarEvent {
                        room_id: id("designers"),
                        organizer_id: DAVID,
                        title: format!("Added {n}"),
                        starts_at: Some(
                            tx.now()
                                .since(jiff::SignedDuration::from_hours((4 + n) * 24)),
                        ),
                        time_zone: "UTC".into(),
                        venue_room_id: Some(vid),
                        ..Default::default()
                    },
                )
            })
            .await
            .unwrap();
    }
    let large = query_count(&app, &mut david, &index_path()).await;
    assert_eq!(small, large, "shared venue SQL must stay bounded");
}
#[tokio::test]
async fn cutover_events_show_hides_non_https_meet_link() {
    let app = app().await;
    let eid = id("launch_party");
    app.db()
        .write(move |tx| CalendarEvent::save_meet_link(tx, eid, Some("javascript:alert(1)".into())))
        .await
        .unwrap();
    let mut david = app.sign_in(DAVID).await;
    let reply = david.get(&path(eid)).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert!(!reply.text().contains("Join Google Meet"));
    assert!(!reply.text().contains("javascript:"));
}
