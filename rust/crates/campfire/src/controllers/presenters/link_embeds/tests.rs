use super::*;
use crate::controllers::presenters::test_support::*;
use crate::integrations::link_embed::{Embed, metadata_parser::Metadata, sync_message};
use campfire_db::NewMessage;
use serde_json::json;
use std::time::Duration;

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
