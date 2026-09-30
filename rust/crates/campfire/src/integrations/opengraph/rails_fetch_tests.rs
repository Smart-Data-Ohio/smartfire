//! One native case for each pinned test/models/opengraph/fetch_test.rb test.
use super::*;
use crate::integrations::{
    net::{BoxFuture, Dialer},
    test_support::{FakeResolver, FakeServer, MappingDialer, Route, network},
};
use std::{
    collections::HashSet,
    net::SocketAddr,
    sync::{Arc, Mutex},
};
fn to(server: SocketAddr) -> (Network, Arc<FakeResolver>, Arc<MappingDialer>) {
    let resolver = Arc::new(FakeResolver::new([
        ("www.example.com", vec!["93.184.216.34"]),
        ("www.other.com", vec!["93.184.216.35"]),
    ]));
    let dialer = Arc::new(MappingDialer {
        public: HashSet::from([
            "93.184.216.34".parse().unwrap(),
            "93.184.216.35".parse().unwrap(),
        ]),
        to: server,
        dialed: Mutex::new(Vec::new()),
    });
    (network(resolver.clone(), dialer.clone()), resolver, dialer)
}
fn url(path: &str) -> Uri {
    campfire_richtext::uri::parse(&format!("http://www.example.com{path}")).unwrap()
}
async fn document(routes: Vec<Route>) -> Result<Option<Vec<u8>>, FetchError> {
    let server = FakeServer::start_ws15e(routes).await;
    fetch_document(
        &to(server.addr).0,
        &url("/"),
        "93.184.216.34".parse().unwrap(),
    )
    .await
}
fn html(path: &str) -> Route {
    Route::new("GET", "www.example.com", path, 200)
        .header("Content-Type", "text/html")
        .body("<body>ok<body>")
}
#[tokio::test]
async fn ws15e_rails_opengraph_valid_html() {
    assert_eq!(
        document(vec![html("/")]).await.unwrap(),
        Some(b"<body>ok<body>".to_vec())
    );
}
#[tokio::test]
async fn ws15e_rails_opengraph_other_content_type() {
    assert_eq!(
        document(vec![
            Route::new("GET", "www.example.com", "/", 200)
                .header("Content-Type", "text/plain")
                .body("I'm not HTML!")
        ])
        .await
        .unwrap(),
        None
    );
}
#[tokio::test]
async fn ws15e_rails_opengraph_absolute_redirect() {
    let server = FakeServer::start_ws15e(vec![
        Route::new("GET", "www.example.com", "/", 302).header("Location", "http://www.other.com/"),
        Route::new("GET", "www.other.com", "/", 200)
            .header("Content-Type", "text/html")
            .body("<body>ok<body>"),
    ])
    .await;
    let (net, resolver, dialer) = to(server.addr);
    assert_eq!(
        fetch_document(&net, &url("/"), "93.184.216.34".parse().unwrap())
            .await
            .unwrap(),
        Some(b"<body>ok<body>".to_vec())
    );
    assert_eq!(resolver.lookups(), ["www.other.com"]);
    assert_eq!(dialer.dialed.lock().unwrap().len(), 2);
}
#[tokio::test]
async fn ws15e_rails_opengraph_relative_redirect() {
    assert_eq!(
        document(vec![
            Route::new("GET", "www.example.com", "/", 302).header("Location", "/other"),
            html("/other")
        ])
        .await
        .unwrap(),
        Some(b"<body>ok<body>".to_vec())
    );
}
#[tokio::test]
async fn ws15e_rails_opengraph_non_http_redirect() {
    assert!(matches!(
        document(vec![
            Route::new("GET", "www.example.com", "/", 302)
                .header("Location", "javascript:alert(1)")
        ])
        .await,
        Err(FetchError::RedirectDenied)
    ));
}
struct Stalled;
impl Dialer for Stalled {
    fn connect(&self, _: SocketAddr) -> BoxFuture<'_, std::io::Result<tokio::net::TcpStream>> {
        Box::pin(std::future::pending())
    }
}
#[tokio::test]
async fn ws15e_rails_opengraph_explicit_timeouts_bound_open() {
    let net = Network {
        resolver: Arc::new(FakeResolver::default()),
        dialer: Arc::new(Stalled),
        tls: crate::integrations::net::tls_config(rustls::RootCertStore::empty()),
    };
    let started = std::time::Instant::now();
    let result = tokio::time::timeout(
        Duration::from_secs(7),
        fetch_document(&net, &url("/"), "93.184.216.34".parse().unwrap()),
    )
    .await
    .expect("open timeout must bound stalled dial");
    assert!(matches!(
        result,
        Err(FetchError::Http(HttpError::OpenTimeout))
    ));
    assert!(started.elapsed() >= Duration::from_secs(5));
    assert_eq!(
        (TIMEOUTS.open, TIMEOUTS.read, TIMEOUTS.write),
        (
            Duration::from_secs(5),
            Duration::from_secs(5),
            Duration::from_secs(5)
        )
    );
}
#[tokio::test]
async fn ws15e_rails_opengraph_private_redirect() {
    let server = FakeServer::start_ws15e(vec![
        Route::new("GET", "www.example.com", "/", 302).header("Location", "http://www.other.com/"),
    ])
    .await;
    let (net, resolver, dialer) = to(server.addr);
    resolver.set("www.other.com", vec![vec!["127.0.0.1".parse().unwrap()]]);
    assert!(matches!(
        fetch_document(&net, &url("/"), "93.184.216.34".parse().unwrap()).await,
        Err(FetchError::Guard(_))
    ));
    assert_eq!(dialer.dialed.lock().unwrap().len(), 1);
}
#[tokio::test]
async fn ws15e_rails_opengraph_initial_dns_not_rebound() {
    let server = FakeServer::start_ws15e(vec![html("/")]).await;
    let (net, resolver, dialer) = to(server.addr);
    resolver.set(
        "www.example.com",
        vec![
            vec!["93.184.216.34".parse().unwrap()],
            vec!["127.0.0.1".parse().unwrap()],
        ],
    );
    let ip = guard::resolve(net.resolver.as_ref(), "www.example.com")
        .await
        .unwrap();
    assert!(fetch_document(&net, &url("/"), ip).await.unwrap().is_some());
    assert_eq!(resolver.lookups(), ["www.example.com"]);
    assert_eq!(
        *dialer.dialed.lock().unwrap(),
        ["93.184.216.34:80".parse::<SocketAddr>().unwrap()]
    );
}
#[tokio::test]
async fn ws15e_rails_opengraph_redirect_dns_not_rebound() {
    let server = FakeServer::start_ws15e(vec![
        Route::new("GET", "www.example.com", "/", 302).header("Location", "http://www.other.com/"),
        Route::new("GET", "www.other.com", "/", 200)
            .header("Content-Type", "text/html")
            .body("ok"),
    ])
    .await;
    let (net, resolver, dialer) = to(server.addr);
    resolver.set(
        "www.other.com",
        vec![
            vec!["93.184.216.35".parse().unwrap()],
            vec!["127.0.0.1".parse().unwrap()],
        ],
    );
    assert_eq!(
        fetch_document(&net, &url("/"), "93.184.216.34".parse().unwrap())
            .await
            .unwrap(),
        Some(b"ok".to_vec())
    );
    assert_eq!(resolver.lookups(), ["www.other.com"]);
    assert_eq!(
        *dialer.dialed.lock().unwrap(),
        [
            "93.184.216.34:80".parse::<SocketAddr>().unwrap(),
            "93.184.216.35:80".parse::<SocketAddr>().unwrap()
        ]
    );
}
#[tokio::test]
async fn ws15e_rails_opengraph_endless_redirect() {
    assert!(matches!(
        document(vec![
            Route::new("GET", "www.example.com", "/", 302)
                .header("Location", "http://www.example.com/")
        ])
        .await,
        Err(FetchError::TooManyRedirects)
    ));
}
#[tokio::test]
async fn ws15e_rails_opengraph_exact_redirect_budget() {
    for fourth in [false, true] {
        let mut routes = (0..3)
            .map(|i| {
                Route::new("GET", "www.example.com", &format!("/{i}"), 302)
                    .header("Location", &format!("/{next}", next = i + 1))
            })
            .collect::<Vec<_>>();
        routes.push(if fourth {
            Route::new("GET", "www.example.com", "/3", 302).header("Location", "/4")
        } else {
            html("/3")
        });
        routes.push(html("/4"));
        let server = FakeServer::start_ws15e(routes).await;
        let (net, _, _) = to(server.addr);
        let result = fetch_document_with(
            &net,
            &url("/0"),
            "93.184.216.34".parse().unwrap(),
            FetchOptions {
                max_redirects: 3,
                deadline: None,
            },
        )
        .await;
        if fourth {
            assert!(matches!(result, Err(FetchError::TooManyRedirects)));
        } else {
            assert_eq!(result.unwrap(), Some(b"<body>ok<body>".to_vec()));
        }
        assert_eq!(server.received().len(), 4);
    }
}
#[tokio::test]
async fn ws15e_rails_opengraph_slow_drip_deadline() {
    let server = crate::integrations::test_support::ws15e_trickling_server(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nConnection: close\r\n\r\n",
    )
    .await;
    let started = std::time::Instant::now();
    let (net, _, _) = to(server);
    assert!(matches!(
        fetch_document_with(
            &net,
            &url("/"),
            "93.184.216.34".parse().unwrap(),
            FetchOptions {
                max_redirects: 10,
                deadline: Some(Duration::from_millis(500))
            }
        )
        .await,
        Err(FetchError::Deadline)
    ));
    assert!(started.elapsed() < Duration::from_secs(4));
}
#[tokio::test]
async fn ws15e_rails_opengraph_declared_gigabyte() {
    assert_eq!(
        document(vec![html("/").header("Content-Length", "1073741824")])
            .await
            .unwrap(),
        None
    );
}
#[tokio::test]
async fn ws15e_rails_opengraph_missing_length_large_body() {
    let mut route = html("/").body(vec![b'x'; MAX_BODY_SIZE + 1]);
    route.chunked = true;
    assert_eq!(document(vec![route]).await.unwrap(), None);
}
#[tokio::test]
async fn ws15e_rails_opengraph_lying_length_large_body() {
    let mut route = html("/")
        .header("Content-Length", "1048576")
        .body(vec![b'x'; MAX_BODY_SIZE + 1]);
    route.chunked = true;
    assert_eq!(document(vec![route]).await.unwrap(), None);
}
#[tokio::test]
async fn ws15e_rails_opengraph_head_content_type() {
    let server = FakeServer::start_ws15e(vec![
        Route::new("HEAD", "www.example.com", "/image.png", 200)
            .header("Content-Type", "image/png"),
    ])
    .await;
    assert_eq!(
        fetch_content_type(
            &to(server.addr).0,
            &url("/image.png"),
            "93.184.216.34".parse().unwrap()
        )
        .await
        .unwrap(),
        Some("image/png".into())
    );
    assert_eq!(server.received()[0].method, "HEAD");
}
