//! Discriminating ports of the remaining original Event declarations.
use super::*;

#[tokio::test]
async fn cutover_recurrence_guard_flag_injection_cannot_change_the_rule() {
    let app = TestApp::boot().await.expect("pinned default seed");
    let app = app.without_job_runner().await;
    let head = app
        .db()
        .write(|tx| {
            CalendarEvent::create(
                tx,
                NewCalendarEvent {
                    room_id: ALL_TALK,
                    organizer_id: DAVID,
                    title: "Weekly sync".into(),
                    starts_at: Timestamp::parse_db("2026-10-05 09:00:00"),
                    time_zone: "UTC".into(),
                    recurrence_rule: Some("weekly".into()),
                    recurrence_until: Some("2026-10-19".parse().unwrap()),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
    let mut david = app.david();
    let rejected = david.write(Req::new(Method::PATCH, &format!("/rooms/{ALL_TALK}/events/{}",head.id))
        .header("content-type", "application/json")
        .body(serde_json::to_vec(&serde_json::json!({"event":{"allow_recurrence_mutation":true,"recurrence_rule":"daily"}})).unwrap())).await;
    assert_eq!(rejected.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(rejected.text().contains("first event"));
    assert_eq!(
        app.db()
            .read(move |c| CalendarEvent::find(c, head.id))
            .await
            .unwrap()
            .recurrence_rule
            .as_deref(),
        Some("weekly")
    );
}
