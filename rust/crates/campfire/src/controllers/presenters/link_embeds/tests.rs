use super::*;
use crate::controllers::presenters::test_support::*;
use crate::integrations::link_embed::{Embed, metadata_parser::Metadata, sync_message};
use campfire_db::NewMessage;
use serde_json::json;
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
async fn ws15e_link_card_broadcasts_only_after_commit_with_message_key_and_scroll() {
    use crate::channels::tests::support::Client;
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    let mut app = TestApp::boot().await.expect("build parity seed");
    app.booted.jobs.stop(Duration::from_secs(1)).await;
    let (message, embed) = app
        .db()
        .write(|tx| {
            let message = Message::create(
                tx,
                NewMessage {
                    room_id: ALL_TALK,
                    creator_id: DAVID,
                    client_message_id: Some("ws15e-broadcast".into()),
                    body: Some("<p>https://example.com/broadcast#own</p>".into()),
                    ..Default::default()
                },
            )?;
            tx.conn().execute(
                "UPDATE messages SET markdown_source='https://example.com/broadcast#own' WHERE id=?",
                [message.id],
            )?;
            let message = Message::find(tx.conn(), message.id)?;
            sync_message(tx, &message, false)?;
            let embed = Reference::for_message(tx.conn(), &message)?[0].embed.clone();
            Ok((message, embed))
        })
        .await
        .unwrap();
    let listener = crate::integrations::test_support::ws15e_listener().await;
    let addr = listener.local_addr().unwrap();
    let router = app.booted.app.cable.router::<()>("/cable");
    let serving = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let mut request = format!("ws://{addr}/cable").into_client_request().unwrap();
    request.headers_mut().insert("origin", format!("http://{addr}").parse().unwrap());
    request
        .headers_mut()
        .insert("sec-websocket-protocol", "actioncable-v1-json".parse().unwrap());
    request.headers_mut().insert("cookie", david_cookie().parse().unwrap());
    let mut client = Client {
        socket: tokio_tungstenite::connect_async(request).await.unwrap().0,
    };
    assert_eq!(client.next_text().await, r#"{"type":"welcome"}"#);
    let room = app.db().read(|conn| Room::find(conn, ALL_TALK)).await.unwrap();
    let stream = Stream::conversation(&room, &message);
    let signed = rails_compat::turbo::signed_stream_name(&app.booted.app.secrets, &stream.streamables());
    let identifier = json!({"channel":"RoomMessagesChannel","signed_stream_name":signed}).to_string();
    client.confirm(&identifier).await;
    let rollback = embed.clone();
    let result: campfire_db::Result<()> = app
        .db()
        .write(move |tx| {
            rollback.save_metadata(
                tx,
                &Metadata {
                    title: Some("Rolled back".into()),
                    ..Default::default()
                },
            )?;
            Err(campfire_db::Error::Other("rollback".into()))
        })
        .await;
    assert!(result.is_err());
    client.assert_silent().await;
    app.db()
        .write(move |tx| {
            embed.save_metadata(
                tx,
                &Metadata {
                    title: Some("Committed & safe".into()),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
    let frame: serde_json::Value = serde_json::from_str(&client.next_text().await).unwrap();
    let html = frame["message"].as_str().unwrap();
    assert_eq!(frame["identifier"], identifier);
    assert!(html.contains("action=\"replace\""));
    assert!(html.contains("target=\"link_embed_cards_message_ws15e-broadcast\""));
    assert!(html.contains("maintain_scroll=\"true\""));
    assert!(html.contains("Committed &amp; safe"));
    assert!(html.contains("https://example.com/broadcast#own"));
    client.assert_silent().await;
    serving.abort();
}

#[tokio::test]
async fn ws15e_review_http_edit_removes_live_link_card() {
    use crate::channels::tests::support::Client;
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    use axum::http::Method;
    for bot in [false, true] {
    let mut app=TestApp::boot().await.expect("seed");
    app.booted.jobs.stop(Duration::from_secs(1)).await;
    let message=app.db().write(move |tx| {
        let message=Message::create(tx,NewMessage {room_id:ALL_TALK,creator_id:if bot { BENDER } else { DAVID },client_message_id:Some("reviewer-edit".into()),markdown_source:Some("https://example.com/reviewer-edit https://x.com/jack/status/80001 https://app.fizzy.do/897362094/cards/579 https://www.linkedin.com/posts/smartdata_activity-1234567890123456789-test".into()),..Default::default()})?;
        Reference::for_message(tx.conn(),&message)?[0].embed.save_metadata(tx,&Metadata {title:Some("Original card".into()),..Default::default()})?;
        Ok(message)
    }).await.unwrap();
    let listener=crate::integrations::test_support::ws15e_listener().await;
    let addr=listener.local_addr().unwrap();
    let router=app.booted.app.cable.router::<()>("/cable");
    let serving=tokio::spawn(async move {axum::serve(listener,router).await.unwrap()});
    let mut request=format!("ws://{addr}/cable").into_client_request().unwrap();
    request.headers_mut().insert("origin",format!("http://{addr}").parse().unwrap());
    request.headers_mut().insert("sec-websocket-protocol","actioncable-v1-json".parse().unwrap());
    request.headers_mut().insert("cookie",david_cookie().parse().unwrap());
    let mut client=Client {socket:tokio_tungstenite::connect_async(request).await.unwrap().0};
    assert_eq!(client.next_text().await,r#"{"type":"welcome"}"#);
    let room=app.db().read(|c|Room::find(c,ALL_TALK)).await.unwrap();
    let stream=Stream::conversation(&room,&message);
    let signed=rails_compat::turbo::signed_stream_name(&app.booted.app.secrets,&stream.streamables());
    let identifier=json!({"channel":"RoomMessagesChannel","signed_stream_name":signed}).to_string();
    client.confirm(&identifier).await;
    let response = if bot {
        app.anonymous().send(Req::new(Method::PATCH,&format!("/rooms/{ALL_TALK}/{BENDER_KEY}/messages/{}",message.id)).header("content-type","text/plain").body("Removed link")).await
    } else {
        app.david().write(Req::new(Method::PATCH,&format!("/rooms/{ALL_TALK}/messages/{}",message.id)).form(&[("message[markdown_source]","Removed link")])).await
    };
    assert_eq!(response.status, if bot { axum::http::StatusCode::OK } else { axum::http::StatusCode::FOUND });
    let mid=message.id;
    let refs:i64=app.db().read(move|c|Ok(c.query_row("SELECT COUNT(*) FROM link_embed_references WHERE message_id=?",[mid],|r|r.get(0))?)).await.unwrap();
    assert_eq!(refs,0);
    app.db().read(move |c| {
        for table in ["twitter_post_references", "fizzy_card_references"] {
            assert_eq!(c.query_row(&format!("SELECT COUNT(*) FROM {table} WHERE message_id=?"),[mid],|r|r.get::<_,i64>(0))?,0);
        }
        Ok(())
    }).await.unwrap();
    let mut frames=Vec::new();
    while let Ok(frame)=tokio::time::timeout(Duration::from_millis(400),client.next_text()).await {frames.push(frame);}
    eprintln!("REVIEW removed-link edit: references={refs}; websocket frames={}; bot={bot}",frames.len());
    serving.abort();
    let expected = ["presentation", "meta", "github_pr_cards", "twitter_cards", "message_link_cards", "fizzy_cards", "linkedin_cards", "link_embed_cards"];
    assert_eq!(frames.len(), expected.len(), "one ordered replacement per Rails partial, bot={bot}");
    for (frame, part) in frames.iter().zip(expected) {
        let frame: serde_json::Value = serde_json::from_str(frame).unwrap();
        assert_eq!(frame["identifier"], identifier);
        let html = frame["message"].as_str().unwrap();
        assert!(html.contains(&format!("target=\"{part}_message_reviewer-edit\"")), "{html}");
        assert!(html.contains("action=\"replace\""));
        assert!(html.contains("maintain_scroll=\"true\""));
        if part.ends_with("cards") {
            assert!(!html.contains("<article"));
            assert!(!html.contains("<turbo-frame"));
            assert!(!html.contains("Original card"));
        }
    }
    }
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

#[tokio::test]
async fn ws15e_review_seed_x_update_reaches_real_cable_caller() {
    use crate::channels::tests::support::Client;
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    let mut app = TestApp::boot().await.expect("pinned seed required");
    app.booted.jobs.stop(Duration::from_secs(1)).await;
    let (message, post) = app.db().read(|conn| {
        let message = Message::find(conn, 935961886)?;
        let posts = crate::integrations::twitter::post::Post::for_message(conn, message.id)?;
        assert_eq!(posts.len(), 1);
        Ok((message, posts[0].clone()))
    }).await.unwrap();
    let listener = crate::integrations::test_support::ws15e_listener().await;
    let addr = listener.local_addr().unwrap();
    let router = app.booted.app.cable.router::<()>("/cable");
    let serving = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let mut request = format!("ws://{addr}/cable").into_client_request().unwrap();
    request.headers_mut().insert("origin", format!("http://{addr}").parse().unwrap());
    request.headers_mut().insert("sec-websocket-protocol", "actioncable-v1-json".parse().unwrap());
    request.headers_mut().insert("cookie", david_cookie().parse().unwrap());
    let mut client = Client { socket: tokio_tungstenite::connect_async(request).await.unwrap().0 };
    assert_eq!(client.next_text().await, r#"{"type":"welcome"}"#);
    let room = app.db().read(move |conn| Room::find(conn, message.room_id)).await.unwrap();
    let stream = Stream::conversation(&room, &message);
    let signed = rails_compat::turbo::signed_stream_name(&app.booted.app.secrets, &stream.streamables());
    let identifier = json!({"channel":"RoomMessagesChannel","signed_stream_name":signed}).to_string();
    client.confirm(&identifier).await;
    let key = message_dom_id(&message, Some("twitter_cards"));
    app.db().write(move |tx| post.save_error(tx, "Review X refresh")).await.unwrap();
    let frame = tokio::time::timeout(Duration::from_millis(750), client.next_text()).await;
    eprintln!("REVIEW seeded X update: websocket frame received={}", frame.is_ok());
    serving.abort();
    let frame: serde_json::Value = serde_json::from_str(&frame.expect("X after_update_commit reaches cable")).unwrap();
    let html = frame["message"].as_str().unwrap();
    assert_eq!(frame["identifier"], identifier);
    assert!(html.contains(&format!("target=\"{key}\"")));
    assert!(html.contains("Couldn’t load this post."));
    assert!(html.contains("maintain_scroll=\"true\""));
}
