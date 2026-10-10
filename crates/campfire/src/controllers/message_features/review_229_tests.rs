//! Fresh Rails HTTP sections, persisted sends and ordered publications.
//! Failing-first controls: 91cb61210 renders send-now in UTC and grows the
//! visible-row reads faster than Rails. No expected fields construct real output.
use super::{
    comparison_support,
    quote_integration_tests::{app_rows},
};
use crate::controllers::presenters::test_support::*;
use campfire_db::{NewScheduledMessage, ScheduledMessage};
use serde_json::{Value, json};

fn oracle() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/review_229.json"
    ))
    .unwrap()
}

#[tokio::test]
async fn send_now_preserves_viewer_zone_in_fresh_rails_frames_and_rows() {
    let vector = oracle();
    for case in vector["sends"].as_array().unwrap() {
        let inputs = campfire_db::database::FixtureInputs {
            message_uuid: std::sync::Arc::new(|| "fixture-send-now-zone".into()),
            sqlite_now: campfire_db::Timestamp::parse_db(SEED_NOW).unwrap(),
        };
        let app = crate::test_support::with_message_inputs(inputs, app_rows(json!({}))).await;
        let zone = case["zone"].as_str().unwrap().to_owned();
        let id = app
            .db()
            .write(move |tx| {
                tx.conn().execute(
                    "UPDATE users SET time_zone=? WHERE id=?",
                    rusqlite::params![zone, DAVID],
                )?;
                Ok(ScheduledMessage::create(
                    tx,
                    NewScheduledMessage {
                        user_id: DAVID,
                        room_id: QUIET_CORNER,
                        thread_id: None,
                        reply_to_message_id: None,
                        markdown_source: "Review send now".into(),
                        send_at: campfire_db::Timestamp::parse_db("10000-03-05 09:00:00").unwrap(),
                    },
                )?
                .id)
            })
            .await
            .unwrap();

        let response = app
            .david()
            .write(
                Req::new(
                    hyper::Method::POST,
                    &format!("/scheduled_messages/{id}/send_now"),
                )
                .header("accept", "application/json")
                .header("content-type", "application/json")
                .body(b"{}".to_vec()),
            )
            .await;
        assert_eq!(
            json!({"status":response.status.as_u16(),"content_type":response.header("content-type"),"body":response.text()}),
            json!({"status":case["status"],"content_type":case["content_type"],"body":case["body"]}),
            "send-now real HTTP response"
        );
        let expected = case.clone();
        app.db()
            .read(move |conn| {
                let row = ScheduledMessage::find(conn, id)?;
                comparison_support::same_row(
                    &comparison_support::row(conn, "scheduled_messages", id)?,
                    &expected["scheduled"],
                    "send-now persisted schedule",
                );
                comparison_support::same_row(
                    &comparison_support::row(conn, "messages", row.sent_message_id.unwrap())?,
                    &expected["message"],
                    "send-now persisted message",
                );
                Ok(())
            })
            .await
            .unwrap();

    }
    println!(
        "WS8bm2 review229 send-now: 3 viewer zones; complete fresh Rails HTTP responses, persisted rows and ordered frames byte-identical"
    );
}
