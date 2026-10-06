use super::super::pull_requests::PullRequest;
use super::*;
use campfire_db::Tx;
use crate::{app, config::Config};
use campfire_db::{Message, MessageChanges, NewMessage, fixtures};

pub(crate) async fn application() -> (app::App, tempfile::TempDir) {
    let scratch = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.scratch/ws15g");
        std::fs::create_dir_all(&scratch).unwrap();
        let dir = tempfile::tempdir_in(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.scratch/ws15g"),
    )
    .unwrap();
    let root = dir.path().to_string_lossy().into_owned();
    let secret:serde_json::Value=serde_json::from_str(include_str!("../../../../../../vectors/github.json")).unwrap();
    let config = Config::from_lookup(|name| match name {
        "SECRET_KEY_BASE" => secret["secret_key_base"].as_str().map(str::to_owned),
        "CAMPFIRE_STORAGE_PATH" => Some(root.clone()),
        "DISABLE_SSL" => Some("1".into()),
        _ => None,
    })
    .unwrap();
    let booted = crate::server::boot_with_github_read(
        config,
        std::sync::Arc::new(campfire_kit::FrozenClock::new(
            "2026-01-01T12:00:00Z".parse().unwrap(),
        )),
        super::super::client::ReadClient::new(None),
    )
    .await
    .unwrap();
    booted
        .jobs
        .shutdown(std::time::Duration::from_secs(1))
        .await;
    booted
        .app
        .db
        .write(|tx| {
            fixtures::load(
                tx.conn(),
                &fixtures::reference_dir(),
                &fixtures::Options {
                    now: tx.now(),
                    bcrypt_cost: 4,
                },
            )
        })
        .await
        .unwrap();
    (booted.app, dir)
}

#[test]
fn github_url_extraction_and_non_code_html_match_pinned_rails() {
    let vectors: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../vectors/github_references.json"
    ))
    .unwrap();
    for case in vectors["texts"].as_array().unwrap() {
        let text = case["text"].as_str().unwrap_or("");
        assert_eq!(
            serde_json::to_value(extract(text)).unwrap(),
            case["references"],
            "{case}"
        );
        assert_eq!(pull_request_url(text), case["matches"], "{case}");
    }
    for case in vectors["html"].as_array().unwrap() {
        assert_eq!(
            campfire_db::models::message_reference::non_code_text(case["html"].as_str().unwrap())
                .unwrap(),
            case["text"],
            "{case}"
        );
    }
}
fn new_message() -> NewMessage {
    NewMessage {
        room_id: fixtures::identify("designers"),
        creator_id: fixtures::identify("david"),
        ..Default::default()
    }
}
fn numbers(tx: &Tx<'_>, message: &Message) -> campfire_db::Result<Vec<i64>> {
    Ok(tx.conn().prepare("SELECT p.number FROM github_pull_requests p JOIN github_pull_request_references r ON r.github_pull_request_id=p.id WHERE r.message_id=? ORDER BY p.number")?.query_map([message.id],|r|r.get(0))?.collect::<rusqlite::Result<_>>()?)
}

#[tokio::test]
async fn github_fresh_card_new_reference_enqueues_but_existing_reference_does_not() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../vectors/messaging/fresh-github-reference.json"
    ))
    .unwrap();
    // application() stops the runner before loading fixtures: record jobs, never fetch.
    let (app, _dir) = application().await;
    app.db.write(move |tx| {
        let pr = PullRequest::for_reference(tx, "rails", "rails", 3141)?;
        tx.conn().execute(
            "UPDATE github_pull_requests SET private=0,title='Cached card',state='open',fetched_at=?,fetch_requested_at=NULL WHERE id=?",
            (tx.now(), pr.id),
        )?;
        let message = Message::create_markdown(tx, new_message(), "see https://github.com/rails/rails/pull/3141")?;
        for row in oracle["rows"].as_array().unwrap() {
            if row["name"] == "existing_reference" {
                tx.conn().execute("UPDATE github_pull_requests SET fetch_requested_at=NULL WHERE id=?", [pr.id])?;
                sync(tx, &message, true)?;
            }
            assert_eq!(serde_json::json!(PullRequest::find(tx.conn(), pr.id)?.stale(tx.now())), row["stale"]);
            let count: i64 = tx.conn().query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='Github::FetchPullRequestJob'", [], |r| r.get(0))?;
            assert_eq!(serde_json::json!(count), row["fetches"], "{}", row["name"]);
        }
        Ok(())
    }).await.unwrap();
}

