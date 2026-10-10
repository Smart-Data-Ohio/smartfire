//! Discriminating ports of the remaining original Event declarations.
use super::*;

#[tokio::test]
async fn cutover_recurrence_guard_flag_injection_cannot_change_the_rule() {
    let app = support::app().await;
    let head = app
        .db()
        .write(|tx| {
            CalendarEvent::create(
                tx,
                NewCalendarEvent {
                    room_id: support::id("designers"),
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
    let mut david = app.sign_in(DAVID).await;
    let rejected = david.write(Req::new(Method::PATCH, &support::path(head.id))
        .header("content-type", "application/json")
        .body(serde_json::to_vec(&serde_json::json!({"event":{"allow_recurrence_mutation":true,"recurrence_rule":"daily"}})).unwrap())).await;
    assert_eq!(rejected.status, StatusCode::UNPROCESSABLE_ENTITY);

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

mod controllers;
mod support;

mod attendances;


mod interactions;


mod d_cards;
