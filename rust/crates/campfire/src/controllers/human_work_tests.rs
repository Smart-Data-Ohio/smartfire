//! Compare complete human work responses, including permission denials, against real Rails.
use super::presenters::test_support::*;
use axum::http::Method;
use serde_json::Value;

#[tokio::test]
async fn human_links_save_no_activity_and_claim_one_real_pr_fetch_across_threads() {
    let oracle: Value =
        serde_json::from_str(include_str!("../../../../vectors/human_work_http.json")).unwrap();
    let row = oracle["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["name"] == "link-pr")
        .unwrap();
    let app = TestApp::boot_frozen()
        .await
        .expect("default seed required")
        .without_job_runner()
        .await;
    setup(&app, row).await;
    let before = app
        .db()
        .read(|conn| {
            Ok(
                conn.query_row("SELECT COUNT(*) FROM activity_items", [], |r| {
                    r.get::<_, i64>(0)
                })?,
            )
        })
        .await
        .unwrap();
    let mut browser = app.david();
    for thread in [90, 91] {
        let response = browser
            .write(
                Req::new(
                    Method::POST,
                    &format!("/threads/{thread}/work/links.turbo_stream"),
                )
                .header("content-type", "application/json")
                .body(row["input"].to_string()),
            )
            .await;
        assert_eq!(response.status, 200, "{}", response.text());
    }
    let id=app.db().read(move |conn| {
        let id:i64=conn.query_row("SELECT id FROM github_pull_requests WHERE owner='rails' AND repo='rails' AND number=12",[],|r|r.get(0))?;
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM work_thread_links WHERE github_pull_request_id=?",[id],|r|r.get::<_,i64>(0))?,2);
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='Github::FetchPullRequestJob' AND json_extract(arguments,'$.pull_request_id')=?",[id],|r|r.get::<_,i64>(0))?,1);
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM activity_items",[],|r|r.get::<_,i64>(0))?,before);
        Ok(id)
    }).await.unwrap();
    let link = app
        .db()
        .read(|conn| {
            Ok(campfire_db::WorkThreadLink::for_thread(conn, 90)?
                .into_iter()
                .find(|link| link.github_pull_request_id.is_some())
                .unwrap()
                .id)
        })
        .await
        .unwrap();
    let response = browser
        .write(Req::new(
            Method::DELETE,
            &format!("/threads/90/work/links/{link}.turbo_stream"),
        ))
        .await;
    assert_eq!(response.status, 200);
    app.db()
        .read(move |conn| {
            assert!(campfire_db::WorkThreadLink::find(conn, link)?.is_none());
            assert_eq!(
                crate::integrations::github::pull_requests::PullRequest::find(conn, id)?.id,
                id
            );
            assert_eq!(
                conn.query_row("SELECT COUNT(*) FROM activity_items", [], |r| r
                    .get::<_, i64>(0))?,
                before
            );
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn human_drive_title_adapter_uses_the_linkers_identity_and_falls_back_on_error() {
    use super::work_threads::links::DriveTitle;
    use futures_util::future::BoxFuture;
    use std::sync::Arc;
    struct Title(bool);
    impl DriveTitle for Title {
        fn title<'a>(
            &'a self,
            user_id: i64,
            file_id: &'a str,
        ) -> BoxFuture<'a, campfire_db::Result<Option<String>>> {
            Box::pin(async move {
                assert_eq!(user_id, DAVID);
                assert_eq!(file_id, "1AbcDefGhIjKlMnOpQrSt");
                if self.0 {
                    Ok(Some("Plan <&> title".into()))
                } else {
                    Err(campfire_db::Error::Other("Google request failed".into()))
                }
            })
        }
    }
    let oracle: Value =
        serde_json::from_str(include_str!("../../../../vectors/human_work_http.json")).unwrap();
    let row = oracle["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["name"] == "link-drive")
        .unwrap();
    for success in [true, false] {
        let app = TestApp::boot_frozen()
            .await
            .expect("default seed required")
            .without_job_runner()
            .await;
        setup(&app, row).await;
        app.booted
            .app
            .work_link_drive_titles
            .install(Arc::new(Title(success)));
        let response = app
            .david()
            .write(
                Req::new(Method::POST, row["path"].as_str().unwrap())
                    .header("content-type", "application/json")
                    .body(row["input"].to_string()),
            )
            .await;
        assert_eq!(response.status, 200, "{}", response.text());
        if success {
            assert!(response.text().contains("Plan &lt;&amp;&gt; title"));
        }
        let title = app
            .db()
            .read(|conn| {
                Ok(campfire_db::WorkThreadLink::for_thread(conn, 90)?
                    .last()
                    .unwrap()
                    .title
                    .clone())
            })
            .await
            .unwrap();
        assert_eq!(title, success.then(|| "Plan <&> title".into()));
    }
}

async fn setup(app: &TestApp, row: &Value) {
    let setup = row["setup"]
        .as_array()
        .unwrap()
        .iter()
        .map(|sql| sql.as_str().unwrap().to_string())
        .collect::<Vec<_>>();
    app.db()
        .write(move |tx| {
            for sql in setup {
                tx.conn().execute_batch(&sql)?;
            }
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn human_handoff_authorization_precedes_receiver_validation_and_creates_nothing() {
    let oracle: Value =
        serde_json::from_str(include_str!("../../../../vectors/human_work_http.json")).unwrap();
    for name in [
        "handoff-plain-member",
        "handoff-nonmember",
        "handoff-untracked",
        "handoff-create-forbidden",
        "handoff-create-hidden",
        "handoff-receiver-read",
    ] {
        let row = oracle["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["name"] == name)
            .unwrap();
        let app = TestApp::boot_frozen()
            .await
            .expect("default seed required")
            .without_job_runner()
            .await;
        setup(&app, row).await;
        let mut browser = app.sign_in(row["user_id"].as_i64().unwrap()).await;
        let response = if row["method"] == "get" {
            browser.get(row["path"].as_str().unwrap()).await
        } else {
            browser
                .write(
                    Req::new(Method::POST, row["path"].as_str().unwrap())
                        .header("content-type", "application/json")
                        .body(row["input"].to_string()),
                )
                .await
        };
        assert_eq!(
            response.status.as_u16(),
            row["status"].as_u64().unwrap() as u16,
            "{name}"
        );
        app.db()
            .read(|conn| {
                for (table, clause) in [
                    ("work_handoffs", "channel_thread_id=90"),
                    ("work_thread_events", "channel_thread_id=90"),
                    ("audit_logs", "action='work.handoff' AND target_id=90"),
                    ("agent_events", "json_extract(metadata,'$.thread_id')=90"),
                ] {
                    assert_eq!(
                        conn.query_row(
                            &format!("SELECT COUNT(*) FROM {table} WHERE {clause}"),
                            [],
                            |row| row.get::<_, i64>(0)
                        )?,
                        0,
                        "{table}"
                    );
                }
                Ok(())
            })
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn human_handoff_http_commits_history_audit_ledger_and_job_together() {
    let oracle: Value =
        serde_json::from_str(include_str!("../../../../vectors/human_work_http.json")).unwrap();
    let row = oracle["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["name"] == "handoff-create-json")
        .unwrap();
    let app = TestApp::boot_frozen()
        .await
        .expect("default seed required")
        .without_job_runner()
        .await;
    setup(&app, row).await;
    app.db().write(|tx| {
        tx.conn().execute_batch("CREATE TRIGGER reject_human_handoff_job BEFORE INSERT ON background_jobs WHEN NEW.job_class='Agent::EventWebhookJob' AND EXISTS(SELECT 1 FROM agent_events WHERE id=json_extract(NEW.arguments,'$.event_id') AND event_type='work_handed_off') BEGIN SELECT RAISE(ABORT,'rejected human handoff job'); END")?;
        Ok(())
    }).await.unwrap();
    let mut browser = app.david();
    let mut package = row["input"].clone();
    package["summary"] = serde_json::json!("<script>alert(1)</script>");
    let request = || {
        Req::new(Method::POST, "/threads/90/work/handoff.json")
            .header("content-type", "application/json")
            .body(package.to_string())
    };
    assert_eq!(browser.write(request()).await.status, 500);
    app.db()
        .read(|conn| {
            assert_eq!(
                campfire_db::ChannelThread::find(conn, 90)?.work_owner_id,
                Some(DAVID)
            );
            for (table, clause) in [
                ("work_handoffs", "channel_thread_id=90"),
                ("work_thread_events", "channel_thread_id=90"),
                ("audit_logs", "action='work.handoff' AND target_id=90"),
                ("agent_events", "json_extract(metadata,'$.thread_id')=90"),
            ] {
                assert_eq!(
                    conn.query_row(
                        &format!("SELECT COUNT(*) FROM {table} WHERE {clause}"),
                        [],
                        |row| row.get::<_, i64>(0)
                    )?,
                    0,
                    "{table}"
                );
            }
            Ok(())
        })
        .await
        .unwrap();
    app.db()
        .write(|tx| {
            tx.conn()
                .execute_batch("DROP TRIGGER reject_human_handoff_job")?;
            Ok(())
        })
        .await
        .unwrap();
    let response = browser.write(request()).await;
    assert_eq!(response.status, 201, "{}", response.text());
    let history = browser.get("/rooms/699448332/threads/90").await;
    assert_eq!(history.status, 200);
    assert!(history.text().contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
    assert!(!history.text().contains("<script>alert(1)</script>"));
    app.db().read(|conn| {
        assert_eq!(campfire_db::ChannelThread::find(conn,90)?.work_owner_id,Some(BENDER));
        for (table,clause) in [("work_handoffs","channel_thread_id=90"),("work_thread_events","channel_thread_id=90"),("audit_logs","action='work.handoff' AND target_id=90"),("agent_events","event_type='work_handed_off' AND json_extract(metadata,'$.thread_id')=90"),("background_jobs","job_class='Agent::EventWebhookJob' AND json_extract(arguments,'$.event_id') IN (SELECT id FROM agent_events WHERE event_type='work_handed_off' AND json_extract(metadata,'$.thread_id')=90)")] {
            assert_eq!(conn.query_row(&format!("SELECT COUNT(*) FROM {table} WHERE {clause}"),[],|row|row.get::<_,i64>(0))?,1,"{table}");
        }
        Ok(())
    }).await.unwrap();
}

#[tokio::test]
async fn human_work_http_matches_complete_rails_responses() {
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
    let oracle: Value =
        serde_json::from_str(include_str!("../../../../vectors/human_work_http.json")).unwrap();
    let mut mismatches = Vec::new();
    for row in oracle["rows"].as_array().unwrap() {
        let app = TestApp::boot_frozen_with_env(&vapid)
            .await
            .expect("default seed required")
            .without_job_runner()
            .await;
        let setup = row["setup"]
            .as_array()
            .unwrap()
            .iter()
            .map(|sql| sql.as_str().unwrap().to_string())
            .collect::<Vec<_>>();
        app.db()
            .write(move |tx| {
                for sql in setup {
                    tx.conn().execute_batch(&sql)?;
                }
                Ok(())
            })
            .await
            .unwrap();
        let mut browser = app.sign_in(row["user_id"].as_i64().unwrap()).await;
        let method = row["method"]
            .as_str()
            .unwrap()
            .to_ascii_uppercase()
            .parse::<Method>()
            .unwrap();
        let mut request =
            Req::new(method.clone(), row["path"].as_str().unwrap()).header("user-agent", "Mozilla");
        for (key, value) in row["headers"].as_object().unwrap() {
            request = request.header(key, value.as_str().unwrap());
        }
        if method != Method::GET {
            request = request
                .header("content-type", "application/json")
                .body(row["input"].to_string());
        }
        let response = with_fixed_render_secrets(async {
            if method == Method::GET {
                browser.send(request).await
            } else {
                browser.write(request).await
            }
        })
        .await;
        let name = row["name"].as_str().unwrap();
        assert_eq!(
            response.status.as_u16(),
            row["status"].as_u64().unwrap() as u16,
            "{name}: {}",
            response.text()
        );
        assert_eq!(response.location(), row["location"].as_str(), "{name}");
        assert_eq!(
            response.header("cache-control"),
            row["cache_control"].as_str(),
            "{name}"
        );
        let actual = response.text();
        let expected = row["body"].as_str().unwrap();
        if actual != expected {
            let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../.scratch/human-work/diffs");
            std::fs::create_dir_all(&directory).unwrap();
            std::fs::write(directory.join(format!("{name}-actual.html")), actual).unwrap();
            std::fs::write(directory.join(format!("{name}-expected.html")), expected).unwrap();
            mismatches.push(name.to_string());
        }
    }
    assert!(
        mismatches.is_empty(),
        "Rails full response mismatches: {mismatches:?}"
    );
}