#[tokio::test]
async fn github_message_create_and_edit_hooks_reconcile_references_and_fetches() {
    let (app, _dir) = application().await;
    app.db.write(|tx| {
        let mut message=Message::create_markdown(tx,new_message(),"https://github.com/Rails/Rails/pull/7 and https://github.com/rails/rails/pull/007")?;
        assert_eq!(numbers(tx,&message)?,vec![7]);
        assert_eq!(tx.conn().query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='Github::FetchPullRequestJob'",[],|r|r.get::<_,i64>(0))?,1);
        assert!(!message.markdown_source.as_ref().unwrap().contains("github-pr-card"));
        message.update(tx,MessageChanges {markdown_source:Some("now https://github.com/o/r/pull/8".into()),..Default::default()})?;
        assert_eq!(numbers(tx,&message)?,vec![8]);
        message.update(tx,MessageChanges {markdown_source:Some("never mind".into()),..Default::default()})?;
        assert!(numbers(tx,&message)?.is_empty());
        message.update(tx,MessageChanges {markdown_source:Some("added https://github.com/o/r/pull/10".into()),..Default::default()})?;
        assert_eq!(numbers(tx,&message)?,vec![10]);
        let empty=Message::create_markdown(tx,new_message(),"hello")?;
        assert!(numbers(tx,&empty)?.is_empty());
        Ok(())
    }).await.unwrap();
}

#[tokio::test]
async fn github_message_reference_security_ignores_code_and_caps_before_case_normalization() {
    let (app, _dir) = application().await;
    app.db.write(|tx| {
        let message=Message::create_markdown(tx,new_message(),"see `https://github.com/o/r/pull/1`\n\n```text\nhttps://github.com/o/r/pull/2\n```\n\n[the PR](https://github.com/o/r/pull/3)\n\nhttps://github.com/o/r/pull/4\n\nhttps://github.com/o/r/pull/5\n\nhttps://github.com/o/r/pull/6\n\nhttps://github.com/o/r/pull/7")?;
        assert_eq!(numbers(tx,&message)?,vec![4,5,6,7]);
        let labeled=Message::create_markdown(tx,new_message(),"[the PR](https://github.com/o/r/pull/3)")?;
        assert_eq!(numbers(tx,&labeled)?,vec![3]);
        let message=Message::create_markdown(tx,new_message(),"https://github.com/O/R/pull/11 https://github.com/o/r/pull/11 https://github.com/o/r/pull/12 https://github.com/o/r/pull/13 https://github.com/o/r/pull/14")?;
        assert_eq!(numbers(tx,&message)?,vec![11,12,13]);
        let mut attributes=new_message();
        attributes.forward_note=Some("https://github.com/o/r/pull/15".into());
        let forwarded=Message::create_markdown(tx,attributes,"hello")?;
        assert_eq!(numbers(tx,&forwarded)?,vec![15]);
        Ok(())
    }).await.unwrap();
}

#[tokio::test]
async fn github_message_reference_queue_failure_rolls_back_message_pr_and_claim() {
    let (app, _dir) = application().await;
    app.db.write(|tx| {tx.conn().execute_batch("CREATE TRIGGER reject_github_fetch BEFORE INSERT ON background_jobs WHEN NEW.job_class='Github::FetchPullRequestJob' BEGIN SELECT RAISE(ABORT,'queue unavailable'); END;")?;Ok(())}).await.unwrap();
    assert!(
        app.db
            .write(|tx| Message::create_markdown(
                tx,
                new_message(),
                "https://github.com/o/r/pull/9"
            ))
            .await
            .is_err()
    );
    app.db.read(|conn| {
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM github_pull_requests",[],|r|r.get::<_,i64>(0))?,0);
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM github_pull_request_references",[],|r|r.get::<_,i64>(0))?,0);
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM messages WHERE markdown_source LIKE '%github.com/o/r/pull/9%'",[],|r|r.get::<_,i64>(0))?,0);
        Ok(())
    }).await.unwrap();
}

#[tokio::test]
async fn github_message_stream_and_import_reference_entrypoints_keep_fetches_quiet() {
    let (app, _dir) = application().await;
    app.db.write(|tx| {
        let mut attributes=new_message();attributes.streaming=true;
        let mut message=Message::create_markdown(tx,attributes,"https://github.com/o/r/pull/17")?;
        assert!(numbers(tx,&message)?.is_empty());
        message.update(tx,MessageChanges {markdown_source:Some("https://github.com/o/r/pull/18".into()),..Default::default()})?;
        assert!(numbers(tx,&message)?.is_empty());
        assert!(message.claim_stream_finalized(tx)?);
        message.sync_integration_references(tx,false)?;
        assert_eq!(numbers(tx,&message)?,vec![18]);
        assert_eq!(tx.conn().query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='Github::FetchPullRequestJob'",[],|r|r.get::<_,i64>(0))?,0);
        message.sync_integration_references(tx,true)?;
        assert_eq!(tx.conn().query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='Github::FetchPullRequestJob'",[],|r|r.get::<_,i64>(0))?,1);
        message.sync_integration_references(tx,true)?;
        assert_eq!(tx.conn().query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='Github::FetchPullRequestJob'",[],|r|r.get::<_,i64>(0))?,1);
        Ok(())
    }).await.unwrap();
}
