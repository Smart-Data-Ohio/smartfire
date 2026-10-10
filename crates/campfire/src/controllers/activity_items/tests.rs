//! Request contracts captured directly from Rails d7c7de92 on its pinned seed.
use crate::controllers::presenters::test_support::{Req, TestApp};
use axum::http::{Method, StatusCode};

mod review_regressions;
mod lifecycle;
mod board_nudge;

// Rails fixture requests disable forgery verification; remove only those dynamic fields.

#[tokio::test]
async fn ws11ui_inbox_http_matches_pinned_rails_bytes_and_permissions() {
    let corpus: serde_json::Value =
        serde_json::from_str(include_str!("../../../../../vectors/inbox-http.json")).unwrap();
    let setup = corpus["setup"].as_array().unwrap();
    for case in corpus["cases"].as_array().unwrap().iter().filter(|case| case["accept"] == "application/json" || !case["method"].as_str().unwrap().eq_ignore_ascii_case("GET")) {
        let t = TestApp::boot_frozen()
            .await
            .expect("pinned default seed")
            .without_job_runner()
            .await;
        let mut browser = t.david();
        let token = browser.authenticity_token().await;
        let sql = setup
            .iter()
            .chain(case["sql"].as_array().unwrap())
            .map(|s| s.as_str().unwrap().to_owned())
            .collect::<Vec<_>>();
        t.db()
            .write(move |tx| {
                for sql in sql {
                    tx.conn().execute(&sql, [])?;
                }
                Ok(())
            })
            .await
            .unwrap();
        let req = Req::new(
            Method::from_bytes(case["method"].as_str().unwrap().to_uppercase().as_bytes()).unwrap(),
            case["path"].as_str().unwrap(),
        )
        .header("accept", case["accept"].as_str().unwrap())
        .header("turbo-frame", "activity_test")
        .header("content-type", "application/json")
        .header(campfire_kit::csrf::HEADER, &token)
        .body(serde_json::to_vec(&case["params"]).unwrap());
        let response = browser.send(req).await;
        let name = case["name"].as_str().unwrap().to_owned();
        let retired_stream = case["accept"] == "text/vnd.turbo-stream.html" && case["status"].as_u64().unwrap() < 400;
        let expected_status = if retired_stream {
            StatusCode::NOT_ACCEPTABLE.as_u16()
        } else {
            case["status"].as_u64().unwrap() as u16
        };
        assert_eq!(
            response.status.as_u16(),
            expected_status,
            "{name}: {}",
            response.text()
        );
        if retired_stream {
            assert!(response.body.is_empty(), "{name}");
            assert_eq!(response.location(), None, "{name}");
        } else if case["accept"] == "application/json" {
            for key in ["content-type", "cache-control", "pragma", "location"] {
                assert_eq!(response.header(key), case["headers"][key].as_str(), "{name}: {key}");
            }
            assert_eq!(response.text(), case["body"].as_str().unwrap(), "{name}: legacy JSON bytes");
        }

        let expected = case["opened_state"]
            .as_object()
            .map(|_| case["opened_state"].clone())
            .unwrap_or_else(|| case["state"].clone());
        let state_id = if case["opened_state"].is_object() {
            case["path"]
                .as_str()
                .unwrap()
                .split('/')
                .nth(2)
                .unwrap()
                .parse()
                .unwrap()
        } else {
            8400000000
        };
        if expected.is_null() {
            continue;
        }
        t.db()
            .read(move |conn| {
                let row = campfire_db::ActivityItem::find(conn, state_id)?;
                let stamp = |t: campfire_db::Timestamp| {
                    t.jiff().strftime("%Y-%m-%d %H:%M:%S UTC").to_string()
                };
                assert_eq!(
                    row.read_at.map(stamp),
                    expected["read_at"].as_str().map(str::to_owned),
                    "{name} read"
                );
                assert_eq!(
                    row.handled_at.map(stamp),
                    expected["handled_at"].as_str().map(str::to_owned),
                    "{name} handled"
                );
                Ok(())
            })
            .await
            .unwrap();
    }

}
