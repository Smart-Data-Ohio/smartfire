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

#[test]
fn broader_calendar_inputs_match_actual_rails() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/date_coercions.json"
    ))
    .unwrap();
    let mut mismatches = Vec::new();
    for row in oracle["cases"].as_array().unwrap() {
        let zone = campfire_views::time::Zone::lookup(row["zone"].as_str().unwrap()).unwrap();
        let result = super::parse_time(
            row["input"].as_str().unwrap(),
            &zone,
            SEED_NOW.parse().unwrap(),
        );
        let matches = if row["error"].is_string() {
            result.is_err()
        } else {
            result.as_ref().ok().map(|t| t.map(|t| t.jiff()))
                == Some(
                    row["result"]
                        .as_str()
                        .map(|s| s.parse::<jiff::Timestamp>().unwrap()),
                )
        };
        if !matches {
            mismatches.push(format!("{row}: {result:?}"));
        }
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

#[test]
fn request_string_and_presence_coercions_match_actual_rails() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/date_coercions.json"
    ))
    .unwrap();
    for row in oracle["coercions"].as_array().unwrap() {
        let param = campfire_kit::Param::from_json(row["input"].clone());
        assert_eq!(
            super::param_string(&param),
            row["string"].as_str().unwrap(),
            "{row}"
        );
        assert_eq!(
            param.is_present(),
            row["present"].as_bool().unwrap(),
            "{row}"
        );
    }
}

#[tokio::test]
async fn reminder_parameter_shapes_match_eight_actual_rails_responses() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/date_coercions.json"
    ))
    .unwrap();
    let app = super::quote_integration_tests::app_rows(oracle["rows"].clone()).await;
    for case in oracle["steps"].as_array().unwrap() {
        let response = app
            .david()
            .write(
                Req::new(Method::POST, "/saved")
                    .header("accept", "application/json")
                    .header("content-type", "application/json")
                    .body(serde_json::to_vec(&case["input"]).unwrap()),
            )
            .await;
        assert_eq!(
            response.status.as_u16(),
            case["status"].as_u64().unwrap() as u16,
            "{case}: {}",
            response.text()
        );
        assert_eq!(
            serde_json::from_slice::<Value>(&response.body).unwrap(),
            serde_json::from_str::<Value>(case["body"].as_str().unwrap()).unwrap(),
            "{case}"
        );
    }
}
#[tokio::test]
async fn array_search_queries_and_slash_text_keep_their_ruby_strings() {
    let app = super::quote_integration_tests::app_rows(json!({})).await;
    let response = app.david().get("/searches?q%5B%5D=coercionword").await;
    assert_eq!(response.status, StatusCode::OK);
    assert!(response.text().contains("[&quot;coercionword&quot;]"));
    let response = app
        .david()
        .write(
            Req::new(Method::POST, &format!("/rooms/{ALL_TALK}/slash_commands"))
                .header("content-type", "application/json")
                .body(serde_json::to_vec(&json!({"text":["/shrug"]})).unwrap()),
        )
        .await;
    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(
        serde_json::from_slice::<Value>(&response.body).unwrap()["status"],
        "error"
    );
}
