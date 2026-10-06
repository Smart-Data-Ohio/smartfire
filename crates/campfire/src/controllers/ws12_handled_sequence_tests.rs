//! The original Rails handled -> unhandled -> unread sequence uses one fixture.
use super::presenters::test_support::{DAVID, Req, TestApp};
use campfire_kit::{FrozenClock, Method};
use serde_json::{Value, json};
use std::sync::Arc;

#[tokio::test]
async fn ws12_handled_unhandled_unread_http_sequence_matches_rails() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../vectors/ws12_handled_sequence.json"
    ))
    .unwrap();
    let clock = Arc::new(FrozenClock::new("2026-03-02T16:00:00Z".parse().unwrap()));
    let app = TestApp::boot_with_test_clock(clock.clone())
        .await
        .unwrap()
        .without_job_runner()
        .await;
    app.db().write(|tx| {
        tx.conn().execute_batch("DELETE FROM activity_items; INSERT INTO activity_items(id,user_id,source_type,source_id,event_type,created_at,updated_at) VALUES(8400000000,127326141,'Message',136976342,'mention','2026-03-02 16:00:00','2026-03-02 16:00:00');")?;
        Ok(())
    }).await.unwrap();
    let mut browser = app.david();
    for step in oracle["steps"].as_array().unwrap() {
        let request = Req::new(
            Method::from_bytes(step["method"].as_str().unwrap().to_uppercase().as_bytes()).unwrap(),
            step["path"].as_str().unwrap(),
        )
        .header("accept", "application/json")
        .header("content-type", "application/json")
        .body(serde_json::to_vec(&step["params"]).unwrap());
        let response = browser.write(request).await;
        let name = step["name"].as_str().unwrap();
        assert_eq!(
            response.status.as_u16() as u64,
            step["status"].as_u64().unwrap(),
            "{name}: status"
        );
        assert_eq!(
            response.text(),
            step["body"].as_str().unwrap(),
            "{name}: complete response bytes"
        );
        for key in ["content-type", "cache-control", "pragma", "location"] {
            assert_eq!(
                response.header(key),
                step["headers"][key].as_str(),
                "{name}: {key}"
            );
        }
        let actual = app.db().read(|conn| {
            let item = campfire_db::ActivityItem::find(conn,8400000000)?;
            let stamp = |t: campfire_db::Timestamp| t.jiff().strftime("%Y-%m-%dT%H:%M:%S.000Z").to_string();
            Ok(json!({"read_at":item.read_at.map(stamp),"handled_at":item.handled_at.map(stamp),"updated_at":stamp(item.updated_at),"read":item.read(),"unread":item.unread(),"handled":item.handled()}))
        }).await.unwrap();
        assert_eq!(
            actual, step["state"],
            "{name}: retained read timestamp and complete persisted state"
        );
        let count = app
            .db()
            .read(|conn| {
                campfire_db::ActivityItem::unread_count(
                    conn,
                    &campfire_db::User::find(conn, DAVID)?,
                )
            })
            .await
            .unwrap();
        assert_eq!(
            count as u64,
            step["unread_count"].as_u64().unwrap(),
            "{name}: unread count"
        );
        clock.advance(jiff::SignedDuration::from_secs(1));
    }
    println!(
        "WS12_HANDLED_SEQUENCE 4 HTTP responses; 4 persisted states; 4 unread counts; 0 mismatches"
    );
}
