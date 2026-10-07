use super::*;
use campfire_db::Database;
use crate::integrations::{
    github::{client::ReadClient, jobs::FetchPullRequestJob, tests::fake},
    test_support::{Route, TestDb},
};
use campfire_db::{Connection, Event, TestClock, Timestamp};
use serde_json::{Value, json};
use std::sync::Arc;

fn cases() -> Value {
    serde_json::from_str(include_str!("../../../../../../vectors/github_fetch.json")).unwrap()
}
fn routes(case: &Value) -> Vec<Route> {
    [
        ("pr", "pr_status", "/repos/rails/rails/pulls/123"),
        (
            "reviews",
            "review_status",
            "/repos/rails/rails/pulls/123/reviews?per_page=100",
        ),
        (
            "runs",
            "run_status",
            "/repos/rails/rails/commits/abc/check-runs?per_page=100",
        ),
        (
            "status",
            "status_status",
            "/repos/rails/rails/commits/abc/status",
        ),
        (
            "files",
            "file_status",
            "/repos/rails/rails/pulls/123/files?per_page=100",
        ),
    ]
    .into_iter()
    .map(|(key, status, path)| {
        let body = case
            .get(format!("{key}_raw"))
            .and_then(Value::as_str)
            .map(str::to_owned)
            .unwrap_or_else(|| case[key].to_string());
        let mut route = Route::new(
            "GET",
            "api.github.com",
            path,
            case[status].as_u64().unwrap_or(200) as u16,
        )
        .body(body);
        if key == "pr"
            && let Some(headers) = case["headers"].as_object()
        {
            for (key, value) in headers {
                route = route.header(key, value.as_str().unwrap());
            }
        }
        route
    })
    .collect()
}
async fn database() -> TestDb {
    let scratch = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.scratch/ws15g");
    let clock = Arc::new(TestClock::frozen_at(Timestamp::from_jiff(
        "2026-01-01T12:00:00Z".parse().unwrap(),
    )));
    tokio::task::spawn_blocking(move || TestDb::in_dir(clock, &scratch))
        .await
        .unwrap()
}
async fn seed(db: &Database, mapped: bool) {
    db.write(move |tx| {
        let old = tx.now().ago(jiff::SignedDuration::from_hours(1));
        tx.conn().execute("INSERT INTO github_pull_requests (id, owner, repo, number, title, private, author_login, state, base_branch, head_branch, head_sha, review_decision, check_status, payload, fetched_at, fetch_error, changed_files, changed_files_fetched_at, created_at, updated_at) VALUES (123, 'rails', 'rails', 123, 'Previous title', 1, 'previous', 'closed', 'old', 'old', 'old', 'old', 'old', ?, ?, 'Previous error', ?, ?, ?, ?)", rusqlite::params![json!({"previous":true}).to_string(),old,json!({"files":[{"filename":"old.rs","additions":1,"deletions":0,"status":"added"}],"total_count":1}).to_string(),old,old,old])?;
        if mapped {
            tx.conn().execute("INSERT INTO channel_threads (id, room_id, creator_id, name, last_activity_at, created_at, updated_at) VALUES (9991, ?, ?, 'Discussion', ?, ?, ?)", rusqlite::params![TestDb::id("designers"),TestDb::id("david"),tx.now(),tx.now(),tx.now()])?;
            tx.conn().execute("INSERT INTO github_pull_request_threads (github_pull_request_id, room_id, channel_thread_id, created_at, updated_at) VALUES (123, ?, 9991, ?, ?)", rusqlite::params![TestDb::id("designers"),tx.now(),tx.now()])?;
        }
        Ok(())
    }).await.unwrap();
}
fn snapshot(conn: &Connection) -> campfire_db::Result<Value> {
    let fields = [
        "title",
        "private",
        "author_login",
        "author_avatar_url",
        "state",
        "base_branch",
        "head_branch",
        "head_sha",
        "html_url",
        "review_decision",
        "check_status",
        "payload",
        "fetch_error",
        "changed_files",
        "github_updated_at",
        "fetched_at",
        "changed_files_fetched_at",
        "updated_at",
    ];
    Ok(conn.query_row(
        &format!(
            "SELECT {} FROM github_pull_requests WHERE id = 123",
            fields.join(",")
        ),
        [],
        |row| {
            let mut result = serde_json::Map::new();
            for (i, key) in fields.into_iter().enumerate() {
                let value = if key == "private" {
                    json!(row.get::<_, Option<bool>>(i)?)
                } else {
                    let text = row.get::<_, Option<String>>(i)?;
                    if ["payload", "changed_files"].contains(&key) {
                        text.map_or(Value::Null, |s| serde_json::from_str(&s).unwrap())
                    } else {
                        json!(text)
                    }
                };
                result.insert(key.into(), value);
            }
            Ok(Value::Object(result))
        },
    )?)
}

