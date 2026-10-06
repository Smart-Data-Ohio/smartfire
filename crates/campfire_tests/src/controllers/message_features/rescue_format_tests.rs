//! Rails chooses formats inside the callback chain, and only normal returns run after-actions.
use crate::controllers::presenters::test_support::*;
use axum::http::Method;
use serde_json::Value;

#[tokio::test]
async fn review_rescue_formats_match_rails_callbacks_actions_and_xhr() {
    let cases: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/review_rescue_formats.json"
    ))
    .unwrap();
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
    let mut browser = app.david();
    let mut differences = Vec::new();
    let mut original_differences = 0;
    let mut original_count = 0;
    let mut nonempty_controls = 0;
    for row in cases.as_array().unwrap() {
        let method = row["method"].as_str().unwrap();
        let path = row["path"].as_str().unwrap();
        let accept = row["accept"].as_str().unwrap();
        let xhr = row["xhr"].as_bool().unwrap();
        let mut request = Req::new(
            Method::from_bytes(method.to_uppercase().as_bytes()).unwrap(),
            path,
        )
        .header("accept", accept)
        .header("content-type", "application/json")
        .body(serde_json::to_vec(&row["input"]).unwrap());
        if xhr {
            request = request.header("x-requested-with", "XMLHttpRequest");
        }
        let response = browser.write(request).await;
        let mut failures = Vec::new();
        if u64::from(response.status.as_u16()) != row["status"].as_u64().unwrap() {
            failures.push(format!(
                "status Rust={} Rails={}",
                response.status, row["status"]
            ));
        }
        for (name, expected) in row["headers"].as_object().unwrap() {
            if response.header(name) != expected.as_str() {
                failures.push(format!(
                    "{name} Rust={:?} Rails={expected}",
                    response.header(name)
                ));
            }
        }
        if response.text() != row["body"].as_str().unwrap() {
            failures.push(format!(
                "body Rust={} bytes Rails={} bytes",
                response.body.len(),
                row["body"].as_str().unwrap().len()
            ));
        }
        if row["original_probe"].as_bool().unwrap() {
            original_count += 1;
            original_differences += usize::from(!failures.is_empty());
        }
        if row["kind"] == "public_exception" && !response.body.is_empty() {
            nonempty_controls += 1;
        }
        if !failures.is_empty() {
            differences.push(format!(
                "{} {method} {path} {accept} xhr={xhr}: {}",
                row["kind"],
                failures.join("; ")
            ));
        }
    }
    println!(
        "WS8bm2 original rescue format probe: {original_count} responses; {original_differences} differences"
    );
    println!(
        "WS8bm2 rescue format matrix: {} responses; 7 headers and body per response; {nonempty_controls} nonempty public-exception controls; {} differences",
        cases.as_array().unwrap().len(),
        differences.len()
    );
    assert!(differences.is_empty(), "{}", differences.join("\n"));
}
