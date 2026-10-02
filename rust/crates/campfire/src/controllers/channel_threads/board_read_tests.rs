//! Complete HTTP read responses from the pinned Rails app plus approved board drift.
use crate::controllers::presenters::test_support::*;
#[tokio::test]
async fn board_post_forms_pages_and_panes_match_complete_rails_http_responses() {
    let env = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../parity/.env.reference"
    ))
    .unwrap();
    let vapid = env
        .lines()
        .filter_map(|line| line.split_once('='))
        .filter(|(key, _)| matches!(*key, "VAPID_PUBLIC_KEY" | "VAPID_PRIVATE_KEY"))
        .collect::<Vec<_>>();
    let oracle: serde_json::Value =
        serde_json::from_str(include_str!("../../../../../vectors/boards_post_read.json")).unwrap();
    let mut mismatches = Vec::new();
    for row in oracle["rows"].as_array().unwrap() {
        let app = TestApp::boot_frozen_with_env(&vapid)
            .await
            .expect("default seed required");
        fixture_setup(&app, row).await;
        let mut browser = app.sign_in(row["user_id"].as_i64().unwrap()).await;
        let method = row["method"]
            .as_str()
            .unwrap_or("get")
            .to_ascii_uppercase()
            .parse::<axum::http::Method>()
            .unwrap();
        let mut request =
            Req::new(method.clone(), row["path"].as_str().unwrap()).header("user-agent", "Mozilla");
        if let Some(headers) = row["headers"].as_object() {
            for (key, value) in headers {
                request = request.header(key, value.as_str().unwrap());
            }
        }
        let form = row["form"].as_array().map(|pairs| {
            pairs
                .iter()
                .map(|pair| (pair[0].as_str().unwrap(), pair[1].as_str().unwrap()))
                .collect::<Vec<_>>()
        });
        if let Some(form) = form {
            request = request.form(&form);
        }
        let response = if method == axum::http::Method::GET {
            with_fixed_render_secrets(browser.send(request)).await
        } else {
            with_fixed_render_secrets(browser.write(request)).await
        };
        if let Some(location) = row["location"].as_str() {
            assert_eq!(response.location(), Some(location));
        }
        assert_eq!(
            response.status.as_u16(),
            row["status"].as_u64().unwrap() as u16
        );
        let actual = response.text();
        let expected = row["html"].as_str().unwrap();
        if actual != expected {
            let name = row["name"].as_str().unwrap();
            let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../target/ws12-post-diffs");
            std::fs::create_dir_all(&directory).unwrap();
            std::fs::write(directory.join(format!("{name}-actual.html")), &actual).unwrap();
            std::fs::write(directory.join(format!("{name}-expected.html")), expected).unwrap();
            mismatches.push(name.to_string());
        }
    }
    assert!(
        mismatches.is_empty(),
        "Rails body mismatches: {mismatches:?}"
    );
}

async fn fixture_setup(app: &TestApp, row: &serde_json::Value) {
    if let Some(sql) = row["setup"].as_array() {
        let sql = sql
            .iter()
            .map(|sql| sql.as_str().unwrap().to_string())
            .collect::<Vec<_>>();
        app.db()
            .write(move |tx| {
                for sql in sql {
                    tx.conn().execute_batch(&sql)?;
                }
                Ok(())
            })
            .await
            .unwrap();
    }
}
