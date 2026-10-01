//! Request contracts captured directly from Rails d7c7de92 on its pinned seed.
use crate::controllers::presenters::test_support::{Req, TestApp};
use axum::http::{Method, StatusCode};

// Rails fixture requests disable forgery verification; remove only those dynamic fields.
fn page_bytes(html: &str) -> String {
    let forms = regex::Regex::new(r#"<input type="hidden" name="authenticity_token" value="[^"]*"(?: autocomplete="off")? />"#).unwrap().replace_all(html, "");
    regex::Regex::new(r#"<meta name="csrf-param" content="authenticity_token" />\n<meta name="csrf-token" content="[^"]*" />"#).unwrap().replace_all(&forms, "").into_owned()
}
#[tokio::test]
async fn ws11ui_inbox_http_matches_pinned_rails_bytes_and_permissions() {
    let corpus: serde_json::Value =
        serde_json::from_str(include_str!("../../../../../vectors/inbox-http.json")).unwrap();
    let setup = corpus["setup"].as_array().unwrap();
    let mut differences = Vec::new();
    for case in corpus["cases"].as_array().unwrap() {
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
        assert_eq!(
            response.status.as_u16(),
            case["status"].as_u64().unwrap() as u16,
            "{name}: {}",
            response.text()
        );
        if response.status != StatusCode::NOT_FOUND {
            for key in ["content-type", "cache-control", "pragma", "location"] {
                assert_eq!(
                    response.header(key),
                    case["headers"][key].as_str(),
                    "{name}: {key}"
                );
            }
            let body = page_bytes(&response.text());
            let expected = page_bytes(case["body"].as_str().unwrap());
            if body != expected {
                let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../target/ws11ui-inbox-diffs");
                std::fs::create_dir_all(&directory).unwrap();
                let file = format!("{}.txt", name.replace(['/', '?', ' ', '\"'], "_"));
                std::fs::write(directory.join(format!("actual-{file}")), &body).unwrap();
                std::fs::write(directory.join(format!("expected-{file}")), &expected).unwrap();
                differences.push(name.clone());
            }
        }
        let expected = case["state"].clone();
        if expected.is_null() {
            continue;
        }
        t.db()
            .read(move |conn| {
                let row = campfire_db::ActivityItem::find(conn, 8400000000)?;
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
    assert!(
        differences.is_empty(),
        "Rails response differences: {differences:?}"
    );
}
