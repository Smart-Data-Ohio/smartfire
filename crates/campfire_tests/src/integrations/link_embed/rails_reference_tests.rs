//! Complete pinned LinkEmbed::ReferenceSyncTest through real WS8 writes.
use super::*;
use crate::controllers::presenters::test_support::*;
use campfire_db::{Message, MessageChanges, NewMessage, Tx};
use rusqlite::params;
use std::time::Duration;
async fn app() -> TestApp {
    let mut app = TestApp::boot().await.expect("pinned seed required");
    app.booted.jobs.stop(Duration::from_secs(1)).await;
    app
}
fn message(tx: &mut Tx<'_>, source: &str) -> campfire_db::Result<Message> {
    Message::create(
        tx,
        NewMessage {
            room_id: ALL_TALK,
            creator_id: DAVID,
            markdown_source: Some(source.into()),
            ..Default::default()
        },
    )
}
fn urls(tx: &Tx<'_>, message: &Message) -> campfire_db::Result<Vec<String>> {
    let mut urls=Reference::for_message(tx.conn(),message)?.iter().map(|r|r.embed.normalized_url.clone()).collect::<Vec<_>>();
    urls.sort(); Ok(urls)
}

fn jobs(tx: &Tx<'_>) -> campfire_db::Result<i64> {
    Ok(tx.conn().query_row(
        "SELECT COUNT(*) FROM background_jobs WHERE job_class='LinkEmbed::FetchJob'",
        [],
        |r| r.get(0),
    )?)
}
fn clear_jobs(tx: &Tx<'_>) -> campfire_db::Result<()> {
    tx.conn().execute(
        "DELETE FROM background_jobs WHERE job_class='LinkEmbed::FetchJob'",
        [],
    )?;
    Ok(())
}
#[tokio::test]
async fn ws15e_rails_reference_basic() {
    let app = app().await;
    app.db().write(|tx|{let m=message(tx,"read https://example.com/one and https://example.com/two plus https://www.linkedin.com/feed/update/urn:li:activity:42")?;assert_eq!(jobs(tx)?,3);assert_eq!(urls(tx,&m)?,["https://example.com/one","https://example.com/two","https://www.linkedin.com/feed/update/urn:li:activity:42"]);assert_eq!(Reference::for_message(tx.conn(),&m)?.iter().map(|r|r.position).collect::<Vec<_>>(),[0,1,2]);Ok(())}).await.unwrap();
}
#[tokio::test]
async fn ws15e_rails_reference_idempotent_edit() {
    let app = app().await;
    app.db()
        .write(|tx| {
            let mut m = message(tx, "read https://example.com/before")?;
            clear_jobs(tx)?;
            sync_message(tx, &m, true)?;
            assert_eq!(jobs(tx)?, 0);
            m.edit(
                tx,
                MessageChanges {
                    markdown_source: Some("read https://example.com/after".into()),
                    ..Default::default()
                },
            )?;
            assert_eq!(jobs(tx)?, 1);
            assert_eq!(urls(tx, &m)?, ["https://example.com/after"]);
            Ok(())
        })
        .await
        .unwrap();
}
#[tokio::test]
async fn ws15e_rails_reference_skips_special_code_suppressed() {
    let app = app().await;
    app.db().write(|tx|{let m=message(tx,"PR https://github.com/rails/rails/pull/1 and post https://x.com/jack/status/20\n<https://example.com/hidden> and <https://www.linkedin.com/posts/bracketed-1>\n`https://example.com/code`\n```text\nhttps://example.com/fenced\n```\nbut https://example.com/kept stays")?;assert_eq!(urls(tx,&m)?,["https://example.com/kept"]);Ok(())}).await.unwrap();
}
#[tokio::test]
async fn ws15e_rails_reference_linkedin_period() {
    let app = app().await;
    app.db()
        .write(|tx| {
            let m = message(
                tx,
                "see https://www.linkedin.com/posts/slug-55. Next sentence",
            )?;
            assert_eq!(urls(tx, &m)?, ["https://www.linkedin.com/posts/slug-55"]);
            Ok(())
        })
        .await
        .unwrap();
}
#[tokio::test]
async fn ws15e_rails_reference_parentheses() {
    let app = app().await;
    app.db()
        .write(|tx| {
            let m = message(
                tx,
                "see https://en.wikipedia.org/wiki/Rust_(programming_language) today",
            )?;
            assert_eq!(
                urls(tx, &m)?,
                ["https://en.wikipedia.org/wiki/Rust_(programming_language)"]
            );
            Ok(())
        })
        .await
        .unwrap();
}
#[tokio::test]
async fn ws15e_rails_reference_separate_linkedin_cap() {
    let app = app().await;
    app.db().write(|tx|{let m=message(tx,"https://example.com/1 https://example.com/2 https://example.com/3 https://example.com/4 https://www.linkedin.com/posts/slug-99")?;assert_eq!(urls(tx,&m)?,["https://example.com/1","https://example.com/2","https://example.com/3","https://www.linkedin.com/posts/slug-99"]);assert_eq!(jobs(tx)?,4);Ok(())}).await.unwrap();
}
#[tokio::test]
async fn ws15e_rails_reference_legacy() {
    let app = app().await;
    app.db()
        .write(|tx| {
            let m = Message::create(
                tx,
                NewMessage {
                    room_id: ALL_TALK,
                    creator_id: DAVID,
                    body: Some("<div>Look: https://example.com/legacy</div>".into()),
                    ..Default::default()
                },
            )?;
            assert!(urls(tx, &m)?.is_empty());
            assert_eq!(jobs(tx)?, 0);
            Ok(())
        })
        .await
        .unwrap();
}
#[tokio::test]
async fn ws15e_rails_reference_suppressed() {
    let app = app().await;
    app.db()
        .write(|tx| {
            let mut m = message(tx, "read https://example.com/suppressed")?;
            tx.conn().execute(
                "DELETE FROM link_embed_references WHERE message_id=?",
                [m.id],
            )?;
            tx.conn().execute(
                "DELETE FROM link_embeds WHERE normalized_url='https://example.com/suppressed'",
                [],
            )?;
            tx.conn()
                .execute("UPDATE messages SET embeds_suppressed=1 WHERE id=?", [m.id])?;
            m = Message::find(tx.conn(), m.id)?;
            clear_jobs(tx)?;
            sync_message(tx, &m, true)?;
            assert_eq!(jobs(tx)?, 0);
            assert_eq!(urls(tx, &m)?, ["https://example.com/suppressed"]);
            Ok(())
        })
        .await
        .unwrap();
}
#[tokio::test]
async fn ws15e_rails_reference_own_raw_urls() {
    let app = app().await;
    app.db()
        .write(|tx| {
            let a = message(tx, "diagram https://excalidraw.com/#json=AAA,111")?;
            let b = Message::create(
                tx,
                NewMessage {
                    room_id: QUIET_CORNER,
                    creator_id: DAVID,
                    markdown_source: Some("diagram https://excalidraw.com/#json=BBB,222".into()),
                    ..Default::default()
                },
            )?;
            let a = Reference::for_message(tx.conn(), &a)?.remove(0);
            let b = Reference::for_message(tx.conn(), &b)?.remove(0);
            assert_eq!(a.embed.id, b.embed.id);
            assert_eq!(a.display_url(), "https://excalidraw.com/#json=AAA,111");
            assert_eq!(b.display_url(), "https://excalidraw.com/#json=BBB,222");
            Ok(())
        })
        .await
        .unwrap();
}
#[tokio::test]
async fn ws15e_rails_reference_fragment_edit() {
    let app = app().await;
    app.db()
        .write(|tx| {
            let mut m = message(tx, "diagram https://excalidraw.com/#json=AAA,111")?;
            clear_jobs(tx)?;
            m.edit(
                tx,
                MessageChanges {
                    markdown_source: Some("diagram https://excalidraw.com/#json=BBB,222".into()),
                    ..Default::default()
                },
            )?;
            assert_eq!(jobs(tx)?, 0);
            assert_eq!(
                Reference::for_message(tx.conn(), &m)?[0].display_url(),
                "https://excalidraw.com/#json=BBB,222"
            );
            Ok(())
        })
        .await
        .unwrap();
}
#[tokio::test]
async fn ws15e_rails_reference_expired_refetch() {
    let app = app().await;
    app.db()
        .write(|tx| {
            let m = message(tx, "read https://example.com/stale")?;
            let embed = Reference::for_message(tx.conn(), &m)?.remove(0).embed;
            let past = tx.now().ago(jiff::SignedDuration::from_hours(48));
            tx.conn().execute(
                "UPDATE link_embeds SET fetched_at=?,expires_at=?,fetch_requested_at=? WHERE id=?",
                params![
                    past,
                    tx.now().ago(jiff::SignedDuration::from_hours(24)),
                    past,
                    embed.id
                ],
            )?;
            clear_jobs(tx)?;
            sync_message(tx, &m, true)?;
            assert_eq!(jobs(tx)?, 1);
            Ok(())
        })
        .await
        .unwrap();
}
