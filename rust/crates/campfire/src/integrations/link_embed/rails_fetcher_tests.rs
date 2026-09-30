//! Complete pinned LinkEmbed::FetcherTest: persisted results over local guarded TLS.
use super::{Embed, fetcher::fetch};
use crate::{
    controllers::presenters::test_support::TestApp,
    integrations::{
        net::{self, BoxFuture, Dialer, Network},
        test_support::*,
    },
};
use std::{net::SocketAddr, sync::Arc, time::Duration};
async fn app() -> TestApp {
    let mut app = TestApp::boot().await.expect("pinned seed required");
    app.booted.jobs.stop(Duration::from_secs(1)).await;
    app
}
async fn setup(routes: Vec<Route>) -> (FakeServer, Network, Arc<FakeResolver>) {
    let (server, roots) =
        FakeServer::start_named_tls_ws15e(routes, vec!["example.com".into()]).await;
    let resolver = Arc::new(FakeResolver::new([
        ("example.com", vec!["93.184.216.34"]),
        ("intranet.example", vec!["10.0.0.5"]),
    ]));
    let dialer = Arc::new(MappingDialer {
        public: ["93.184.216.34".parse().unwrap()].into(),
        to: server.addr,
        dialed: Default::default(),
    });
    let net = Network {
        resolver: resolver.clone(),
        dialer,
        tls: net::tls_config(roots),
    };
    (server, net, resolver)
}
fn page(path: &str, body: &str) -> Route {
    Route::new("GET", "example.com", path, 200)
        .header("Content-Type", "text/html")
        .body(body)
}
async fn run(app: &TestApp, net: &Network, url: &str) -> Embed {
    let url = url.to_owned();
    let e = app
        .db()
        .write(move |tx| Embed::for_reference(tx, &url))
        .await
        .unwrap();
    fetch(&app.booted.app, net, e.id).await.unwrap();
    app.db().read(move |c| Embed::find(c, e.id)).await.unwrap()
}
fn ttl(embed: &Embed, seconds: i64) {
    assert_eq!(
        embed.expires_at.unwrap().as_second() - embed.fetched_at.unwrap().as_second(),
        seconds
    );
}
#[tokio::test]
async fn ws15e_rails_embed_fetch_metadata_ttl() {
    let app = app().await;
    let (_s,n,_)=setup(vec![page("/page","<meta property='og:title' content='Example Title'><meta property='og:description' content='An example description.'><meta property='og:site_name' content='Example'><meta property='og:image' content='https://example.com/image.png'>"),Route::new("HEAD","example.com","/image.png",200).header("Content-Type","image/png")]).await;
    let e = run(&app, &n, "https://example.com/page").await;
    assert_eq!(e.title.as_deref(), Some("Example Title"));
    assert_eq!(e.description.as_deref(), Some("An example description."));
    assert_eq!(e.site_name.as_deref(), Some("Example"));
    assert_eq!(
        e.image_url.as_deref(),
        Some("https://example.com/image.png")
    );
    assert!(e.fetch_error.is_none());
    ttl(&e, 86400);
}
#[tokio::test]
async fn ws15e_rails_embed_fetch_nonimage_keeps_text() {
    let app = app().await;
    let (_s,n,_)=setup(vec![page("/noimg","<meta property='og:title' content='No Image'><meta property='og:description' content='The image is an HTML page.'><meta property='og:image' content='https://example.com/not-an-image'>"),Route::new("HEAD","example.com","/not-an-image",200).header("Content-Type","text/html")]).await;
    let e = run(&app, &n, "https://example.com/noimg").await;
    assert_eq!(e.title.as_deref(), Some("No Image"));
    assert!(e.image_url.is_none());
    assert!(e.fetch_error.is_none());
}
#[tokio::test]
async fn ws15e_rails_embed_fetch_http_private_images() {
    let app = app().await;
    for image in ["http://example.com/image.png", "https://10.0.0.5/image.png"] {
        let body = format!(
            "<meta property='og:title' content='Plain Image'><meta property='og:image' content='{image}'>"
        );
        let (s, n, _) = setup(vec![page("/plainimg", &body)]).await;
        let e = run(&app, &n, "https://example.com/plainimg").await;
        assert_eq!(e.title.as_deref(), Some("Plain Image"));
        assert!(e.image_url.is_none());
        assert_eq!(s.received().len(), 1);
    }
}
#[tokio::test]
async fn ws15e_rails_embed_fetch_negative_ttl() {
    let app = app().await;
    let (_s, n, _) = setup(vec![page(
        "/empty",
        "<html><head></head><body>login wall</body></html>",
    )])
    .await;
    let e = run(&app, &n, "https://example.com/empty").await;
    assert_eq!(
        e.fetch_error.as_deref(),
        Some("No preview available for this link")
    );
    ttl(&e, 3600);
}
#[tokio::test]
async fn ws15e_rails_embed_fetch_private_before_http() {
    let app = app().await;
    let (s, n, _) = setup(vec![]).await;
    let e = run(&app, &n, "https://intranet.example/page").await;
    assert_eq!(e.fetch_error.as_deref(), Some("is not public"));
    assert!(e.expires_at.is_some());
    assert!(s.received().is_empty());
}
#[tokio::test]
async fn ws15e_rails_embed_fetch_private_redirect() {
    let app = app().await;
    let (s, n, _) = setup(vec![
        Route::new("GET", "example.com", "/redirect", 302)
            .header("Location", "https://intranet.example/secret"),
    ])
    .await;
    let e = run(&app, &n, "https://example.com/redirect").await;
    assert_eq!(e.fetch_error.as_deref(), Some("Could not load this link"));
    assert_eq!(s.received().len(), 1);
}
#[tokio::test]
async fn ws15e_rails_embed_fetch_three_and_four_hops() {
    let app = app().await;
    for fourth in [false, true] {
        let mut routes = (0..3)
            .map(|i| {
                Route::new("GET", "example.com", &format!("/hop-{i}"), 302)
                    .header("Location", &format!("/hop-{}", i + 1))
            })
            .collect::<Vec<_>>();
        routes.push(if fourth {
            Route::new("GET", "example.com", "/hop-3", 302).header("Location", "/hop-4")
        } else {
            page("/hop-3", "<meta property='og:title' content='Three hops'>")
        });
        routes.push(page(
            "/hop-4",
            "<meta property='og:title' content='Four hops'>",
        ));
        let (s, n, _) = setup(routes).await;
        let e = run(&app, &n, "https://example.com/hop-0").await;
        if fourth {
            assert_eq!(e.fetch_error.as_deref(), Some("Could not load this link"));
        } else {
            assert!(e.fetch_error.is_none());
            assert_eq!(e.title.as_deref(), Some("Three hops"));
        }
        assert_eq!(s.received().len(), 4);
    }
}
#[tokio::test]
async fn ws15e_rails_embed_fetch_deadline_is_negative_cache() {
    let app = app().await;
    let address = ws15e_trickling_server(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nConnection: close\r\n\r\n",
    )
    .await;
    let resolver = Arc::new(FakeResolver::new([("example.com", vec!["93.184.216.34"])]));
    let dialer = Arc::new(MappingDialer {
        public: ["93.184.216.34".parse().unwrap()].into(),
        to: address,
        dialed: Default::default(),
    });
    let n = network(resolver, dialer);
    let e = tokio::time::timeout(
        Duration::from_secs(13),
        run(&app, &n, "http://example.com/slow"),
    )
    .await
    .expect("overall ten-second deadline must bound fetch");
    assert_eq!(e.fetch_error.as_deref(), Some("Could not load this link"));
    ttl(&e, 3600);
}
struct Refused;
impl Dialer for Refused {
    fn connect(&self, _: SocketAddr) -> BoxFuture<'_, std::io::Result<tokio::net::TcpStream>> {
        Box::pin(async { Err(std::io::Error::from(std::io::ErrorKind::ConnectionRefused)) })
    }
}
#[tokio::test]
async fn ws15e_rails_embed_fetch_network_error_is_negative_cache() {
    let app = app().await;
    let (_s, mut n, _) = setup(vec![]).await;
    n.dialer = Arc::new(Refused);
    let e = run(&app, &n, "https://example.com/down").await;
    assert_eq!(e.fetch_error.as_deref(), Some("Could not load this link"));
    ttl(&e, 3600);
}
#[tokio::test]
async fn ws15e_rails_embed_fetch_no_cookies() {
    let app = app().await;
    let (s, n, _) = setup(vec![page("/page", "<html><head></head></html>")]).await;
    run(&app, &n, "https://example.com/page").await;
    let received = s.received();
    assert!(!received.is_empty());
    assert!(
        received
            .iter()
            .all(|r| r.header("Cookie").is_none() && r.header("Authorization").is_none())
    );
}
