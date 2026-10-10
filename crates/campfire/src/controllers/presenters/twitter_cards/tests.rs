use crate::controllers::presenters::test_support::*;
use campfire_db::{Message, NewMessage};
use std::time::Duration;

async fn app() -> TestApp {
    let mut app = TestApp::boot().await.expect("pinned seed required");
    app.booted.jobs.stop(Duration::from_secs(1)).await;
    app
}
use crate::integrations::twitter::{post::Post, references};
use rusqlite::params;

fn legacy_body(id: &str) -> String {
    format!(
        "<div>Look at this: https://x.com/jack/status/{id}</div><action-text-attachment content-type=\"application/vnd.actiontext.opengraph-embed\" href=\"https://x.com/jack/status/{id}\" url=\"https://pbs.twimg.com/profile_images/1/avatar_200x200.jpg\" filename=\"jack (@jack)\" caption=\"just setting up my twttr\"></action-text-attachment>"
    )
}
#[tokio::test]
async fn ws15e_x_legacy_boxes_switch_to_cards_only_with_a_post_row_and_backfill() {
    let app = app().await;
    for (id, has_post) in [("20", true), ("201", false)] {
        let message = app.db().write(move |tx| {
            let message = Message::create(tx,NewMessage{room_id:ALL_TALK,creator_id:DAVID,client_message_id:Some(format!("ws15e-legacy-{id}")),body:Some(legacy_body(id)),..Default::default()})?;
            if has_post {
                let post = Post::for_message(tx.conn(),message.id)?.remove(0);
                tx.conn().execute("UPDATE twitter_posts SET text='just setting up my twttr',fetched_at=? WHERE id=?",params![tx.now(),post.id])?;
            } else {
                tx.conn().execute("DELETE FROM twitter_post_references WHERE message_id=?",[message.id])?;
                tx.conn().execute("DELETE FROM twitter_posts WHERE post_id=?",[id])?;
            }
            Ok(message)
        }).await.unwrap();
        app.db().write(move |tx| message.destroy(tx)).await.unwrap();
    }
    let message = app
        .db()
        .write(|tx| {
            let message = Message::create(
                tx,
                NewMessage {
                    room_id: ALL_TALK,
                    creator_id: DAVID,
                    body: Some(legacy_body("202")),
                    client_message_id: Some("ws15e-legacy-backfill".into()),
                    ..Default::default()
                },
            )?;
            tx.conn().execute(
                "DELETE FROM twitter_post_references WHERE message_id=?",
                [message.id],
            )?;
            tx.conn()
                .execute("DELETE FROM twitter_posts WHERE post_id='202'", [])?;
            let scanned = references::backfill(tx, false)?;
            assert_eq!(scanned, 3, "two matching seed messages plus this legacy message");
            assert_eq!(Post::for_message(tx.conn(), message.id)?[0].post_id, "202");
            Ok(message)
        })
        .await
        .unwrap();
    let mut browser = app.david();
    let _page = browser.get(&format!("/api/v1/rooms/{ALL_TALK}/messages")).await;
    let app2 = app.booted.app.clone();
    app.db()
        .read(move |c| {
            let html = super::super::Presenter::new(c, &app2, None).rendered_body_html(&message)?;
            assert!(!html.contains("og-embed"));
            Ok(())
        })
        .await
        .unwrap();
}
#[tokio::test]
async fn ws15e_x_pending_render_recovers_once_and_suppression_does_not_hide_x() {
    let app = app().await;
    let (message, post_id) = app
        .db()
        .write(|tx| {
            let message = Message::create(
                tx,
                NewMessage {
                    room_id: ALL_TALK,
                    creator_id: DAVID,
                    markdown_source: Some("https://x.com/jack/status/601".into()),
                    ..Default::default()
                },
            )?;
            let post = Post::for_message(tx.conn(), message.id)?.remove(0);
            tx.conn().execute(
                "UPDATE twitter_posts SET fetch_requested_at=NULL WHERE id=?",
                [post.id],
            )?;
            tx.conn().execute(
                "DELETE FROM background_jobs WHERE job_class='Twitter::FetchPostJob'",
                [],
            )?;
            tx.conn().execute(
                "UPDATE messages SET embeds_suppressed=1 WHERE id=?",
                [message.id],
            )?;
            Ok((message, post.id))
        })
        .await
        .unwrap();
    for _ in 0..2 {
        let mut browser = app.david();
        let page = browser.get(&format!("/api/v1/rooms/{ALL_TALK}/messages")).await;
        assert_eq!(page.status, axum::http::StatusCode::OK);
    }
    app.db()
        .read(move |c| {
            assert_eq!(
                c.query_row(
                    "SELECT COUNT(*) FROM background_jobs WHERE job_class='Twitter::FetchPostJob'",
                    [],
                    |r| r.get::<_, i64>(0)
                )?,
                1
            );
            assert!(Post::find(c, post_id)?.fetch_pending());
            Ok(())
        })
        .await
        .unwrap();
    for failed in [false, true] {
        app.db().write(move|tx|{
            tx.conn().execute("UPDATE twitter_posts SET fetched_at=?,fetch_error=?,fetch_requested_at=NULL WHERE id=?",params![tx.now().ago(jiff::SignedDuration::from_hours(1)),if failed{Some("Failed")}else{None},post_id])?;
            tx.conn().execute("DELETE FROM background_jobs WHERE job_class='Twitter::FetchPostJob'",[])?;
            Ok(())
        }).await.unwrap();
        let mut browser = app.david();
        let page = browser.get(&format!("/api/v1/rooms/{ALL_TALK}/messages")).await;
        assert_eq!(page.status, axum::http::StatusCode::OK);
        app.db().read(|c|{assert_eq!(c.query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='Twitter::FetchPostJob'",[],|r|r.get::<_,i64>(0))?,0);Ok(())}).await.unwrap();
    }
    app.db().write(move |tx| message.destroy(tx)).await.unwrap();
}

#[tokio::test]
async fn ws15e_x_numeric_order_preloads_a_page_and_memoizes_both_existence_results() {
    use campfire_richtext::AttachableResolver;
    use rusqlite::hooks::{AuthAction, Authorization};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let app = app().await;
    let messages=app.db().write(|tx|{
        (0..3).map(|_| Message::create(tx,NewMessage{room_id:ALL_TALK,creator_id:DAVID,markdown_source:Some("https://x.com/jack/status/502 https://x.com/jack/status/99 https://x.com/i/status/9999999999999999999999999 https://x.com/i/status/00001".into()),..Default::default()})).collect::<campfire_db::Result<Vec<_>>>()
    }).await.unwrap();
    let app2 = app.booted.app.clone();
    app.db()
        .read(move |c| {
            let presenter = super::super::Presenter::new(c, &app2, None).preload_payload(&messages)?;
            let views = messages.iter().map(|message| presenter.twitter_posts(message)).collect::<campfire_db::Result<Vec<_>>>()?;
            let reads = Arc::new(AtomicUsize::new(0));
            let observed = reads.clone();
            c.authorizer(Some(move |context: rusqlite::hooks::AuthContext<'_>| {
                if matches!(
                    context.action,
                    AuthAction::Read {
                        table_name: "twitter_posts",
                        ..
                    }
                ) {
                    observed.fetch_add(1, Ordering::SeqCst);
                }
                Authorization::Allow
            }));
            for message in &messages {
                assert_eq!(
                    presenter
                        .twitter_posts(message)?
                        .iter()
                        .map(|p| p.post_id.as_str())
                        .collect::<Vec<_>>(),
                    ["00001", "99", "502", "9999999999999999999999999"]
                );
            }
            assert_eq!(
                reads.load(Ordering::SeqCst),
                0,
                "preloaded page performs no per-message X SELECT"
            );
            let resolver = presenter.resolver();
            assert!(resolver.twitter_post_exists_for_url("https://x.com/a/status/99"));
            assert!(!resolver.twitter_post_exists_for_url("https://x.com/a/status/123456788876"));
            assert!(reads.load(Ordering::SeqCst) > 0);
            reads.store(0, Ordering::SeqCst);
            for _ in 0..2 {
                let resolver = presenter.resolver();
                assert!(resolver.twitter_post_exists_for_url("https://x.com/b/status/99"));
                assert!(
                    !resolver.twitter_post_exists_for_url("https://x.com/c/status/123456788876")
                );
            }
            assert_eq!(
                reads.load(Ordering::SeqCst),
                0,
                "new resolvers reuse positive and negative request memo"
            );
            c.authorizer(None::<fn(rusqlite::hooks::AuthContext<'_>) -> Authorization>);
            assert_eq!(views.len(), 3);

            Ok(())
        })
        .await
        .unwrap();
}



#[tokio::test]
async fn ws15e_x_render_and_broadcast_sibling_claims_rollback_if_enqueue_is_rejected() {
    let app = app().await;
    let (message,ids)=app.db().write(|tx| {
        let message=Message::create(tx,NewMessage{room_id:ALL_TALK,creator_id:DAVID,markdown_source:Some("https://x.com/a/status/801 https://x.com/a/status/802".into()),..Default::default()})?;
        let posts=Post::for_message(tx.conn(),message.id)?;
        tx.conn().execute("DELETE FROM background_jobs WHERE job_class='Twitter::FetchPostJob'",[])?;
        for post in &posts {tx.conn().execute("UPDATE twitter_posts SET fetch_requested_at=NULL WHERE id=?",[post.id])?;}
        tx.conn().execute_batch("CREATE TEMP TRIGGER reject_x_render_job BEFORE INSERT ON background_jobs WHEN NEW.job_class='Twitter::FetchPostJob' BEGIN SELECT RAISE(ABORT,'reject X render'); END;")?;
        Ok((message,posts.iter().map(|p|p.id).collect::<Vec<_>>()))
    }).await.unwrap();
    let mut browser = app.david();
    let response = browser.get(&format!("/api/v1/rooms/{ALL_TALK}/messages")).await;
    assert_eq!(
        response.status,
        axum::http::StatusCode::OK
    );
    let first = ids[0];
    let result = app
        .db()
        .write(move |tx| {
            Post::find(tx.conn(), first)?.save_error(tx, "Failed before sibling enqueue")
        })
        .await;
    assert!(result.is_err());
    let check = ids.clone();
    app.db()
        .read(move |c| {
            for id in check {
                let post = Post::find(c, id)?;
                assert!(post.fetch_pending());
                assert!(c.query_row(
                    "SELECT fetch_requested_at IS NULL FROM twitter_posts WHERE id=?",
                    [id],
                    |r| r.get::<_, bool>(0)
                )?);
            }
            assert_eq!(
                c.query_row(
                    "SELECT COUNT(*) FROM background_jobs WHERE job_class='Twitter::FetchPostJob'",
                    [],
                    |r| r.get::<_, i64>(0)
                )?,
                0
            );
            Ok(())
        })
        .await
        .unwrap();
    app.db()
        .write(|tx| {
            tx.conn()
                .execute_batch("DROP TRIGGER reject_x_render_job")?;
            Ok(())
        })
        .await
        .unwrap();
    let response = browser.get(&format!("/api/v1/rooms/{ALL_TALK}/messages")).await;
    assert_eq!(response.status, axum::http::StatusCode::OK);
    app.db()
        .read(|c| {
            assert_eq!(
                c.query_row(
                    "SELECT COUNT(*) FROM background_jobs WHERE job_class='Twitter::FetchPostJob'",
                    [],
                    |r| r.get::<_, i64>(0)
                )?,
                2
            );
            Ok(())
        })
        .await
        .unwrap();
    app.db().write(move |tx| message.destroy(tx)).await.unwrap();
}

#[tokio::test]
async fn ws15e_x_bot_http_message_creates_reference_and_durable_fetch() {
    let app = app().await;
    let mut bot = app.anonymous();
    let response = bot
        .send(
            Req::new(
                axum::http::Method::POST,
                &format!("/rooms/{ALL_TALK}/{BENDER_KEY}/messages"),
            )
            .header("content-type", "text/plain")
            .body("bot says https://x.com/jack/status/134"),
        )
        .await;
    assert_eq!(response.status, axum::http::StatusCode::CREATED);
    app.db()
        .read(|c| {
            let message = Message::find(
                c,
                c.query_row("SELECT MAX(id) FROM messages", [], |r| r.get::<_, i64>(0))?,
            )?;
            assert_eq!(message.creator_id, BENDER);
            assert_eq!(
                Post::for_message(c, message.id)?
                    .iter()
                    .map(|p| p.post_id.as_str())
                    .collect::<Vec<_>>(),
                ["134"]
            );
            assert_eq!(
                c.query_row(
                    "SELECT COUNT(*) FROM background_jobs WHERE job_class='Twitter::FetchPostJob'",
                    [],
                    |r| r.get::<_, i64>(0)
                )?,
                1
            );
            Ok(())
        })
        .await
        .unwrap();
}
