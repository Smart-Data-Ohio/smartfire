use campfire_db::Timestamp;
use campfire_kit::StatusCode;
use campfire_runtime::presenters::accounts::audit_logs::{filters, selection, csv_body, parse_date};
use campfire_presentation::time::Zone;
use crate::controllers::presenters::test_support::*;
use axum::http::Method;
use serde_json::Value;

fn vectors() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../../vectors/users_audit_logs.json"
    ))
    .unwrap()
}
async fn fixture() -> TestApp {
    let app = TestApp::boot_frozen().await.expect("seed required");
    let rows = vectors()["rows"].clone();
    app.db().write(move |tx| {
        tx.conn().execute("DELETE FROM audit_logs",[])?;
        for row in rows.as_array().unwrap() {
            let created=Timestamp::from_jiff(row["created_at"].as_str().unwrap().parse().unwrap());
            tx.conn().execute("INSERT INTO audit_logs(id,action,actor_label,target_type,target_label,details,ip_address,user_agent,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?,?)",rusqlite::params![row["id"].as_i64(),row["action"].as_str(),row["actor_label"].as_str(),row["target_type"].as_str(),row["target_label"].as_str(),row["details"].to_string(),row["ip_address"].as_str(),row["user_agent"].as_str(),created,created])?;
        }
        Ok(())
    }).await.unwrap();
    app
}
fn assert_bytes(name: &str, actual: &str, expected: &str) {
    if let Ok(dir) = std::env::var("WS8BR2_DIFF_DIR") {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(format!("{dir}/{name}.actual"), actual).unwrap();
        std::fs::write(format!("{dir}/{name}.expected"), expected).unwrap();
    }
    assert_eq!(actual, expected, "{name}: complete Rails bytes");
}
#[tokio::test]
async fn html_navigation_csv_and_filtered_order_match_rails() {
    let app = fixture().await;
    for case in vectors()["cases"].as_array().unwrap() {
        let filters = filters(|k| case["query"][k].as_str().map(str::to_owned));
        let selection = selection(&filters, &Zone::utc()).unwrap();
        let (_count, all) = app
            .db()
            .read(move |c| Ok((selection.count(c)?, selection.entries(c, 5000, 0)?)))
            .await
            .unwrap();
        assert_eq!(
            serde_json::to_value(all.iter().map(|r| r.id).collect::<Vec<_>>()).unwrap(),
            case["ids"],
            "{}",
            case["name"]
        );
        assert_bytes(
            "audit-csv",
            &csv_body(&all, &Zone::utc()),
            case["csv"].as_str().unwrap(),
        );

    }
}
#[test]
fn full_dates_and_invalid_inputs_match_rails_date_parse() {
    for case in vectors()["dates"].as_array().unwrap() {
        assert_eq!(
            parse_date(case["input"].as_str().unwrap()).map(|d| d.to_string()),
            case["date"].as_str().map(str::to_owned),
            "{}",
            case["input"]
        );
    }
}
#[tokio::test]
async fn authorization_and_csv_sudo_are_checked_before_export() {
    let app = fixture().await;
    for path in ["/account/audit_log", "/account/audit_log.csv"] {
        assert_eq!(
            app.anonymous().get(path).await.location(),
            Some("http://campfire.test/session/new")
        );
        assert_eq!(
            app.sign_in(KEVIN).await.send(Req::new(Method::GET, path).header("x-requested-with", "XMLHttpRequest")).await.status,
            StatusCode::FORBIDDEN
        );
    }
    let mut browser = app.david();
    assert_eq!(
        browser.get("/account/audit_log").await.status,
        StatusCode::FOUND
    );
    assert_eq!(
        browser.get("/account/audit_log.csv").await.location(),
        Some("http://campfire.test/sudo/new")
    );
    assert_eq!(
        browser
            .write(Req::new(Method::POST, "/sudo").form(&[("password", "secret123456")]))
            .await
            .status,
        StatusCode::FOUND
    );
    let response = browser
        .get("/account/audit_log.csv?audit_action=user.ban")
        .await;
    assert_eq!(response.status, StatusCode::OK);
    assert!(
        response
            .header("content-type")
            .unwrap()
            .starts_with("text/csv")
    );
    assert!(
        response
            .header("content-disposition")
            .unwrap()
            .contains("audit-log-20260302-160000.csv")
    );
    assert_eq!(response.header("cache-control"), Some("no-store"));
    assert_bytes(
        "http-audit-csv",
        &response.text(),
        vectors()["cases"][4]["csv"].as_str().unwrap(),
    );
}
#[tokio::test]
async fn export_cap_and_truncated_filename_use_filtered_count() {
    let app = fixture().await;
    app.db().write(|tx| {
        tx.conn().execute("WITH RECURSIVE n(i) AS (VALUES(1) UNION ALL SELECT i+1 FROM n WHERE i<5001) INSERT INTO audit_logs(id,action,details,created_at,updated_at) SELECT i,'user.unban','{}',?,? FROM n",rusqlite::params![tx.now(),tx.now()])?;Ok(())
    }).await.unwrap();
    let mut browser = app.david();
    browser
        .write(Req::new(Method::POST, "/sudo").form(&[("password", "secret123456")]))
        .await;
    let csv = browser
        .get("/account/audit_log.csv?audit_action=user.unban")
        .await;
    assert!(
        csv.header("content-disposition")
            .unwrap()
            .contains("truncated-to-5000")
    );
    assert_eq!(csv.text().lines().count(), 5001);
    let under = browser
        .get("/account/audit_log.csv?audit_action=room.create")
        .await;
    assert!(
        !under
            .header("content-disposition")
            .unwrap()
            .contains("truncated")
    );
}
