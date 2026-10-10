//! PR #191 regressions: real HTTP, production cache keys and request-free cable delivery.
use crate::controllers::presenters::test_support::*;
use axum::http::{Method, StatusCode};
use campfire_db::{Message, NewMessage};
use serde_json::Value;

#[tokio::test]
async fn review_warm_http_search_query_count_is_constant() {
    let app = TestApp::boot_frozen()
        .await
        .unwrap()
        .without_job_runner()
        .await;
    let mut browser = app.david();
    let mut counts = Vec::new();
    let mut previous = 0;
    for count in [4, 16] {
        for _ in previous..count {
            app.db()
                .write(|tx| {
                    Message::create(
                        tx,
                        NewMessage {
                            room_id: ALL_TALK,
                            creator_id: DAVID,
                            markdown_source: Some("reviewquerycount common".into()),
                            ..Default::default()
                        },
                    )
                })
                .await
                .unwrap();
        }
        let path = "/searches?q=reviewquerycount";
        assert_eq!(browser.get(path).await.status, StatusCode::OK);
        let queries = app.db().capture_read_queries();
        let response = browser.get(path).await;
        app.db().stop_capturing_read_queries();
        assert_eq!(response.status, StatusCode::OK);
        assert_eq!(response.text().matches("data-message-id=").count(), count);
        let reads = queries
            .lock()
            .unwrap()
            .iter()
            .filter(|sql| {
                let sql = sql.trim_start();
                sql.starts_with("SELECT") || sql.starts_with("WITH")
            })
            .count();
        println!("WS8bm2 Rust warm HTTP search: {count} results; {reads} SELECT/WITH executions");
        counts.push(reads);
        previous = count;
    }
    assert_eq!(
        counts[0], counts[1],
        "production cache-key work must be page-scoped"
    );
}

#[tokio::test]
async fn review_denied_not_found_is_empty_in_json_html_and_turbo() {
    let app = TestApp::boot_frozen()
        .await
        .unwrap()
        .without_job_runner()
        .await;
    app.db()
        .write(|tx| {
            tx.conn().execute(
                "DELETE FROM memberships WHERE user_id=? AND room_id=654632876",
                [DAVID],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let paths = [
        (Method::GET, "/rooms/654632876/polls/1"),
        (Method::POST, "/rooms/654632876/polls"),
        (Method::POST, "/rooms/654632876/polls/1/vote"),
        (Method::DELETE, "/messages/935962047/pin"),
        (Method::POST, "/saved"),
        (Method::POST, "/rooms/654632876/scheduled_messages"),
        (Method::POST, "/rooms/654632876/slash_commands"),
        (
            Method::GET,
            "/autocompletable/slash_commands?room_id=654632876",
        ),
    ];
    let mut browser = app.david();
    for accept in [
        "application/json",
        "text/html",
        "text/vnd.turbo-stream.html",
    ] {
        for (method, path) in &paths {
            let response = browser
                .write(
                    Req::new(method.clone(), path)
                        .header("accept", accept)
                        .header("content-type", "application/json")
                        .body(br#"{"message_id":935962047}"#.to_vec()),
                )
                .await;
            assert_eq!(response.status, StatusCode::NOT_FOUND, "{accept}: {path}");
            assert!(
                response.body.is_empty(),
                "{accept}: {path}: {}",
                response.text()
            );
        }
    }
    println!("WS8bm2 denied response formats: 24/24 empty 404 bodies");
}

#[tokio::test]
async fn review_role_room_http_matrix_matches_rails() {
    let cases: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/review_http_matrix.json"
    ))
    .unwrap();
    let mut differences = Vec::new();
    let mut checked = 0;
    let mut bodies = 0;
    for case in cases.as_array().unwrap() {
        let app = TestApp::boot_frozen()
            .await
            .unwrap()
            .without_job_runner()
            .await;
        let role = case["role"].as_i64().unwrap();
        let status = case["user_status"].as_i64().unwrap();
        let kind = case["kind"].as_str().unwrap().to_owned();
        let member = case["member"].as_bool().unwrap();
        app.db()
            .write(move |tx| {
                tx.conn().execute(
                    "UPDATE users SET role=?,status=? WHERE id=?",
                    (role, status, DAVID),
                )?;
                tx.conn()
                    .execute("UPDATE rooms SET type=? WHERE id=654632876", [kind])?;
                if !member {
                    tx.conn().execute(
                        "DELETE FROM memberships WHERE user_id=? AND room_id=654632876",
                        [DAVID],
                    )?;
                }
                Ok(())
            })
            .await
            .unwrap();
        let mut browser = app.david();
        for request in case["requests"].as_array().unwrap() {
            let path = request["path"].as_str().unwrap();
            let method = request["method"].as_str().unwrap();
            let response = browser
                .write(
                    Req::new(
                        Method::from_bytes(method.to_uppercase().as_bytes()).unwrap(),
                        path,
                    )
                    .header("accept", "application/json")
                    .header("content-type", "application/json")
                    .body(serde_json::to_vec(&request["input"]).unwrap()),
                )
                .await;
            checked += 1;
            let context = format!(
                "role={role} status={status} kind={} member={member} {method} {path}",
                case["kind"]
            );
            if u64::from(response.status.as_u16()) != request["status"].as_u64().unwrap() {
                differences.push(format!(
                    "{context}: status Rust={} Rails={}",
                    response.status, request["status"]
                ));
            } else if request["compare_body"].as_bool().unwrap() {
                bodies += 1;
                if response.text() != request["body"].as_str().unwrap() {
                    differences.push(format!(
                        "{context}: body Rust={} Rails={}",
                        response.text(),
                        request["body"]
                    ));
                }
            }
        }
    }
    println!(
        "WS8bm2 role/room matrix: {checked} responses; {bodies} byte comparisons; {} differences",
        differences.len()
    );
    assert!(differences.is_empty(), "{}", differences.join("\n"));
}
