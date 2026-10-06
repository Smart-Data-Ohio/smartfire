use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use tower::ServiceExt;

use crate::{
    app::App,
    config::Config,
};

struct Fresh {
    app: App,
    router: axum::Router,
    _dir: tempfile::TempDir,
}

impl Fresh {
    async fn new(secret: Option<&str>) -> Self {
        let scratch =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.scratch/ws15g");
        std::fs::create_dir_all(&scratch).unwrap();
        let dir = tempfile::tempdir_in(scratch).unwrap();
        let config = Config::from_lookup(|name| match name {
            "SECRET_KEY_BASE" => Some("a".repeat(128)),
            "GITHUB_WEBHOOK_SECRET" => secret.map(str::to_owned),
            "DISABLE_SSL" => Some("1".into()),
            "CAMPFIRE_STORAGE_PATH" => Some(dir.path().to_string_lossy().into_owned()),
            _ => None,
        })
        .unwrap();
        let booted = crate::server::boot(config).await.unwrap();
        let crate::server::Booted { app, router, jobs, .. } = booted;
        jobs.shutdown(std::time::Duration::from_secs(1)).await;
        Self {
            app,
            router,
            _dir: dir,
        }
    }

    async fn post(
        &self,
        body: &str,
        signature: Option<&str>,
        guid: Option<&str>,
        event: Option<&str>,
    ) -> (StatusCode, String) {
        self.send("POST", body, signature, guid, event).await
    }

    async fn send(
        &self,
        method: &str,
        body: &str,
        signature: Option<&str>,
        guid: Option<&str>,
        event: Option<&str>,
    ) -> (StatusCode, String) {
        let mut request = Request::builder()
            .method(method)
            .uri("/github/webhooks")
            .header("Host", "campfire.test")
            .header("Content-Type", "application/json");
        for (name, value) in [
            ("X-Hub-Signature-256", signature),
            ("X-GitHub-Delivery", guid),
            ("X-GitHub-Event", event),
        ] {
            if let Some(value) = value {
                request = request.header(name, value);
            }
        }
        let response = self
            .router
            .clone()
            .oneshot(request.body(Body::from(body.to_owned())).unwrap())
            .await
            .unwrap();
        assert!(
            !response.headers().contains_key("set-cookie"),
            "the API never creates a session"
        );
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        (status, String::from_utf8(body.to_vec()).unwrap())
    }
}

fn signature(secret: &str, body: &str) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(body.as_bytes());
    format!("sha256={:x}", mac.finalize().into_bytes())
}

#[tokio::test]
async fn webhook_security_missing_or_blank_secret_is_unavailable() {
    for secret in [None, Some(""), Some(" \t")] {
        let app = Fresh::new(secret).await;
        assert_eq!(
            app.post("{bad", None, None, None).await,
            (StatusCode::SERVICE_UNAVAILABLE, String::new())
        );
    }
}

#[tokio::test]
async fn webhook_security_rejects_bad_or_missing_signatures_before_parsing() {
    let app = Fresh::new(Some("fixture-webhook-secret")).await;
    for signed in [None, Some("sha256=deadbeef"), Some("sha1=wrong")] {
        assert_eq!(
            app.post("{bad", signed, Some("delivery"), Some("pull_request"))
                .await,
            (StatusCode::UNAUTHORIZED, String::new())
        );
    }
}

#[tokio::test]
async fn webhook_security_authenticates_exact_raw_bytes_and_lowercase_prefix() {
    let app = Fresh::new(Some("fixture-webhook-secret")).await;
    let signed = signature("fixture-webhook-secret", "{bad");
    for bad in [
        signature("fixture-webhook-secret", "{bad "),
        signed.to_uppercase(),
        format!("{signed} "),
    ] {
        assert_eq!(
            app.post("{bad", Some(&bad), Some("delivery"), Some("pull_request"))
                .await,
            (StatusCode::UNAUTHORIZED, String::new())
        );
    }
    assert_eq!(
        app.post(
            "{bad",
            Some(&signed),
            Some("delivery"),
            Some("pull_request")
        )
        .await,
        (StatusCode::OK, String::new())
    );
}

#[tokio::test]
async fn webhook_security_requires_a_nonblank_delivery_guid() {
    let app = Fresh::new(Some("fixture-webhook-secret")).await;
    let signed = signature("fixture-webhook-secret", "{}");
    for guid in [None, Some(""), Some(" \t")] {
        assert_eq!(
            app.post("{}", Some(&signed), guid, Some("ping")).await,
            (StatusCode::UNAUTHORIZED, String::new())
        );
    }
}

