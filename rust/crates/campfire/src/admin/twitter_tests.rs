use super::*;
use crate::controllers::presenters::test_support::TestApp;
use serde_json::{Value, json};

fn oracle() -> Value {
    serde_json::from_str(include_str!("../../../../vectors/twitter_backfill.json")).unwrap()
}

fn insert(
    conn: &Connection,
    id: i64,
    markdown: Option<&str>,
    html: Option<&str>,
    note: Option<&str>,
) {
    conn.execute("INSERT INTO messages(id,room_id,creator_id,client_message_id,markdown_source,forward_note,created_at,updated_at) VALUES(?1,486777696,127326141,?2,?3,?4,'2026-03-02 16:00:00','2026-03-02 16:00:00')", rusqlite::params![id, format!("backfill-{id}"), markdown, note]).unwrap();
    if let Some(html) = html {
        conn.execute("INSERT INTO action_text_rich_texts(name,record_type,record_id,body,created_at,updated_at) VALUES('body','Message',?1,?2,'2026-03-02 16:00:00','2026-03-02 16:00:00')", rusqlite::params![id,html]).unwrap();
    }
}

async fn arranged() -> TestApp {
    let app = TestApp::boot_frozen()
        .await
        .unwrap()
        .without_job_runner()
        .await;
    let conn = Connection::open(app.db().path()).unwrap();
    conn.execute_batch("BEGIN IMMEDIATE").unwrap();
    for n in 0..oracle()["filler_count"].as_i64().unwrap() {
        insert(&conn, 2118000000 + n, Some("ordinary"), None, None);
    }
    for row in oracle()["inputs"].as_array().unwrap() {
        insert(
            &conn,
            row["id"].as_i64().unwrap(),
            row["markdown"].as_str(),
            row["html"].as_str(),
            row["forward_note"].as_str(),
        );
    }
    conn.execute_batch("COMMIT").unwrap();
    app
}

fn state(conn: &Connection) -> Value {
    let references = conn.prepare("SELECT r.message_id,p.post_id FROM twitter_post_references r JOIN twitter_posts p ON p.id=r.twitter_post_id ORDER BY r.message_id,p.post_id").unwrap().query_map([], |r| Ok(json!([r.get::<_,i64>(0)?, r.get::<_,String>(1)?]))).unwrap().collect::<rusqlite::Result<Vec<_>>>().unwrap();
    let posts = conn
        .prepare("SELECT post_id,url,fetch_requested_at FROM twitter_posts ORDER BY post_id")
        .unwrap()
        .query_map([], |r| {
            Ok(json!([
                r.get::<_, String>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, Option<campfire_db::Timestamp>>(2)?
                    .map(|at| format!("{:.6}", at.jiff()))
            ]))
        })
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();
    let jobs = conn.prepare("SELECT p.post_id FROM background_jobs j JOIN twitter_posts p ON p.id=json_extract(j.arguments,'$.post_id') WHERE j.job_class='Twitter::FetchPostJob' ORDER BY p.post_id").unwrap().query_map([], |r| r.get::<_,String>(0)).unwrap().collect::<rusqlite::Result<Vec<_>>>().unwrap();
    json!({"references":references,"posts":posts,"jobs":jobs})
}

#[tokio::test]
async fn twitter_backfill_matches_actual_rake_output_rows_and_jobs_across_batches() {
    let app = arranged().await;
    let conn = Connection::open(app.db().path()).unwrap();
    let env = app.db().env();
    assert_eq!(
        twitter_backfill_with_env(&conn, env).unwrap(),
        oracle()["output"].as_str().unwrap()
    );
    assert_eq!(state(&conn), oracle()["state"]);
    let before = state(&conn);
    assert_eq!(
        twitter_backfill_with_env(&conn, env).unwrap(),
        oracle()["repeated_output"].as_str().unwrap()
    );
    assert_eq!(state(&conn), before);
    let invalid: i64 = conn.query_row("SELECT count(*) FROM background_jobs WHERE job_class='Twitter::FetchPostJob' AND (queue_name<>'default' OR status<>'ready' OR payload_version<>1)",[],|r|r.get(0)).unwrap();
    assert_eq!(invalid, 0);
    let dangling: i64 = conn.query_row("SELECT count(*) FROM background_jobs j WHERE j.job_class='Twitter::FetchPostJob' AND NOT EXISTS (SELECT 1 FROM twitter_posts p WHERE p.id=json_extract(j.arguments,'$.post_id'))",[],|r|r.get(0)).unwrap();
    assert_eq!(dangling, 0);
}

