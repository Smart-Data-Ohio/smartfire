use super::*;
use campfire_db::Timestamp;
use campfire_db::models::audit_log::browsing;
use campfire_db::models::audit_log;
use campfire_kit::StatusCode;
use campfire_views::accounts::audit_logs as views;
use campfire_views::time::Zone;
use crate::controllers::presenters::{pagination::Page, test_support::*};
use askama::Template;
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
        let (count, all) = app
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
        let page = Page::new(case["query"]["page"].as_str(), count, &[50]);
        let entries = page
            .records(&all)
            .into_iter()
            .map(entry)
            .collect::<Vec<_>>();
        for block in ["html", "nav"] {
            let actual = crate::controllers::users::people_tests::render_with(
                &app,
                |_| {},
                |ctx| {
                    let show = views::Show {
                        ctx,
                        filters: filters.clone(),
                        entries: entries.clone(),
                        actions: audit_log::actions(),
                        target_types: browsing::TARGET_TYPES.iter().map(|s| (*s).into()).collect(),
                        export_truncated: case["name"] == "query_0",
                        export_limit: "5,000".into(),
                        first_page: page.number == 1,
                        next_page: (!page.is_last()).then(|| page.next_param().to_string()),
                    };
                    if block == "html" {
                        show.as_content().render().unwrap()
                    } else {
                        show.as_nav().render().unwrap()
                    }
                },
            );
            assert_bytes(
                &format!("audit-{}-{block}", case["name"].as_str().unwrap()),
                &actual,
                case[block].as_str().unwrap(),
            );
        }
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
            app.sign_in(KEVIN).await.get(path).await.status,
            StatusCode::FORBIDDEN
        );
    }
    let mut browser = app.david();
    assert_eq!(
        browser.get("/account/audit_log").await.status,
        StatusCode::OK
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
async fn pages_show_fifty_rows_and_links_keep_filters() {
    let app = fixture().await;
    let first = app.david().get("/account/audit_log").await;
    assert_eq!(first.status, StatusCode::OK);
    assert_eq!(first.text().matches("<td><code>").count(), 50);
    assert!(first.text().contains("Older entries"));
    assert!(!first.text().contains("Newest entries"));
    let second = app.david().get("/account/audit_log?page=2").await;
    assert_eq!(second.text().matches("<td><code>").count(), 15);
    assert!(second.text().contains("Newest entries"));
    assert!(!second.text().contains("Older entries"));
}
#[tokio::test]
async fn export_cap_and_truncated_filename_use_filtered_count() {
    let app = fixture().await;
    app.db().write(|tx| {
        tx.conn().execute("WITH RECURSIVE n(i) AS (VALUES(1) UNION ALL SELECT i+1 FROM n WHERE i<5001) INSERT INTO audit_logs(id,action,details,created_at,updated_at) SELECT i,'user.unban','{}',?,? FROM n",rusqlite::params![tx.now(),tx.now()])?;Ok(())
    }).await.unwrap();
    let mut browser = app.david();
    let first = browser
        .get("/account/audit_log?audit_action=user.unban")
        .await;
    assert!(first.text().contains("newest 5,000 matching rows"));
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