impl Fresh {
    async fn seed(&self, subscribed: bool) {
        self.app.db.write(move |tx| {
            campfire_db::fixtures::load(tx.conn(), &campfire_db::fixtures::reference_dir(), &campfire_db::fixtures::Options { now: tx.now(), bcrypt_cost: 4 })?;
            let now = tx.now();
            for id in [123, 124] {
                tx.conn().execute("INSERT INTO github_pull_requests (id, owner, repo, number, head_branch, private, fetched_at, created_at, updated_at) VALUES (?, 'rails', 'rails', ?, 'shiny', 0, ?, ?, ?)", rusqlite::params![id, id, now, now, now])?;
            }
            tx.conn().execute("INSERT INTO github_pull_request_references (github_pull_request_id, message_id, created_at, updated_at) VALUES (123, ?, ?, ?)", rusqlite::params![campfire_db::fixtures::identify("first"), now, now])?;
            if subscribed {
                tx.conn().execute("INSERT INTO github_repository_subscriptions (room_id, owner, repo, created_at, updated_at) VALUES (?, 'rails', 'rails', ?, ?)", rusqlite::params![campfire_db::fixtures::identify("designers"), now, now])?;
            }
            crate::integrations::github::tests::enqueue_retention_job(tx)
        }).await.unwrap();
    }

    async fn counts(&self) -> (i64, Vec<String>, Option<bool>) {
        self.app
            .db
            .read(|conn| {
                let deliveries = conn.query_row(
                    "SELECT COUNT(*) FROM github_webhook_deliveries",
                    [],
                    |row| row.get(0),
                )?;
                // Startup can leave unrelated recurring jobs queued before shutdown.
                let mut statement =
                    conn.prepare("SELECT job_class FROM background_jobs WHERE job_class GLOB 'Github::*' ORDER BY id")?;
                let jobs = statement
                    .query_map([], |row| row.get(0))?
                    .collect::<rusqlite::Result<Vec<String>>>()?;
                let private = conn.query_row(
                    "SELECT private FROM github_pull_requests WHERE id = 123",
                    [],
                    |row| row.get(0),
                )?;
                Ok((deliveries, jobs, private))
            })
            .await
            .unwrap()
    }
}