#[tokio::test]
async fn twitter_backfill_rejected_fetch_rolls_back_its_sync_and_preserves_earlier_progress() {
    let app = arranged().await;
    let conn = Connection::open(app.db().path()).unwrap();
    conn.execute_batch("CREATE TRIGGER reject_backfill_fetch BEFORE INSERT ON background_jobs WHEN NEW.job_class='Twitter::FetchPostJob' AND json_extract(NEW.arguments,'$.post_id') IN (SELECT id FROM twitter_posts WHERE post_id='77105') BEGIN SELECT RAISE(ABORT,'injected fetch queue failure'); END;").unwrap();
    assert!(
        twitter_backfill_with_env(&conn, app.db().env())
            .unwrap_err()
            .to_string()
            .contains("injected fetch queue failure")
    );
    let rows = state(&conn);
    assert!(
        rows["references"]
            .as_array()
            .unwrap()
            .contains(&json!([-2, "77101"]))
    );
    assert!(
        rows["references"]
            .as_array()
            .unwrap()
            .contains(&json!([0, "77102"]))
    );
    assert!(
        !rows["posts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row[0] == "77105")
    );
    assert!(
        !rows["jobs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row == "77105")
    );
    conn.execute_batch("DROP TRIGGER reject_backfill_fetch")
        .unwrap();
    assert_eq!(
        twitter_backfill_with_env(&conn, app.db().env()).unwrap(),
        oracle()["output"].as_str().unwrap()
    );
    assert_eq!(state(&conn), oracle()["state"]);
}

#[test]
fn twitter_operator_refuses_missing_or_unknown_databases_and_pluralizes_one() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("production.sqlite3");
    let args = [
        "twitter-backfill-references".into(),
        path.to_str().unwrap().into(),
    ];
    assert_eq!(execute(&args).unwrap_err().0, 2);
    assert!(!path.exists());
    let mut conn = Connection::open(&path).unwrap();
    schema::prepare(&mut conn, "production", &campfire_db::SystemClock).unwrap();
    conn.execute_batch("INSERT INTO users(id,name,email_address,created_at,updated_at) VALUES(127326141,'Operator','operator@example.test','2026-03-02','2026-03-02'); INSERT INTO rooms(id,type,name,creator_id,created_at,updated_at) VALUES(486777696,'Rooms::Closed','Backfill',127326141,'2026-03-02','2026-03-02');").unwrap();
    insert(&conn, 0, Some("https://x.com/a/status/77101"), None, None);
    assert_eq!(execute(&args).unwrap(), "Backfilled 1 message\n");
    conn.execute("INSERT INTO schema_migrations VALUES('29990101000000')", [])
        .unwrap();
    assert_eq!(execute(&args).unwrap_err().0, 2);
    assert_eq!(
        execute(&["twitter-backfill-references".into()])
            .unwrap_err()
            .0,
        2
    );
}

fn rendered_oracle() -> Value {
    serde_json::from_str(include_str!(
        "../../../../vectors/twitter_backfill_rendered.json"
    ))
    .unwrap()
}

fn attachment_oracle() -> Value {
    serde_json::from_str(include_str!(
        "../../../../vectors/twitter_backfill_attachment.json"
    ))
    .unwrap()
}

#[tokio::test]
async fn twitter_backfill_saved_content_attachment_aborts_and_preserves_only_earlier_commits() {
    let vector = attachment_oracle();
    // Exercise both the reviewer's raw persisted HTML and the normal rich-text save output.
    for inputs in [&vector["inputs"], &vector["stored"]] {
        let app = arranged_rendered(&json!({"inputs": inputs})).await;
        let conn = Connection::open(app.db().path()).unwrap();
        for result in [&vector["first"], &vector["repeated"]] {
            let error = twitter_backfill_with_env(&conn, app.db().env()).unwrap_err();
            assert_eq!(
                error.to_string(),
                format!(
                    "{}: {}",
                    result["error"]["class"].as_str().unwrap(),
                    result["error"]["message"].as_str().unwrap()
                )
            );
            assert!(conn.is_autocommit());
            assert_eq!(
                state(&conn),
                result["state"],
                "earlier rows and jobs commit; failing/later rows are untouched"
            );
        }
    }
}

#[tokio::test]
async fn twitter_saved_content_attachment_default_and_request_renderers_and_sync_match_rails() {
    let vector = attachment_oracle();
    let app = arranged_rendered(&json!({"inputs": vector["stored"]})).await;
    let conn = Connection::open(app.db().path()).unwrap();
    let html = vector["stored"][1]["html"].as_str().unwrap();
    assert_eq!(
        app.db()
            .env()
            .rich_text
            .try_canonicalize_html(&conn, vector["inputs"][1]["html"].as_str().unwrap())
            .unwrap(),
        html
    );
    assert_eq!(
        app.db()
            .env()
            .rich_text
            .try_rendered_html(&conn, html)
            .unwrap_err(),
        format!(
            "{}: {}",
            vector["render_error"]["class"].as_str().unwrap(),
            vector["render_error"]["message"].as_str().unwrap()
        )
    );
    let secrets = rails_compat::Secrets::new("offline-render-probe");
    let resolver = crate::controllers::presenters::DbResolver::new(
        &conn,
        &secrets,
        jiff::Timestamp::from_second(1772467200).unwrap(),
    );
    let ctx = resolver.render_context(Some("campfire.test".into()));
    assert_eq!(
        campfire_richtext::Content::load(html, &ctx)
            .unwrap()
            .to_rendered_html_with_layout(&ctx)
            .unwrap(),
        vector["request_rendered"].as_str().unwrap()
    );
    let _ = twitter_backfill_with_env(&conn, app.db().env());
    for _ in 0..2 {
        campfire_db::run_write(&conn, app.db().env(), |tx| {
            let message = campfire_db::Message::find(tx.conn(), 2120000008)?;
            crate::integrations::twitter::references::sync_message(tx, &message, true)
        })
        .unwrap();
        assert_eq!(state(&conn), vector["sync"]);
    }
    conn.execute("UPDATE messages SET markdown_source='https://x.com/source/status/99115' WHERE id=2120000008", []).unwrap();
    assert_eq!(
        twitter_backfill_with_env(&conn, app.db().env()).unwrap(),
        vector["short_circuit"]["stdout"].as_str().unwrap()
    );
    assert_eq!(state(&conn), vector["short_circuit"]["state"]);
}

#[tokio::test]
async fn twitter_operator_content_attachment_uses_rake_failure_exit_and_diagnostic() {
    let vector = attachment_oracle();
    let app = arranged_rendered(&json!({"inputs": vector["stored"]})).await;
    let args = [
        "twitter-backfill-references".into(),
        app.db().path().to_str().unwrap().into(),
    ];
    let (status, error) = execute(&args).unwrap_err();
    assert_eq!(
        status,
        vector["cli"]["exit_status"].as_i64().unwrap() as i32
    );
    assert_eq!(
        format!("{error}\n"),
        vector["cli"]["stderr"].as_str().unwrap()
    );
}

async fn arranged_rendered(scenario: &Value) -> TestApp {
    let app = TestApp::boot_frozen()
        .await
        .unwrap()
        .without_job_runner()
        .await;
    let conn = Connection::open(app.db().path()).unwrap();
    for row in scenario["inputs"].as_array().unwrap() {
        insert(
            &conn,
            row["id"].as_i64().unwrap(),
            row["markdown"].as_str(),
            row["html"].as_str(),
            row["forward_note"].as_str(),
        );
    }
    app
}

#[tokio::test]
async fn twitter_backfill_review238_stripped_attribute_does_not_select_forward_note() {
    let vector = rendered_oracle();
    let case = &vector["single"];
    let app = arranged_rendered(case).await;
    let conn = Connection::open(app.db().path()).unwrap();
    assert_eq!(
        twitter_backfill_with_env(&conn, app.db().env()).unwrap(),
        case["output"].as_str().unwrap()
    );
    assert_eq!(state(&conn), case["state"]);
    let before = state(&conn);
    assert_eq!(
        twitter_backfill_with_env(&conn, app.db().env()).unwrap(),
        case["repeated_output"].as_str().unwrap()
    );
    assert_eq!(state(&conn), before);
}

#[tokio::test]
async fn twitter_backfill_rendered_entities_and_false_positives_match_fresh_rails() {
    let vector = rendered_oracle();
    let case = &vector["edges"];
    let app = arranged_rendered(case).await;
    let conn = Connection::open(app.db().path()).unwrap();
    let output = twitter_backfill_with_env(&conn, app.db().env()).unwrap();
    let actual = state(&conn);
    for row in case["inputs"].as_array().unwrap() {
        let references = |state: &Value| {
            state["references"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|reference| reference[0] == row["id"])
                .cloned()
                .collect::<Vec<_>>()
        };
        assert_eq!(
            references(&actual),
            references(&case["state"]),
            "{}: fresh Rails references",
            row["label"]
        );
    }
    assert_eq!(output, case["output"].as_str().unwrap());
    assert_eq!(actual, case["state"]);
    assert_eq!(
        twitter_backfill_with_env(&conn, app.db().env()).unwrap(),
        case["repeated_output"].as_str().unwrap()
    );
    assert_eq!(state(&conn), actual);
}

#[tokio::test]
async fn twitter_backfill_extraction_canonicalizes_without_sanitizing_or_rendering_source_selected_body()
 {
    let vector = rendered_oracle();
    let case = &vector["edges"];
    let app = arranged_rendered(case).await;
    let conn = Connection::open(app.db().path()).unwrap();
    twitter_backfill_with_env(&conn, app.db().env()).unwrap();
    let actual = state(&conn);
    for label in ["canonicalized_attachment_inner", "source_selected_raw_href"] {
        let row = case["inputs"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["label"] == label)
            .unwrap();
        assert_eq!(row["selected_by_markdown"], true);
        // Rails short-circuits before rendering this body. A malformed attachment would
        // raise on render, while Content#to_html only canonicalizes its stored children.
        assert!(row["rendered"].is_null());
        let references = |state: &Value| {
            state["references"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|reference| reference[0] == row["id"])
                .cloned()
                .collect::<Vec<_>>()
        };
        assert_eq!(
            references(&actual),
            references(&case["state"]),
            "{label}: canonical extraction"
        );
    }
}

#[tokio::test]
async fn twitter_backfill_rendering_and_canonical_html_match_fresh_rails() {
    let vector = rendered_oracle();
    let case = &vector["edges"];
    let app = arranged_rendered(case).await;
    let conn = Connection::open(app.db().path()).unwrap();
    for row in case["inputs"].as_array().unwrap() {
        let Some(html) = row["html"].as_str() else {
            assert_eq!(row["rendered"], "");
            assert!(row["canonical"].is_null());
            continue;
        };
        let rich_text = &app.db().env().rich_text;
        let canonical = rich_text.try_canonicalize_html(&conn, html).unwrap();
        assert_eq!(
            canonical,
            row["canonical"].as_str().unwrap(),
            "{}: canonical body",
            row["label"]
        );
        let text = crate::integrations::twitter::urls::non_code_text(&canonical).unwrap();
        assert_eq!(
            text,
            row["non_code_text"].as_str().unwrap(),
            "{}: extraction text",
            row["label"]
        );
        if row["selected_by_markdown"] == false {
            let rendered = rich_text.try_rendered_html(&conn, html).unwrap();
            assert_eq!(
                rendered,
                row["rendered"].as_str().unwrap(),
                "{}: complete rendered body",
                row["label"]
            );
            assert_eq!(
                crate::integrations::twitter::urls::is_post_url(&rendered),
                row["selected"].as_bool().unwrap(),
                "{}: selector",
                row["label"]
            );
        }
    }
}

#[tokio::test]
async fn twitter_operator_rendered_selector_uses_the_production_command() {
    let vector = rendered_oracle();
    let case = &vector["edges"];
    let app = arranged_rendered(case).await;
    let conn = Connection::open(app.db().path()).unwrap();
    let args = [
        "twitter-backfill-references".into(),
        app.db().path().to_str().unwrap().into(),
    ];
    assert_eq!(execute(&args).unwrap(), case["output"].as_str().unwrap());
    let first = state(&conn);
    // The production command uses wall-clock time; the complete frozen-clock state
    // differential above covers every claim timestamp. References and jobs are exact here.
    assert_eq!(first["references"], case["state"]["references"]);
    assert_eq!(first["jobs"], case["state"]["jobs"]);
    assert_eq!(
        execute(&args).unwrap(),
        case["repeated_output"].as_str().unwrap()
    );
    assert_eq!(
        state(&conn),
        first,
        "all post/reference/job facts retain their exact values"
    );
}
