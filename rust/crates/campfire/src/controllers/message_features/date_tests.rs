use crate::controllers::presenters::test_support::*;
use axum::http::{Method, StatusCode};
use serde_json::{Value, json};
#[test]
fn builder_calendar_inputs_match_actual_rails_including_exception_and_nil_results() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/date_inputs.json"
    ))
    .unwrap();
    for row in oracle["cases"].as_array().unwrap() {
        let zone = campfire_views::time::Zone::lookup(row["zone"].as_str().unwrap()).unwrap();
        let result = super::parse_time(
            row["input"].as_str().unwrap(),
            &zone,
            SEED_NOW.parse().unwrap(),
        );
        if row["error"].is_string() {
            assert!(result.is_err(), "{row}: {result:?}");
        } else {
            let result = result.unwrap().map(|t| t.jiff());
            let expected = row["result"]
                .as_str()
                .map(|s| s.parse::<jiff::Timestamp>().unwrap());
            assert_eq!(result, expected, "{row}");
        }
    }
}
#[tokio::test]
async fn reminders_and_scheduled_requests_accept_calendar_month_names_and_compact_dates() {
    let app = super::quote_integration_tests::app_rows(json!({})).await;
    let message = app
        .db()
        .write(|tx| {
            campfire_db::Message::create(
                tx,
                campfire_db::NewMessage {
                    room_id: ALL_TALK,
                    creator_id: DAVID,
                    markdown_source: Some("Calendar dates".into()),
                    client_message_id: Some("calendar-date-source".into()),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
    for (path, body) in [
        (
            campfire_routes::saved_items(),
            json!({"message_id":message.id,"remind_at":"March 5, 2026 14:30"}),
        ),
        (
            campfire_routes::room_scheduled_messages(ALL_TALK),
            json!({"scheduled_message":{"markdown_source":"compact scheduled","send_at":"20260305143000"}}),
        ),
    ] {
        let response = app
            .david()
            .write(
                Req::new(Method::POST, &path)
                    .header("accept", "application/json")
                    .header("content-type", "application/json")
                    .body(serde_json::to_vec(&body).unwrap()),
            )
            .await;
        assert_eq!(response.status, StatusCode::CREATED, "{}", response.text());
        assert!(response.text().contains("2026-03-05T14:30:00.000"));
    }
}
