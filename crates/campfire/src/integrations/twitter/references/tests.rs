use super::*;
use campfire_db::Result;
use campfire_app::integrations::twitter::post::Post;
use campfire_db::Message;
use campfire_db::Tx;
use rusqlite::params;
use crate::controllers::presenters::test_support::*;
use campfire_db::{MessageChanges, NewMessage};
use jiff::SignedDuration;
use std::time::Duration;
async fn app() -> TestApp {
    let mut app = TestApp::boot().await.expect("pinned seed required");
    app.booted.jobs.stop(Duration::from_secs(1)).await;
    app
}
fn message(tx: &mut Tx<'_>, text: &str) -> Result<Message> {
    Message::create(
        tx,
        NewMessage {
            room_id: ALL_TALK,
            creator_id: DAVID,
            markdown_source: Some(text.into()),
            ..Default::default()
        },
    )
}
fn post_ids(tx: &Tx<'_>, id: i64) -> Result<Vec<String>> {
    Ok(Post::for_message(tx.conn(), id)?
        .into_iter()
        .map(|p| p.post_id)
        .collect())
}
fn job_count(tx: &Tx<'_>) -> Result<i64> {
    Ok(tx.conn().query_row(
        "SELECT COUNT(*) FROM background_jobs WHERE job_class='Twitter::FetchPostJob'",
        [],
        |r| r.get(0),
    )?)
}
#[tokio::test]
async fn ws15e_x_ws8_create_edit_code_retry_and_quiet_references() {
    let app = app().await;
    app.db().write(|tx| {
        let mut plain=message(tx,"just chatting")?;assert!(post_ids(tx,plain.id)?.is_empty());assert_eq!(job_count(tx)?,0);
        plain.edit(tx,MessageChanges {markdown_source:Some("now with https://x.com/jack/status/105".into()),..Default::default()})?;
        assert_eq!(post_ids(tx,plain.id)?,vec!["105"]);assert_eq!(job_count(tx)?,1);
        plain.edit(tx,MessageChanges {markdown_source:Some("never mind".into()),..Default::default()})?;assert!(post_ids(tx,plain.id)?.is_empty());
        let created=message(tx,"look https://twitter.com/jack/statuses/120?s=1")?;
        assert_eq!(post_ids(tx,created.id)?,vec!["120"]);assert!(!created.markdown_source.as_ref().unwrap().contains("x-post-card"));
        assert_eq!(Post::for_message(tx.conn(),created.id)?[0].url.as_deref(),Some("https://x.com/jack/status/120"));
        let count=job_count(tx)?;sync_message(tx,&created,true)?;assert_eq!(job_count(tx)?,count);
        let code=message(tx,"see `https://x.com/jack/status/901` inline\n\n```text\nhttps://x.com/jack/status/902\n```\n\nbut do read https://x.com/jack/status/903")?;
        assert_eq!(post_ids(tx,code.id)?,vec!["903"]);
        let duplicate=message(tx,"https://x.com/other/status/120")?;assert_eq!(post_ids(tx,duplicate.id)?,vec!["120"]);assert_eq!(job_count(tx)?,count+1,"only the new code-free post enqueued");
        let failed=Post::for_reference(tx,"104",Some("https://x.com/jack/status/104"))?;
        tx.conn().execute("UPDATE twitter_posts SET fetched_at=?,fetch_error='Post not found on X',fetch_requested_at=? WHERE id=?",params![tx.now().ago(SignedDuration::from_mins(5)),tx.now().ago(SignedDuration::from_mins(11)),failed.id])?;
        let count=job_count(tx)?;message(tx,"https://x.com/jack/status/104")?;assert_eq!(job_count(tx)?,count);
        tx.conn().execute("UPDATE twitter_posts SET fetched_at=? WHERE id=?",params![tx.now().ago(SignedDuration::from_mins(11)),failed.id])?;
        message(tx,"https://x.com/jack/status/104")?;assert_eq!(job_count(tx)?,count+1);
        let mut quiet=message(tx,"quiet source")?;
        tx.conn().execute("UPDATE action_text_rich_texts SET body='<p>https://x.com/i/web/status/888</p><code>https://x.com/a/status/889</code>' WHERE record_type='Message' AND record_id=?",[quiet.id])?;
        quiet.forward_note=Some("https://x.com/note/status/887".into());sync_message(tx,&quiet,false)?;
        assert_eq!(post_ids(tx,quiet.id)?,vec!["888","887"]);assert_eq!(job_count(tx)?,count+1);
        sync_message(tx,&quiet,true)?;assert_eq!(job_count(tx)?,count+1,"unchanged refs never enqueue, even after a quiet import");
        let posts=Post::for_message(tx.conn(),quiet.id)?;assert!(posts.iter().all(|p|p.fetched_at.is_none()));
        quiet.destroy(tx)?;assert!(Post::for_message(tx.conn(),quiet.id)?.is_empty(),"WS8 owns dependent joins");
        Ok(())
    }).await.unwrap();
}
#[tokio::test]
async fn ws15e_x_ws8_concurrent_creates_and_rejected_queue_roll_back_message() {
    let app = app().await;
    let tasks = (0..12)
        .map(|i| {
            let db = app.db().clone();
            tokio::spawn(async move {
                db.write(move |tx| {
                    let m = Message::create(
                        tx,
                        NewMessage {
                            room_id: if i % 2 == 0 { ALL_TALK } else { QUIET_CORNER },
                            creator_id: DAVID,
                            markdown_source: Some(
                                "https://x.com/u/status/9999999999999999999999999".into(),
                            ),
                            client_message_id: Some(format!("x-race-{i}")),
                            ..Default::default()
                        },
                    )?;
                    let refs = Post::for_message(tx.conn(), m.id)?;
                    assert_eq!(refs.len(), 1);
                    Ok(refs[0].id)
                })
                .await
                .unwrap()
            })
        })
        .collect::<Vec<_>>();
    let mut winner = None;
    for task in tasks {
        let id = task.await.unwrap();
        assert_eq!(id, *winner.get_or_insert(id));
    }
    app.db().write(|tx|{assert_eq!(job_count(tx)?,1);tx.conn().execute_batch("CREATE TEMP TRIGGER reject_x_message_job BEFORE INSERT ON background_jobs WHEN NEW.job_class='Twitter::FetchPostJob' BEGIN SELECT RAISE(ABORT,'reject X message'); END;")?;Ok(())}).await.unwrap();
    assert!(
        app.db()
            .write(|tx| message(tx, "https://x.com/a/status/666666"))
            .await
            .is_err()
    );
    app.db()
        .read(|conn| {
            assert_eq!(
                conn.query_row(
                    "SELECT COUNT(*) FROM twitter_posts WHERE post_id='666666'",
                    [],
                    |r| r.get::<_, i64>(0)
                )?,
                0
            );
            assert_eq!(
                conn.query_row(
                    "SELECT COUNT(*) FROM messages WHERE markdown_source LIKE '%666666%'",
                    [],
                    |r| r.get::<_, i64>(0)
                )?,
                0
            );
            Ok(())
        })
        .await
        .unwrap();
}
#[tokio::test]
async fn ws15e_x_backfill_reconciles_markdown_legacy_and_missing_richtext_idempotently() {
    for enqueue in [true, false] {
        let app = app().await;
        app.db().write(move|tx| {
            let markdown=message(tx,"look https://x.com/jack/status/301")?;
            let legacy=Message::create(tx,NewMessage {room_id:ALL_TALK,creator_id:DAVID,body:Some("<div>Look at this: https://x.com/jack/status/302</div>".into()),..Default::default()})?;
            let plain=message(tx,"just chatting")?;
            let bare=message(tx,"missing rich text")?;
            tx.conn().execute("DELETE FROM action_text_rich_texts WHERE record_type='Message' AND record_id=?",[bare.id])?;
            tx.conn().execute("DELETE FROM twitter_post_references WHERE message_id IN (?,?)",params![markdown.id,legacy.id])?;
            tx.conn().execute("DELETE FROM twitter_posts WHERE post_id IN ('301','302')",[])?;
            tx.conn().execute("DELETE FROM background_jobs WHERE job_class='Twitter::FetchPostJob'",[])?;
            assert_eq!(backfill(tx,enqueue)?,4,"the seed has x_card_post and the legacy opengraph_trix_tweet");
            assert_eq!(post_ids(tx,markdown.id)?,vec!["301"]);assert_eq!(post_ids(tx,legacy.id)?,vec!["302"]);assert!(post_ids(tx,plain.id)?.is_empty());assert!(post_ids(tx,bare.id)?.is_empty());
            assert_eq!(job_count(tx)?,if enqueue {3}else{0});
            let refs: i64=tx.conn().query_row("SELECT COUNT(*) FROM twitter_post_references",[],|r|r.get(0))?;
            assert_eq!(backfill(tx,enqueue)?,4);assert_eq!(job_count(tx)?,if enqueue {3}else{0});assert_eq!(tx.conn().query_row("SELECT COUNT(*) FROM twitter_post_references",[],|r|r.get::<_,i64>(0))?,refs);
            if !enqueue {
                assert!(tx.conn().query_row("SELECT fetch_requested_at FROM twitter_posts WHERE post_id='301'",[],|r|r.get::<_,Option<campfire_db::Timestamp>>(0))?.is_none());
            }
            Ok(())
        }).await.unwrap();
    }
}
