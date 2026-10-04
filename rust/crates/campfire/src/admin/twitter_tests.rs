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