#[tokio::test]
async fn github_fetch_persisted_fields_errors_reviews_checks_and_files_match_rails() {
    for case in cases()["cases"].as_array().unwrap() {
        let fixture = database().await;
        seed(&fixture.db, case["mapped"] == true).await;
        let (server, network) = fake(routes(case)).await;
        let token = if case.get("token").is_some() {
            case["token"].as_str().map(str::to_owned)
        } else {
            Some("fixture-workspace-token".into())
        };
        fetch(&fixture.db, &ReadClient::with_network(token, network), 123)
            .await
            .unwrap();
        assert_eq!(
            fixture.db.read(snapshot).await.unwrap(),
            case["expected"],
            "{}",
            case["name"]
        );
        let stored = fixture
            .db
            .read(|conn| {
                Ok(conn.query_row(
                    "SELECT changed_files FROM github_pull_requests WHERE id = 123",
                    [],
                    |row| row.get::<_, Option<String>>(0),
                )?)
            })
            .await
            .unwrap();
        assert_eq!(
            json!(stored),
            case["stored_files"],
            "{} exact stored files",
            case["name"]
        );
        let stored = fixture
            .db
            .read(|conn| {
                Ok(conn.query_row(
                    "SELECT payload FROM github_pull_requests WHERE id = 123",
                    [],
                    |row| row.get::<_, Option<String>>(0),
                )?)
            })
            .await
            .unwrap();
        assert_eq!(
            json!(stored),
            case["stored_payload"],
            "{} exact stored payload",
            case["name"]
        );
        let received = server.received();
        let expected = case["received"].as_array().unwrap();
        assert_eq!(
            received.len(),
            expected.len(),
            "{} request count",
            case["name"]
        );
        for (actual, expected) in received.iter().zip(expected) {
            let authorization = (expected["authorized"] == true).then(|| format!("Bearer {}", case.get("token").and_then(Value::as_str).unwrap_or("fixture-workspace-token")));
            assert_eq!(actual.header("Authorization"), authorization.as_deref());
            assert_eq!(
                actual.target,
                expected["path"].as_str().unwrap(),
                "{}",
                case["name"]
            );
            for header in [
                "Accept",
                "User-Agent",
                "X-GitHub-Api-Version",
            ] {
                assert_eq!(
                    actual.header(header),
                    expected["headers"][header].as_str(),
                    "{} {header}",
                    case["name"]
                );
            }
        }
    }
}

#[tokio::test]
async fn github_fetch_runtime_performs_discards_missing_and_does_not_retry_failures() {
    for mode in ["success", "missing", "persist_failure", "malformed"] {
        let case = &cases()["cases"][0];
        let (server, network) = fake(routes(case)).await;
        let scratch =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.scratch/ws15g");
        std::fs::create_dir_all(&scratch).unwrap();
        let directory = tempfile::tempdir_in(scratch).unwrap();
        let config = crate::config::Config::from_lookup(|name| match name {
            "SECRET_KEY_BASE" => Some("a".repeat(128)),
            "CAMPFIRE_STORAGE_PATH" => Some(directory.path().to_string_lossy().into_owned()),
            "DISABLE_SSL" => Some("1".into()),
            _ => None,
        })
        .unwrap();
        let clock = Arc::new(campfire_kit::clock::FrozenClock::new(
            "2026-01-01T12:00:00Z".parse().unwrap(),
        ));
        let booted = crate::server::boot_with_github_read(
            config,
            clock,
            ReadClient::with_network(Some("fixture-workspace-token".into()), network),
        )
        .await
        .unwrap();
        if mode != "missing" && mode != "malformed" {
            seed(&booted.app.db, false).await;
        }
        booted.app.db.write(move |tx| {
            if mode == "persist_failure" { tx.conn().execute_batch("CREATE TRIGGER reject_fetch BEFORE UPDATE ON github_pull_requests BEGIN SELECT RAISE(ABORT, 'fixture persistence failure'); END")?; }
            let mut request = campfire_db::JobRequest::new(&FetchPullRequestJob { pull_request_id: 123 });
            if mode == "malformed" { request.arguments = json!({"pull_request_id":"invalid"}); }
            tx.emit_after_commit(Event::Job(request));
            Ok(())
        }).await.unwrap();
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            let jobs = booted
                .app
                .db
                .read(campfire_jobs::inspect::all)
                .await
                .unwrap();
            if mode == "persist_failure" {
                if jobs.len() == 1 && jobs[0].status == campfire_jobs::FAILED {
                    assert_eq!(jobs[0].attempts, 1);
                    assert_eq!(jobs[0].run_at, jobs[0].created_at);
                    assert_eq!(jobs[0].class, "Github::FetchPullRequestJob");
                    assert!(
                        jobs[0]
                            .last_error
                            .as_deref()
                            .unwrap()
                            .contains("fixture persistence failure")
                    );
                    break;
                }
            } else if jobs.is_empty() {
                break;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "runtime {mode}: {jobs:?}"
            );
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        if mode == "success" {
            assert_eq!(
                booted.app.db.read(snapshot).await.unwrap(),
                case["expected"]
            );
        }
        if mode == "missing" || mode == "malformed" {
            assert!(server.received().is_empty());
        }
        booted
            .jobs
            .shutdown(std::time::Duration::from_secs(1))
            .await;
    }
}

