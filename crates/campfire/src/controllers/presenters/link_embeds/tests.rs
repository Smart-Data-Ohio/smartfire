use super::*;
use campfire_db::Message;
use crate::integrations::link_embed::Reference;
use crate::controllers::presenters::test_support::*;
use crate::integrations::link_embed::{Embed, metadata_parser::Metadata, sync_message};
use campfire_db::NewMessage;
use std::time::Duration;

#[tokio::test]
async fn cached_message_render_retries_all_link_fetches_after_atomic_enqueue_failure() {
    let app = TestApp::boot().await.expect("build parity seed");
    let ids = app.db().write(|tx| {
        let message = Message::create(tx, NewMessage {room_id: ALL_TALK, creator_id: DAVID,
            body: Some("<p>https://example.com/cache-first https://example.com/cache-second</p>".into()),
            client_message_id: Some("cached-link-retry".into()), ..Default::default()})?;
        tx.conn().execute("UPDATE messages SET markdown_source = ? WHERE id = ?",
            ("https://example.com/cache-first https://example.com/cache-second", message.id))?;
        sync_message(tx, &Message::find(tx.conn(), message.id)?, false)?;
        tx.conn().execute_batch("CREATE TEMP TRIGGER hold_link_fetch AFTER INSERT ON background_jobs WHEN NEW.job_class = 'LinkEmbed::FetchJob' BEGIN UPDATE background_jobs SET run_at = '2099-01-01 00:00:00' WHERE id = NEW.id; END;
            CREATE TEMP TRIGGER reject_link_render BEFORE INSERT ON background_jobs WHEN NEW.job_class = 'LinkEmbed::FetchJob' BEGIN SELECT RAISE(ABORT, 'reject link render'); END;")?;
        Ok(Reference::for_message(tx.conn(), &message)?.into_iter().map(|reference| reference.embed.id).collect::<Vec<_>>())
    }).await.unwrap();
    assert_eq!(ids.len(), 2);
    let mut browser = app.david();
    let path = format!("/rooms/{ALL_TALK}/messages");
    assert_eq!(browser.get(&path).await.status, axum::http::StatusCode::INTERNAL_SERVER_ERROR);
    let check = ids.clone();
    app.db().write(move |tx| {
        for id in check {
            assert!(tx.conn().query_row("SELECT fetch_requested_at IS NULL FROM link_embeds WHERE id = ?", [id], |row| row.get::<_, bool>(0))?);
        }
        assert_eq!(tx.conn().query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class = 'LinkEmbed::FetchJob'", [], |row| row.get::<_, i64>(0))?, 0);
        tx.conn().execute_batch("DROP TRIGGER reject_link_render")?;
        Ok(())
    }).await.unwrap();
    let response = browser.get(&path).await;
    assert_eq!(response.status, axum::http::StatusCode::OK);
    app.db().read(move |conn| {
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class = 'LinkEmbed::FetchJob'", [], |row| row.get::<_, i64>(0))?, 2);
        for id in ids {
            assert!(conn.query_row("SELECT fetch_requested_at IS NOT NULL FROM link_embeds WHERE id = ?", [id], |row| row.get::<_, bool>(0))?);
        }
        Ok(())
    }).await.unwrap();
}

#[tokio::test]
async fn ws15e_link_containers_match_rails_and_view_requests_are_single_flight() {
    let mut app = TestApp::boot().await.expect("build parity seed");
    app.booted.jobs.stop(Duration::from_secs(1)).await;
    let vectors: serde_json::Value = serde_json::from_str(include_str!("../../../../../../vectors/ws15e_link_embed.json")).unwrap();
    for case in vectors["containers"].as_array().unwrap() {
        let attributes = case["attributes"].as_array().unwrap().clone();
        let client_id=case["client_id"].as_str().unwrap().to_string();
        let message=app.db().write(move |tx| {
            let message=Message::create(tx,NewMessage { room_id:ALL_TALK,creator_id:DAVID,client_message_id:Some(client_id),body:Some("body".into()),..Default::default() })?;
            for (index,attrs) in attributes.into_iter().enumerate() {
                let embed=Embed::for_reference(tx,&format!("https://example.com/card-{index}"))?;
                embed.save_metadata(tx,&serde_json::from_value::<Metadata>(attrs).unwrap())?;
                tx.conn().execute("INSERT INTO link_embed_references (message_id,link_embed_id,position,url,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?5)",rusqlite::params![message.id,embed.id,index as i64,"https://example.com/raw#own",tx.now()])?;
            }
            Ok(message)
        }).await.unwrap();
        let app2 = app.booted.app.clone();
        let message2 = message.clone();
        let html = app.db().read(move |conn| container(&app2, conn, &message2, false)).await.unwrap();
        assert_eq!(html, case["html"].as_str().unwrap());
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
                    client_message_id: Some("ws15e-stale".into()),
                    body: Some("<p>https://example.com/stale</p>".into()),
                    ..Default::default()
                },
            )?;
            tx.conn().execute(
                "UPDATE messages SET markdown_source='https://example.com/stale' WHERE id=?",
                [message.id],
            )?;
            let message = Message::find(tx.conn(), message.id)?;
            sync_message(tx, &message, false)?;
            Ok(message)
        })
        .await
        .unwrap();
    let mut browser = app.david();
    for _ in 0..2 {
        assert_eq!(browser.get(&format!("/rooms/{ALL_TALK}")).await.status, axum::http::StatusCode::OK);
    }
    let jobs = app.db().read(campfire_jobs::inspect::all).await.unwrap();
    assert_eq!(jobs.iter().filter(|job| job.class == "LinkEmbed::FetchJob").count(), 1);
    let id = message.id;
    app.db()
        .read(move |conn| {
            let mut message = Message::find(conn, id)?;
            assert!(!can_offer_suppression(conn, &message, DAVID)?, "unusable generic card");
            assert!(!can_offer_suppression(conn, &message, JASON)?);
            message.system_note = true;
            assert!(!can_offer_suppression(conn, &message, DAVID)?);
            Ok(())
        })
        .await
        .unwrap();
}





#[tokio::test]
async fn ws15e_review_seed_935961886_has_one_visible_x_card() {
    let mut app = TestApp::boot().await.expect("pinned seed required");
    app.booted.jobs.stop(Duration::from_secs(1)).await;
    let (room, post) = app.db().read(|conn| {
        let message = Message::find(conn, 935961886)?;
        let posts = crate::integrations::twitter::post::Post::for_message(conn, message.id)?;
        assert_eq!(posts.len(), 1, "pinned Rails fixture has one X association");
        Ok((message.room_id, posts[0].post_id.clone()))
    }).await.unwrap();
    let response = app.david().get(&format!("/rooms/{room}/messages/935961886")).await;
    assert_eq!(response.status, axum::http::StatusCode::OK);
    let marker = format!("<article class=\"x-post-card\" data-twitter-post=\"{post}\">");
    let count = response.text().matches(&marker).count();
    eprintln!("REVIEW seed message 935961886: visible X cards={count}");
    assert_eq!(count, 1);
}