#[tokio::test]
async fn webhook_http_status_body_selection_and_privacy_match_rails() {
    let vectors: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../vectors/github_webhooks.json"
    ))
    .unwrap();
    for case in vectors["cases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let secret = case["secret"].as_str();
        let app = Fresh::new(secret).await;
        app.seed(case["subscribed"].as_bool().unwrap()).await;
        let guid = case["guid"].as_str();
        if case["duplicate"] == true {
            let guid = guid.unwrap().to_owned();
            app.app
                .db
                .write(move |tx| super::webhooks::claim(tx, &guid, "first"))
                .await
                .unwrap();
        }
        let raw = case["body"].as_str().unwrap();
        let signed = signature(secret.unwrap_or("fixture-webhook-secret"), raw);
        let signed = match case["signature"].as_str().unwrap() {
            "missing" => None,
            "bad" => Some("sha256=deadbeef".into()),
            "other_body" => Some(signature(secret.unwrap(), &format!("{raw} "))),
            "uppercase" => Some(signed.to_uppercase()),
            "prefix" => Some(signed.replace("sha256=", "sha1=")),
            "space" => Some(format!("{signed} ")),
            _ => Some(signed),
        };
        let expected = &case["expected"];
        let unchanged_before = app.unchanged_counts().await;
        assert_eq!(
            app.send(
                case["method"].as_str().unwrap(),
                raw,
                signed.as_deref(),
                guid,
                case["event"].as_str()
            )
            .await,
            (
                StatusCode::from_u16(expected["status"].as_u64().unwrap() as u16).unwrap(),
                expected["body"].as_str().unwrap().to_owned()
            ),
            "{name}"
        );
        assert_eq!(
            app.unchanged_counts().await,
            unchanged_before,
            "{name}: no PR, user or notification is created by ingestion"
        );
        if expected["status"] == 500 {
            assert_eq!(
                app.post(raw, signed.as_deref(), guid, case["event"].as_str())
                    .await,
                (StatusCode::OK, String::new()),
                "{name}: the first failed shape retains its delivery claim"
            );
        }
        let (deliveries, jobs, private) = app.counts().await;
        assert_eq!(
            deliveries,
            expected["deliveries"].as_i64().unwrap(),
            "{name}"
        );
        assert_eq!(
            serde_json::to_value(jobs).unwrap(),
            expected["jobs"],
            "{name}"
        );
        assert_eq!(
            serde_json::to_value(private).unwrap(),
            expected["private"],
            "{name}"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn webhook_concurrent_duplicates_claim_and_enqueue_once() {
    let app = Fresh::new(Some("fixture-webhook-secret")).await;
    app.seed(true).await;
    let raw = r#"{"repository":{"full_name":"Rails/Rails","private":true},"pull_request":{"number":123}}"#;
    let signed = signature("fixture-webhook-secret", raw);
    let responses = futures_util::future::join_all((0..24).map(|_| {
        app.post(
            raw,
            Some(&signed),
            Some("shared-guid"),
            Some("pull_request"),
        )
    }))
    .await;
    assert!(
        responses
            .iter()
            .all(|response| response == &(StatusCode::OK, String::new()))
    );
    assert_eq!(
        app.counts().await,
        (
            1,
            vec![
                "Github::FetchPullRequestJob".into(),
                "Github::DeliverSubscriptionEventJob".into()
            ],
            Some(true)
        )
    );
    // A claimed id skips even a signed shape that would fail on its first delivery.
    let bad = "null";
    let signed = signature("fixture-webhook-secret", bad);
    assert_eq!(
        app.post(
            bad,
            Some(&signed),
            Some("shared-guid"),
            Some("pull_request")
        )
        .await,
        (StatusCode::OK, String::new())
    );
    assert_eq!(app.counts().await.1.len(), 2);
}

#[tokio::test]
async fn webhook_claims_prune_strictly_older_rows_only_on_a_winning_claim() {
    use campfire_db::Timestamp;
    let app = Fresh::new(Some("fixture-webhook-secret")).await;
    let fixed = app.app.db.env().now();
    app.app.db.write(move |tx| {
        let boundary = fixed.since(jiff::SignedDuration::from_hours(-7 * 24));
        let older = Timestamp::from_jiff(boundary.jiff().checked_sub(jiff::SignedDuration::from_nanos(1000)).unwrap());
        for (guid, time) in [("older", older), ("boundary", boundary), ("existing", fixed)] {
            tx.conn().execute("INSERT INTO github_webhook_deliveries (delivery_guid, created_at, updated_at) VALUES (?, ?, ?)", rusqlite::params![guid, time, time])?;
        }
        assert!(!super::webhooks::claim(tx, "existing", "second")?);
        assert_eq!(tx.conn().query_row("SELECT COUNT(*) FROM github_webhook_deliveries", [], |row| row.get::<_, i64>(0))?, 3);
        Ok(())
    }).await.unwrap();
    // Freeze the write's clock through an independent domain fixture below; the HTTP app clock
    // advances, so its retention boundary cannot be used for an exact equality assertion.
    let clock = std::sync::Arc::new(campfire_db::TestClock::frozen_at(fixed));
    let scratch = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.scratch/ws15g");
    let domain = tokio::task::spawn_blocking(move || {
        crate::integrations::test_support::TestDb::in_dir(clock, &scratch)
    })
    .await
    .unwrap();
    domain.db.write(move |tx| {
        let boundary = fixed.since(jiff::SignedDuration::from_hours(-7 * 24));
        for (guid, time) in [("older", boundary.since(jiff::SignedDuration::from_nanos(-1000))), ("boundary", boundary)] {
            tx.conn().execute("INSERT INTO github_webhook_deliveries (delivery_guid, created_at, updated_at) VALUES (?, ?, ?)", rusqlite::params![guid, time, time])?;
        }
        assert!(super::webhooks::claim(tx, "new", "ping")?);
        assert_eq!(tx.conn().query_row("SELECT COUNT(*) FROM github_webhook_deliveries", [], |row| row.get::<_, i64>(0))?, 2);
        Ok(())
    }).await.unwrap();
}

#[tokio::test]
async fn webhook_durable_queue_failure_rolls_back_claim_and_privacy() {
    let app = Fresh::new(Some("fixture-webhook-secret")).await;
    app.seed(true).await;
    app.app.db.write(|tx| {
        tx.conn().execute_batch("CREATE TRIGGER reject_github_job BEFORE INSERT ON background_jobs BEGIN SELECT RAISE(ABORT, 'fixture queue unavailable'); END")?;
        Ok(())
    }).await.unwrap();
    let raw = r#"{"repository":{"full_name":"rails/rails","private":true},"pull_request":{"number":123}}"#;
    let signed = signature("fixture-webhook-secret", raw);
    assert_eq!(
        app.post(
            raw,
            Some(&signed),
            Some("retry-after-rollback"),
            Some("pull_request")
        )
        .await
        .0,
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert_eq!(app.counts().await, (0, Vec::new(), Some(false)));
    app.app
        .db
        .write(|tx| {
            tx.conn().execute_batch("DROP TRIGGER reject_github_job")?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(
        app.post(
            raw,
            Some(&signed),
            Some("retry-after-rollback"),
            Some("pull_request")
        )
        .await,
        (StatusCode::OK, String::new())
    );
    assert_eq!(app.counts().await.1.len(), 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn webhook_duplicates_across_independent_database_writers_enqueue_once() {
    let app = Fresh::new(Some("fixture-webhook-secret")).await;
    app.seed(true).await;
    let config = campfire_db::Config::new(&app.app.config.storage.database);
    let env = app.app.db.env().clone();
    let other = tokio::task::spawn_blocking(move || campfire_db::Database::open(config, env))
        .await
        .unwrap()
        .unwrap();
    let raw = br#"{"repository":{"full_name":"rails/rails"},"pull_request":{"number":123}}"#;
    let deliveries = futures_util::future::join_all((0..24).map(|index| {
        let db = if index % 2 == 0 { &app.app.db } else { &other };
        super::webhooks::receive(
            db,
            "independent-writers".into(),
            "pull_request".into(),
            raw.to_vec(),
        )
    }))
    .await;
    assert!(deliveries.iter().all(Result::is_ok));
    assert_eq!(
        app.counts().await,
        (
            1,
            vec![
                "Github::FetchPullRequestJob".into(),
                "Github::DeliverSubscriptionEventJob".into()
            ],
            Some(false)
        )
    );
}

impl Fresh {
    async fn unchanged_counts(&self) -> (i64, i64, i64) {
        self.app
            .db
            .read(|conn| {
                Ok((
                    conn.query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0))?,
                    conn.query_row("SELECT COUNT(*) FROM github_pull_requests", [], |r| {
                        r.get(0)
                    })?,
                    conn.query_row("SELECT COUNT(*) FROM github_notifications", [], |r| {
                        r.get(0)
                    })?,
                ))
            })
            .await
            .unwrap()
    }
}

#[tokio::test]
async fn webhook_redelivered_supported_events_do_not_enqueue_again() {
    let vectors: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../vectors/github_webhooks.json"
    ))
    .unwrap();
    for name in [
        "valid",
        "review",
        "issue_comment",
        "check_suite",
        "check_run",
        "status",
    ] {
        let case = vectors["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|case| case["name"] == name)
            .unwrap();
        let app = Fresh::new(Some("fixture-webhook-secret")).await;
        app.seed(true).await;
        let raw = case["body"].as_str().unwrap();
        let signed = signature("fixture-webhook-secret", raw);
        let event = case["event"].as_str();
        assert_eq!(
            app.post(raw, Some(&signed), Some("redelivered"), event)
                .await,
            (StatusCode::OK, String::new()),
            "{name}"
        );
        let first = app.counts().await;
        assert_eq!(
            first.1.len(),
            if name == "issue_comment" { 1 } else { 2 },
            "{name}"
        );
        assert_eq!(
            app.post(raw, Some(&signed), Some("redelivered"), event)
                .await,
            (StatusCode::OK, String::new()),
            "{name}"
        );
        assert_eq!(app.counts().await, first, "{name}");
        // Authentication still precedes dedupe: a known id cannot acknowledge unsigned traffic.
        assert_eq!(
            app.post(raw, None, Some("redelivered"), event).await,
            (StatusCode::UNAUTHORIZED, String::new()),
            "{name}"
        );
        assert_eq!(app.counts().await, first, "{name}");
        let expected_event = case["event"].clone();
        let expected_payload: serde_json::Value = serde_json::from_str(raw).unwrap();
        app.app
            .db
            .read(move |conn| {
                let mut statement =
                    conn.prepare("SELECT job_class, arguments FROM background_jobs WHERE job_class GLOB 'Github::*' ORDER BY id")?;
                let jobs = statement
                    .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
                    .collect::<rusqlite::Result<Vec<_>>>()?;
                assert_eq!(
                    serde_json::from_str::<serde_json::Value>(&jobs[0].1).unwrap(),
                    serde_json::json!({ "pull_request_id": 123 })
                );
                if jobs.len() == 2 {
                    let args: serde_json::Value = serde_json::from_str(&jobs[1].1).unwrap();
                    assert_eq!(args["event"], expected_event);
                    assert_eq!(args["payload"], expected_payload);
                }
                Ok(())
            })
            .await
            .unwrap();
    }
}