#[test]
fn github_fetch_declares_one_attempt_and_inherited_missing_record_discard() {
    use campfire_jobs::{JobError, JobKind};
    let policy = FetchPullRequestJob::retry_policy();
    assert_eq!(policy.attempts, 1);
    assert_eq!(policy.retry_delay(1, None, 0.5), None);
    assert!(matches!(
        crate::queue::discard_missing(campfire_db::Error::RecordNotFound("Github::PullRequest")),
        JobError::Discard(_)
    ));
}

#[tokio::test]
async fn github_fetch_transport_failure_persists_error_without_changing_card_or_files() {
    use crate::integrations::{net::Network, test_support::FakeResolver};
    let fixture = database().await;
    seed(&fixture.db, false).await;
    let before = fixture.db.read(snapshot).await.unwrap();
    let mut network = Network::system();
    network.resolver = Arc::new(FakeResolver::default());
    fetch(
        &fixture.db,
        &ReadClient::with_network(Some("fixture-secret-workspace-token".into()), network),
        123,
    )
    .await
    .unwrap();
    let mut expected = before;
    expected["fetch_error"] = json!("Could not reach GitHub (Socket Error)");
    expected["fetched_at"] = json!("2026-01-01 12:00:00");
    expected["updated_at"] = json!("2026-01-01 12:00:00");
    assert_eq!(fixture.db.read(snapshot).await.unwrap(), expected);
}

#[tokio::test]
async fn github_fetch_checks_current_thread_mapping_after_http_without_holding_writer() {
    for initially_mapped in [true, false] {
        let fixture = database().await;
        seed(&fixture.db, initially_mapped).await;
        let case = &cases()["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|case| case["name"] == "mapped_files")
            .unwrap()
            .clone();
        let mut routes = routes(case);
        routes[0].delay = std::time::Duration::from_millis(300);
        let (server, network) = fake(routes).await;
        let db = fixture.db.clone();
        let worker =
            tokio::spawn(
                async move { fetch(&db, &ReadClient::with_network(None, network), 123).await },
            );
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(2);
        while server.received().is_empty() {
            assert!(tokio::time::Instant::now() < deadline);
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
        tokio::time::timeout(std::time::Duration::from_millis(150), fixture.db.write(move |tx| {
            if initially_mapped { tx.conn().execute("DELETE FROM github_pull_request_threads WHERE github_pull_request_id = 123", [])?; }
            else {
                tx.conn().execute("INSERT INTO channel_threads (id, room_id, creator_id, name, last_activity_at, created_at, updated_at) VALUES (9991, ?, ?, 'Discussion', ?, ?, ?)", rusqlite::params![TestDb::id("designers"),TestDb::id("david"),tx.now(),tx.now(),tx.now()])?;
                tx.conn().execute("INSERT INTO github_pull_request_threads (github_pull_request_id, room_id, channel_thread_id, created_at, updated_at) VALUES (123, ?, 9991, ?, ?)", rusqlite::params![TestDb::id("designers"),tx.now(),tx.now()])?;
            }
            Ok(())
        })).await.expect("HTTP must not hold the database writer").unwrap();
        worker.await.unwrap().unwrap();
        assert_eq!(
            server
                .received()
                .iter()
                .filter(|r| r.target.ends_with("/files?per_page=100"))
                .count(),
            usize::from(!initially_mapped)
        );
        let after = fixture.db.read(snapshot).await.unwrap();
        assert_eq!(
            after["changed_files"]["files"][0]["filename"],
            if initially_mapped { "old.rs" } else { "a.rs" }
        );
    }
}
