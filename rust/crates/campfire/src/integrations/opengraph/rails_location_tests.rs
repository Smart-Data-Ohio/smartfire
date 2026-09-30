//! Every pinned Opengraph::LocationTest case, including its error messages.
use super::*;
use crate::integrations::{net::Network, test_support::*};
use std::sync::Arc;

async fn setup(route: Route) -> (FakeServer, Network, Arc<FakeResolver>) {
    let (server, roots) =
        FakeServer::start_named_tls_ws15e(vec![route], vec!["www.example.com".into()]).await;
    let resolver = Arc::new(FakeResolver::new([(
        "www.example.com",
        vec!["93.184.216.34"],
    )]));
    let net = Network {
        resolver: resolver.clone(),
        dialer: Arc::new(MappingDialer {
            public: ["93.184.216.34".parse().unwrap()].into(),
            to: server.addr,
            dialed: Default::default(),
        }),
        tls: crate::integrations::net::tls_config(roots),
    };
    (server, net, resolver)
}
fn route() -> Route {
    Route::new("GET", "www.example.com", "/", 200)
        .header("Content-Type", "text/html")
        .body("<body>ok<body>")
}
#[tokio::test]
async fn ws15e_rails_location_validations() {
    let (_s, n, _) = setup(route()).await;
    for url in ["https://www.example.com", "http://www.example.com"] {
        assert!(Location::new(&n, Some(url)).is_valid().await);
    }
    for url in [
        "~/etc/password",
        "ftp://speedtest.tele2.net",
        "httpfake",
        " foo",
        "https/incorrect",
    ] {
        assert!(!Location::new(&n, Some(url)).is_valid().await, "{url}");
    }
}
async fn assert_private(host: &str, addresses: &[&str]) {
    let (server, n, resolver) = setup(route()).await;
    for address in addresses {
        resolver.set(host, vec![vec![address.parse().unwrap()]]);
        let url = format!("https://{host}");
        let mut location = Location::new(&n, Some(&url));
        assert!(!location.is_valid().await);
        assert_eq!(location.errors.0, [("url", "is not public".into())]);
        assert!(!location.is_valid().await); // errors clear, DNS remains memoized
        assert_eq!(location.errors.0, [("url", "is not public".into())]);
        assert!(location.read_html().await.is_none());
    }
    assert!(server.received().is_empty());
    assert_eq!(resolver.lookups().len(), addresses.len());
}
#[tokio::test]
async fn ws15e_rails_location_private() {
    assert_private("www.example.com", &["172.16.0.0"]).await;
}
#[tokio::test]
async fn ws15e_rails_location_link_local() {
    assert_private("metadata.internal", &["169.254.169.254"]).await;
}
#[tokio::test]
async fn ws15e_rails_location_mapped_v4() {
    assert_private(
        "metadata.internal",
        &["::ffff:192.168.1.1", "::ffff:c0a8:0101"],
    )
    .await;
}
#[tokio::test]
async fn ws15e_rails_location_skips_files() {
    let (s, n, _) = setup(route()).await;
    for url in [
        "http://www.example.com/video.mp4",
        "http://www.example.com/archive.tar",
        "https://www.example.com/large.heic",
        "https://www.example.com/image.jpeg",
        "https://www.example.com/malware.exe",
        "https://www.example.com/massiveOS.iso",
    ] {
        assert!(Location::new(&n, Some(url)).read_html().await.is_none());
    }
    assert!(s.received().is_empty());
}
#[tokio::test]
async fn ws15e_rails_location_read_html() {
    let (s, n, _) = setup(route()).await;
    assert_eq!(
        Location::new(&n, Some("https://www.example.com"))
            .read_html()
            .await,
        Some(b"<body>ok<body>".to_vec())
    );
    assert_eq!(s.received().len(), 1);
}
#[tokio::test]
async fn ws15e_rails_location_rejects_large_response() {
    let (s, n, _) = setup(route().header("Content-Length", "1073741824")).await;
    assert!(
        Location::new(&n, Some("https://www.example.com"))
            .read_html()
            .await
            .is_none()
    );
    assert_eq!(s.received().len(), 1);
}
