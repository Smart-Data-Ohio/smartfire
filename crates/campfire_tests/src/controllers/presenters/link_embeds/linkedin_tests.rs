use super::*;
use campfire_db::Message;
use campfire_db::Room;
use crate::cable::broadcasts::Stream;
use crate::integrations::link_embed::Reference;
use crate::controllers::presenters::test_support::*;
use crate::integrations::link_embed::{Embed, metadata_parser::Metadata, sync_message};
use campfire_db::{ChannelThread, NewChannelThread, NewMessage};
use serde_json::{Value, json};
use std::time::Duration;
async fn app() -> TestApp {
    let mut app = TestApp::boot().await.expect("pinned seed required");
    app.booted.jobs.stop(Duration::from_secs(1)).await;
    app
}
#[tokio::test]
async fn ws15e_linkedin_containers_match_pinned_rails_bytes() {
    let app = app().await;
    let vectors: Value = serde_json::from_str(include_str!(
        "../../../../../../vectors/ws15e_linkedin_cards.json"
    ))
    .unwrap();
    for case in vectors["containers"].as_array().unwrap() {
        let cards = case["cards"]
            .as_array()
            .unwrap()
            .iter()
            .map(|name| {
                vectors["cards"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|v| v["name"] == *name)
                    .unwrap()
                    .clone()
            })
            .collect::<Vec<_>>();
        let key = case["client_id"].as_str().unwrap().to_owned();
        let message=app.db().write(move|tx| {
   let message=Message::create(tx,NewMessage{room_id:ALL_TALK,creator_id:DAVID,client_message_id:Some(key),markdown_source:Some("Body".into()),..Default::default()})?;
   for (i,case) in cards.into_iter().enumerate() {
    let url=case["url"].as_str().unwrap();let embed=Embed::for_reference(tx,url)?;
    embed.save_metadata(tx,&serde_json::from_value::<Metadata>(case["attributes"].clone()).unwrap())?;
    tx.conn().execute("INSERT INTO link_embed_references (message_id,link_embed_id,position,url,created_at,updated_at) VALUES (?,?,?,?,?,?)",rusqlite::params![message.id,embed.id,i as i64,url,tx.now(),tx.now()])?;
   }Ok(message)
  }).await.unwrap();
        let app2 = app.booted.app.clone();
        let copy = message.clone();
        let html = app
            .db()
            .read(move |c| container(&app2, c, &copy, true))
            .await
            .unwrap();
        assert_eq!(html, case["html"].as_str().unwrap());
        app.db().write(move |tx| message.destroy(tx)).await.unwrap();
    }
}
#[tokio::test]
async fn ws15e_linkedin_real_fetches_and_room_html_use_each_reference() {
    use crate::integrations::test_support::*;
    use std::sync::Arc;
    let app = app().await;
    let (server,roots)=FakeServer::start_named_tls_ws15e(vec![
  Route::new("GET","www.linkedin.com","/feed/update/urn:li:activity:4242",200).header("Content-Type","text/html").body("<meta property='og:title' content='Jane &amp; Jerry &lt;3'><meta property='og:description' content='We shipped it.'><meta property='og:image' content='https://example.com/li.png'>"),
  Route::new("HEAD","example.com","/li.png",200).header("Content-Type","image/png"),
  Route::new("GET","www.linkedin.com","/posts/gated-post-99",200).header("Content-Type","text/html").body("<html><head></head><body>login</body></html>"),
  Route::new("GET","www.linkedin.com","/posts/open-post-100",200).header("Content-Type","text/html").body("<meta property='og:title' content='Open post'><meta property='og:description' content='Public excerpt.'>"),
 ],vec!["www.linkedin.com".into(),"example.com".into()]).await;
    let resolver = Arc::new(FakeResolver::new([
        ("www.linkedin.com", vec!["93.184.216.34"]),
        ("example.com", vec!["93.184.216.34"]),
    ]));
    let dialer = Arc::new(MappingDialer {
        public: ["93.184.216.34".parse().unwrap()].into(),
        to: server.addr,
        dialed: Default::default(),
    });
    let net = crate::net::Network {
        resolver: resolver.clone(),
        dialer,
        tls: crate::net::tls_config(roots),
    };
    for (index, url) in [
        "https://www.linkedin.com/feed/update/urn:li:activity:4242",
        "https://www.linkedin.com/posts/gated-post-99",
        "https://www.linkedin.com/posts/open-post-100",
    ]
    .into_iter()
    .enumerate()
    {
        let url = url.to_owned();
        let (message, embed) = app
            .db()
            .write(move |tx| {
                let message = Message::create(
                    tx,
                    NewMessage {
                        room_id: ALL_TALK,
                        creator_id: JASON,
                        client_message_id: Some(format!("ws15e-li-http-{index}")),
                        markdown_source: Some(format!("See {url}")),
                        ..Default::default()
                    },
                )?;
                let embed = Reference::for_message(tx.conn(), &message)?[0]
                    .embed
                    .clone();
                Ok((message, embed))
            })
            .await
            .unwrap();
        crate::integrations::link_embed::fetcher::fetch(&app.booted.app, &net, embed.id)
            .await
            .unwrap();
        let mut browser = app.david();
        let page = browser.get(&format!("/rooms/{ALL_TALK}")).await;
        assert_eq!(page.status, axum::http::StatusCode::OK);
        let app2 = app.booted.app.clone();
        let copy = message.clone();
        let html = app
            .db()
            .read(move |c| container(&app2, c, &copy, true))
            .await
            .unwrap();
        assert!(page.text().contains(&html));
        match index {
            0 => {
                assert!(html.contains("Jane &amp; Jerry &lt;3"), "{html}");
                assert!(!html.contains("&amp;amp;"));
                assert!(html.contains("We shipped it."));
                assert!(html.contains("src=\"https://example.com/li.png\""));
                assert!(html.contains("data-linkedin-embed-src-value=\"https://www.linkedin.com/embed/feed/update/urn:li:activity:4242\""));
                assert!(html.contains("Show embedded post"));
                assert!(!html.contains("<iframe"));
            }
            1 => {
                assert!(html.contains("linkedin-post-chip"));
                assert!(!html.contains("linkedin-post-card\""));
                assert!(html.contains("https://www.linkedin.com/posts/gated-post-99"));
            }
            2 => {
                assert!(html.contains("Open post"));
                assert!(!html.contains("Show embedded post"));
            }
            _ => unreachable!(),
        }
    }
    assert_eq!(resolver.lookups().len(), 4);
    // A room-shared row's raw fragment and official player must never replace this message's URL.
    for (room, fragment) in [(ALL_TALK, "room-one"), (QUIET_CORNER, "room-two")] {
        let message = app
            .db()
            .write(move |tx| {
                Message::create(
                    tx,
                    NewMessage {
                        room_id: room,
                        creator_id: DAVID,
                        client_message_id: Some(format!("ws15e-li-own-{fragment}")),
                        markdown_source: Some(format!(
                            "https://www.linkedin.com/feed/update/urn:li:activity:424242#{fragment}"
                        )),
                        ..Default::default()
                    },
                )
            })
            .await
            .unwrap();
        let mut browser = app.david();
        let page = browser.get(&format!("/rooms/{room}")).await;
        assert_eq!(page.status, axum::http::StatusCode::OK);
        let app2 = app.booted.app.clone();
        let html = app
            .db()
            .read(move |c| container(&app2, c, &message, true))
            .await
            .unwrap();
        assert!(page.text().contains(&html));
        assert!(html.contains(&format!("424242#{fragment}")));
        assert!(!html.contains(if fragment == "room-one" {
            "room-two"
        } else {
            "room-one"
        }));
    }
}
#[tokio::test]
async fn ws15e_linkedin_mixed_helpers_split_chips_and_use_own_player() {
    let app = app().await;
    let message=app.db().write(|tx|{
  let message=Message::create(tx,NewMessage{room_id:ALL_TALK,creator_id:DAVID,client_message_id:Some("ws15e-li-mixed".into()),markdown_source:Some("https://example.com/page https://www.linkedin.com/feed/update/urn:li:share:9 https://www.linkedin.com/posts/slug-9".into()),..Default::default()})?;
  for reference in Reference::for_message(tx.conn(),&message)? {
   reference.embed.save_metadata(tx,&Metadata{title:if reference.embed.linkedin(){None}else{Some("Page".into())},..Default::default()})?;
  }Ok(message)
 }).await.unwrap();
    let app2 = app.booted.app.clone();
    let copy = message.clone();
    app.db()
        .read(move |c| {
            let parts = components(&super::super::Presenter::new(c, &app2, None), &copy)?;
            assert_eq!(parts.link_embed_cards.len(), 1);
            assert_eq!(parts.linkedin_cards.len(), 2);
            assert!(parts.link_embed_cards[0].contains("Page"));
            assert!(
                parts
                    .linkedin_cards
                    .iter()
                    .all(|s| s.contains("linkedin-post-chip"))
            );
            let refs = Reference::for_message(c, &copy)?;
            assert_eq!(
                crate::integrations::linkedin::embed_url_for(
                    refs.iter()
                        .find(|r| r.display_url().contains("urn:li:share:"))
                        .unwrap()
                        .display_url()
                )
                .as_deref(),
                Some("https://www.linkedin.com/embed/feed/update/urn:li:share:9")
            );
            assert_eq!(
                crate::integrations::linkedin::embed_url_for(
                    refs.iter()
                        .find(|r| r.display_url().contains("/posts/"))
                        .unwrap()
                        .display_url()
                ),
                None
            );
            Ok(())
        })
        .await
        .unwrap();
    let copy=message.clone();
    app.db().write(move|tx| {
        for reference in Reference::for_message(tx.conn(),&copy)? {
            if !reference.embed.linkedin() {reference.embed.save_metadata(tx,&Metadata::default())?;}
        } Ok(())
    }).await.unwrap();
    let app2=app.booted.app.clone();
    app.db().read(move|c| {
        let parts=components(&super::super::Presenter::new(c,&app2,None),&message)?;
        assert!(parts.link_embed_cards.is_empty());assert_eq!(parts.linkedin_cards.len(),2);Ok(())
    }).await.unwrap();
}

