//! test/controllers/rooms/events/attendances_controller_test.rb: original propagation assertions.
use super::super::*;
use super::support::*;
use serde_json::json;
async fn responses(app: &TestApp, head: i64) -> Vec<Option<String>> {
    app.db()
        .read(move |c| {
            CalendarEvent::find(c, head)?
                .series_events(c)?
                .iter()
                .map(|e| e.response_for(c, Some(KEVIN)))
                .collect()
        })
        .await
        .unwrap()
}
#[tokio::test]
async fn cutover_attendance_head_response_copies_to_each_future_occurrence() {
    let app = app().await;
    let head = series(&app, "Weekly planning", DAVID).await;
    let mut kevin = app.sign_in(KEVIN).await;
    let reply = kevin
        .write(json(
            Method::PATCH,
            &format!("{}/attendance", path(head.id)),
            json!({"response":"going"}),
        ))
        .await;
    redirected(&reply, head.id);
    assert_eq!(
        responses(&app, head.id).await,
        vec![Some("going".into()); 3]
    );
}
#[tokio::test]
async fn cutover_attendance_later_response_is_local_until_apply_future_checked() {
    let app = app().await;
    let head = series(&app, "Weekly planning", DAVID).await;
    let rs = rows(&app, head.id).await;
    let eid = rs[1].id;
    let mut kevin = app.sign_in(KEVIN).await;
    let reply = kevin
        .write(json(
            Method::PATCH,
            &format!("{}/attendance", path(eid)),
            json!({"response":"going"}),
        ))
        .await;
    redirected(&reply, eid);
    assert_eq!(
        responses(&app, head.id).await,
        vec![None, Some("going".into()), None]
    );
    let reply = kevin
        .write(json(
            Method::PATCH,
            &format!("{}/attendance", path(eid)),
            json!({"response":"maybe","apply_to_future":"1"}),
        ))
        .await;
    redirected(&reply, eid);
    assert_eq!(
        responses(&app, head.id).await,
        vec![None, Some("maybe".into()), Some("maybe".into())]
    );
}
