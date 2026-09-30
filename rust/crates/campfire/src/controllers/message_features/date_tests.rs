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
#[tokio::test]
async fn review_regression_compact_date_with_clock_does_not_send_early() {
    let clock = std::sync::Arc::new(campfire_kit::clock::FrozenClock::new(
        SEED_NOW.parse().unwrap(),
    ));
    let app = TestApp::boot_with_test_clock(clock.clone()).await.unwrap();
    app.db()
        .write(|tx| {
            tx.conn()
                .execute("UPDATE users SET time_zone='UTC' WHERE id=?", [DAVID])?;
            Ok(())
        })
        .await
        .unwrap();
    let response=app.david().write(Req::new(Method::POST,&format!("/rooms/{ALL_TALK}/scheduled_messages")).header("accept","application/json").header("content-type","application/json").body(serde_json::to_vec(&json!({"scheduled_message":{"markdown_source":"Reviewer date probe","send_at":"20260305 14:30"}})).unwrap())).await;
    assert_eq!(response.status, StatusCode::CREATED);
    println!("REVIEW_EARLY_HTTP {}", response.text());
    let id = response.json()["id"].as_i64().unwrap();
    clock.set("2026-03-05T00:01:00Z".parse().unwrap());
    let sent = app
        .db()
        .write(move |tx| campfire_db::ScheduledMessage::dispatch(tx, id, tx.now(), false))
        .await
        .unwrap();
    println!("REVIEW_EARLY_DISPATCH sent_at_00_01={sent} expected_due_14_30=true");
    assert!(
        !sent,
        "a calendar date with a trailing clock sent 14h29m early"
    );
    let stored = app
        .db()
        .read(move |conn| campfire_db::ScheduledMessage::find(conn, id))
        .await
        .unwrap();
    assert_eq!(stored.send_at.to_db(), "2026-03-05 14:30:00");
    clock.set("2026-03-05T14:30:00Z".parse().unwrap());
    assert!(
        app.db()
            .write(move |tx| campfire_db::ScheduledMessage::dispatch(tx, id, tx.now(), false))
            .await
            .unwrap()
    );
}

#[test]
fn review_regression_compact_clocks_offsets_and_dst_match_rails() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/review_dates.json"
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
        let slash = campfire_db::slash_commands::time_parser::parse(
            row["input"].as_str().unwrap(),
            row["zone"].as_str().unwrap(),
            campfire_db::Timestamp::from_jiff(SEED_NOW.parse().unwrap()),
        );
        let expected = row["slash_result"]
            .as_str()
            .map(|s| s.parse::<jiff::Timestamp>().unwrap());
        if slash.map(|t| t.jiff()) != expected {
            mismatches.push(format!("slash {row}: {slash:?}"));
        }
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
    println!("WS8bm2 review dates: 88/88 Rails compact/offset/DST cases match");
}

#[tokio::test]
async fn compact_and_offset_requests_store_rails_times_and_dispatch_only_when_due() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/review_dates.json"
    ))
    .unwrap();
    let clock = std::sync::Arc::new(campfire_kit::clock::FrozenClock::new(
        SEED_NOW.parse().unwrap(),
    ));
    let app = TestApp::boot_with_test_clock(clock.clone()).await.unwrap();
    let mut browser = app.david();
    let message = app
        .db()
        .write(|tx| {
            campfire_db::Message::create(
                tx,
                campfire_db::NewMessage {
                    room_id: ALL_TALK,
                    creator_id: DAVID,
                    markdown_source: Some("Compact reminder source".into()),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
    let mut checked = 0;
    for row in oracle["cases"].as_array().unwrap() {
        let Some(expected) = row["result"].as_str() else {
            continue;
        };
        let expected = campfire_db::Timestamp::from_jiff(expected.parse().unwrap());
        clock.set(SEED_NOW.parse().unwrap());
        let zone = row["zone"].as_str().unwrap().to_owned();
        app.db()
            .write(move |tx| {
                tx.conn()
                    .execute("UPDATE users SET time_zone=? WHERE id=?", (zone, DAVID))?;
                Ok(())
            })
            .await
            .unwrap();
        for (path, body) in [
            (
                "/saved".to_owned(),
                json!({"message_id":message.id,"remind_at":row["input"]}),
            ),
            (
                format!("/rooms/{ALL_TALK}/scheduled_messages"),
                json!({"scheduled_message":{"markdown_source":"Compact clock due check","send_at":row["input"]}}),
            ),
        ] {
            let response = browser
                .write(
                    Req::new(Method::POST, &path)
                        .header("accept", "application/json")
                        .header("content-type", "application/json")
                        .body(serde_json::to_vec(&body).unwrap()),
                )
                .await;
            assert_eq!(
                response.status,
                StatusCode::CREATED,
                "{row}: {}",
                response.text()
            );
            let id = response.json()["id"].as_i64().unwrap();
            if path == "/saved" {
                let saved = app
                    .db()
                    .read(move |conn| campfire_db::SavedItem::find(conn, id))
                    .await
                    .unwrap();
                assert_eq!(saved.remind_at, Some(expected), "{row}");
            } else {
                let scheduled = app
                    .db()
                    .read(move |conn| campfire_db::ScheduledMessage::find(conn, id))
                    .await
                    .unwrap();
                assert_eq!(scheduled.send_at, expected, "{row}");
                clock.set(
                    expected
                        .jiff()
                        .checked_sub(jiff::SignedDuration::from_secs(1))
                        .unwrap(),
                );
                assert!(
                    !app.db()
                        .write(move |tx| campfire_db::ScheduledMessage::dispatch(
                            tx,
                            id,
                            tx.now(),
                            false
                        ))
                        .await
                        .unwrap(),
                    "early: {row}"
                );
                clock.set(expected.jiff());
                assert!(
                    app.db()
                        .write(move |tx| campfire_db::ScheduledMessage::dispatch(
                            tx,
                            id,
                            tx.now(),
                            false
                        ))
                        .await
                        .unwrap(),
                    "due: {row}"
                );
            }
        }
        checked += 1;
    }
    println!(
        "WS8bm2 date HTTP: {checked} saved timestamps, {checked} scheduled timestamps and {checked} before/due dispatch pairs match Rails"
    );
}