#[tokio::test]
async fn ws15e_linkedin_broadcasts_commit_to_room_and_thread_with_own_key() {
    use crate::channels::tests::support::Client;
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    let mut app = TestApp::boot().await.expect("build parity seed");
    app.booted.jobs.stop(Duration::from_secs(1)).await;
    for threaded in [false, true] {
        let (message, embed) = app
        .db()
        .write(move |tx| {
            let thread_id=if threaded {Some(ChannelThread::create(tx,NewChannelThread{room_id:ALL_TALK,creator_id:DAVID,name:Some("LinkedIn thread".into()),..Default::default()})?.id)}else{None};
            let message = Message::create(
                tx,
                NewMessage {
                    room_id: ALL_TALK,
                    thread_id,
                    creator_id: DAVID,
                    client_message_id: Some("ws15e-broadcast".into()),
                    body: Some("<p>https://www.linkedin.com/feed/update/urn:li:activity:9000#own</p>".into()),
                    ..Default::default()
                },
            )?;
            tx.conn().execute(
                "UPDATE messages SET markdown_source='https://www.linkedin.com/feed/update/urn:li:activity:9000#own' WHERE id=?",
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
        request
            .headers_mut()
            .insert("origin", format!("http://{addr}").parse().unwrap());
        request.headers_mut().insert(
            "sec-websocket-protocol",
            "actioncable-v1-json".parse().unwrap(),
        );
        request
            .headers_mut()
            .insert("cookie", david_cookie().parse().unwrap());
        let mut client = Client {
            socket: tokio_tungstenite::connect_async(request).await.unwrap().0,
        };
        assert_eq!(client.next_text().await, r#"{"type":"welcome"}"#);
        let room = app
            .db()
            .read(|conn| Room::find(conn, ALL_TALK))
            .await
            .unwrap();
        let stream = Stream::conversation(&room, &message);
        let signed =
            rails_compat::turbo::signed_stream_name(&app.booted.app.secrets, &stream.streamables());
        let identifier =
            json!({"channel":"RoomMessagesChannel","signed_stream_name":signed}).to_string();
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
        assert!(html.contains("target=\"linkedin_cards_message_ws15e-broadcast\""));
        assert!(html.contains("maintain_scroll=\"true\""));
        assert!(html.contains("Committed &amp; safe"));
        assert!(html.contains("https://www.linkedin.com/feed/update/urn:li:activity:9000#own"));
        client.assert_silent().await;
        serving.abort();
    }
}
