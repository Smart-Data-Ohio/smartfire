use campfire_db::Message;
use crate::integrations::link_embed::Reference;
use crate::controllers::presenters::test_support::*;
use campfire_db::NewMessage;
use std::time::Duration;
async fn app() -> TestApp {
    let mut app = TestApp::boot().await.expect("pinned seed required");
    app.booted.jobs.stop(Duration::from_secs(1)).await;
    app
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
        let metadata=app.db().read(move |conn| Ok(Reference::for_message(conn,&message)?[0].embed.clone())).await.unwrap();
        match index {
            0 => { assert_eq!(metadata.title.as_deref(),Some("Jane & Jerry <3")); assert_eq!(metadata.description.as_deref(),Some("We shipped it.")); }
            1 => assert!(metadata.title.is_none()),
            2 => assert_eq!(metadata.title.as_deref(),Some("Open post")),
            _ => unreachable!(),
        }
    }
    assert_eq!(resolver.lookups().len(), 4);
}
