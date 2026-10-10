//! Fresh Rails HTTP sections, persisted sends and ordered publications.
//! Failing-first controls: 91cb61210 renders send-now in UTC and grows the
//! visible-row reads faster than Rails. No expected fields construct real output.
use super::{
    comparison_support,
    quote_integration_tests::{app_rows},
};
use crate::controllers::presenters::test_support::*;
use campfire_db::{Message, NewScheduledMessage, SavedItem, ScheduledMessage};
use serde_json::{Value, json};
use std::collections::BTreeMap;

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

async fn arrange(group: &Value) -> TestApp {
    let app = app_rows(group["rows"].clone()).await;
    let group = group.clone();
    app.db().write(move |tx| {
        for (index,id) in group["old_ids"].as_array().unwrap().iter().enumerate() {
            let id = id.as_i64().unwrap();
            if group["kind"] == "direct" {
                tx.conn().execute("UPDATE messages SET room_id=? WHERE id=?", rusqlite::params![group["direct_room_id"].as_i64().unwrap(),id])?;
            }
            let message = Message::find(tx.conn(),id)?;
            let saved = SavedItem::save_for(tx,DAVID,id,None)?;
            let row = ScheduledMessage::create(tx,NewScheduledMessage {
                user_id:DAVID, room_id:message.room_id, thread_id:None, reply_to_message_id:None,
                markdown_source:"Scaling review <draft> & proof".into(), send_at:campfire_db::Timestamp::parse_db("2026-03-03 16:00:00").unwrap(),
            })?;
            if group["kind"] == "mixed" {
                if index % 2 == 1 { tx.conn().execute("UPDATE saved_items SET status='done' WHERE id=?",[saved.id])?; }
                match index {
                    0 => { tx.conn().execute("UPDATE scheduled_messages SET thread_id=? WHERE id=?", rusqlite::params![group["rows"]["channel_threads"][0]["id"].as_i64().unwrap(),row.id])?; }
                    1 => { tx.conn().execute("UPDATE scheduled_messages SET sent_at=?,sent_message_id=? WHERE id=?",rusqlite::params![tx.now(),id,row.id])?; }
                    2 => { tx.conn().execute("UPDATE scheduled_messages SET dropped_at=?,drop_reason='review access loss' WHERE id=?",rusqlite::params![tx.now(),row.id])?; }
                    3 => {
                        let room: i64 = tx.conn().query_row("SELECT id FROM rooms WHERE id NOT IN (SELECT room_id FROM memberships WHERE user_id=?) AND deleted_at IS NULL ORDER BY id LIMIT 1",[DAVID],|r| r.get(0))?;
                        tx.conn().execute("UPDATE scheduled_messages SET room_id=? WHERE id=?",rusqlite::params![room,row.id])?;
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }).await.unwrap();
    app
}

async fn page_scaling(path: &str, prefix: &str) {
    let vector = oracle();
    // Same existing oracle convention as container_input_tests: Rails disables
    // forgery protection and omits the per-request hidden authenticity input.
    let token =
        regex::Regex::new(r#"<input type="hidden" name="authenticity_token" value="[^"]*" />"#)
            .unwrap();
    let section = regex::Regex::new(&format!(
        r#"(?s)<section class="{prefix}__page".*?</section>"#
    ))
    .unwrap();
    let mut counts: BTreeMap<String, BTreeMap<i64, (usize, i64)>> = BTreeMap::new();
    for group in vector["groups"].as_array().unwrap() {
        let app = arrange(group).await;
        let queries = app.db().capture_queries();
        let response = app.david().get(path).await;
        app.db().stop_capturing_queries();
        let reads = queries.lock().unwrap().len();
        let html = response.text();
        let actual = section
            .find(&html)
            .expect("actual feature section")
            .as_str();
        let actual = token.replace_all(actual, "");
        let expected = group["pages"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["path"] == path)
            .unwrap();
        assert_eq!(
            json!({"status":response.status.as_u16(),"content_type":response.header("content-type"),"html":actual}),
            json!({"status":expected["status"],"content_type":expected["content_type"],"html":expected["html"]}),
            "real feature section differs from fresh Rails: {path} {}",
            group["kind"]
        );
        let size = group["size"].as_i64().unwrap();
        counts
            .entry(group["kind"].as_str().unwrap().to_owned())
            .or_default()
            .insert(size, (reads, expected["reads"].as_i64().unwrap()));
        println!(
            "WS8bm2 review229 visible={size} kind={} path={path}: Rust {reads}; Rails {}",
            group["kind"].as_str().unwrap(),
            expected["reads"]
        );
    }
    for (kind, sizes) in counts {
        let (small, rails_small) = sizes[&4];
        let (large, rails_large) = sizes[&16];
        assert!(
            large as i64 - small as i64 <= rails_large - rails_small,
            "visible-row physical read growth exceeds Rails: {path} {kind}: Rust {small}->{large}; Rails {rails_small}->{rails_large}"
        );
    }
    println!(
        "WS8bm2 review229 {path}: 6 complete fresh Rails HTTP sections byte-identical; visible-row read growth no greater than Rails"
    );
}

#[tokio::test]
async fn saved_visible_row_growth_and_complete_sections_match_fresh_rails() {
    page_scaling("/saved", "saved-items").await;
}
#[tokio::test]
async fn scheduled_visible_row_growth_and_complete_sections_match_fresh_rails() {
    page_scaling("/scheduled_messages", "scheduled-messages").await;
}
