use super::*;
use crate::controllers::presenters::test_support::TestApp;
use crate::integrations::test_support::*;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

#[tokio::test]
async fn ws15e_link_fetch_records_positive_negative_and_guarded_image_results() {
    let mut app = TestApp::boot().await.expect("build parity seed");
    app.booted.jobs.stop(Duration::from_secs(1)).await;
    let page = |path: &str, html: &str| {
        Route::new("GET", "example.com", path, 200)
            .header("Content-Type", "text/html")
            .body(html)
    };
    let server = FakeServer::start_tls_ws15e(vec![
        page(
            "/good",
            "<meta property='og:title' content='Title'><meta property='og:image' content='/photo.png'>",
        ),
        Route::new("HEAD", "example.com", "/photo.png", 200).header("Content-Type", "IMAGE/PNG"),
        page("/upper-image", "<title>Safe title</title><meta property='og:image' content='HTTPs://example.com/photo.png'>"),
        page(
            "/param-image",
            "<title>Safe title</title><meta property='og:image' content='/param.png'>",
        ),
        Route::new("HEAD", "example.com", "/param.png", 200).header("Content-Type", "image/png; charset=binary"),
        page(
            "/private-image",
            "<title>Safe title</title><meta property='og:image' content='https://127.0.0.1/photo.png'>",
        ),
        page("/svg", "<title>Safe title</title><meta property='og:image' content='/vector.svg'>"),
        Route::new("HEAD", "example.com", "/vector.svg", 200).header("Content-Type", "image/svg+xml"),
        page(
            "/http-image",
            "<title>Safe title</title><meta property='og:image' content='http://example.com/photo.png'>",
        ),
        page("/empty", "<meta property='og:site_name' content='Site only'>"),
        page("/blank", " \n\t"),
        Route::new("GET", "example.com", "/error", 500),
        Route::new("GET", "example.com", "/redirect", 302).header("Location", "/redirect"),
    ])
    .await;
    let resolver = Arc::new(FakeResolver::new([("example.com", vec!["93.184.216.34"])]));
    let dialer = Arc::new(MappingDialer {
        public: HashSet::from(["93.184.216.34".parse().unwrap()]),
        to: server.addr,
        dialed: Mutex::new(Vec::new()),
    });
    let net = network(resolver, dialer.clone());
    for (path, error, image) in [
        ("/good", None, Some("https://example.com/photo.png")),
        ("/upper-image", None, Some("HTTPs://example.com/photo.png")),
        ("/param-image", None, None),
        ("/private-image", None, None),
        ("/svg", None, None),
        ("/http-image", None, None),
        ("/empty", Some("No preview available for this link"), None),
        ("/blank", Some("Could not load this link"), None),
        ("/error", Some("Could not load this link"), None),
        ("/redirect", Some("Could not load this link"), None),
    ] {
        let url = format!("https://example.com{path}");
        let embed = app.db().write(move |tx| Embed::for_reference(tx, &url)).await.unwrap();
        fetch(&app.booted.app, &net, embed.id).await.unwrap();
        let fetched = app.db().read(move |conn| Embed::find(conn, embed.id)).await.unwrap();
        assert_eq!(fetched.fetch_error.as_deref(), error, "{path}");
        assert_eq!(fetched.image_url.as_deref(), image, "{path}");
        let ttl = if error.is_some() { 3600 } else { 86400 };
        assert!((fetched.expires_at.unwrap().as_second() - fetched.fetched_at.unwrap().as_second() - ttl).abs() <= 1);
    }
    assert!(
        dialer
            .dialed
            .lock()
            .unwrap()
            .iter()
            .all(|addr| addr.ip() == "93.184.216.34".parse::<std::net::IpAddr>().unwrap())
    );
    let received = server.received.lock().unwrap();
    assert!(received.iter().all(|request| request.header("Cookie").is_none()));
    assert_eq!(received.iter().filter(|request| request.target == "/redirect").count(), 4);
    assert_eq!(
        received.iter().filter(|request| request.target == "/photo.png").count(),
        2,
        "HTTP and private images were never fetched"
    );
}

#[tokio::test]
async fn ws15e_link_durable_job_has_one_attempt_and_discards_deleted_records() {
    let mut app = TestApp::boot().await.expect("build parity seed");
    app.booted.jobs.stop(Duration::from_secs(1)).await;
    let embed = app
        .db()
        .write(|tx| {
            let embed = Embed::for_reference(tx, "http://127.0.0.1/private")?;
            super::super::store::request_fetch(tx, &embed)?;
            tx.emit_after_commit(campfire_db::Event::job(&FetchJob { embed_id: -1 }));
            Ok(embed)
        })
        .await
        .unwrap();
    let config = campfire_jobs::RunnerConfig::new(vec![campfire_jobs::QueueConfig::new("default", 1)]);
    let mut registry = campfire_jobs::Registry::new();
    registry.register(perform);
    let queue = campfire_jobs::JobQueue::new(&registry, &config).unwrap();
    let jobs = app.db().read(campfire_jobs::inspect::all).await.unwrap();
    assert_eq!(jobs.len(), 2);
    assert_eq!(FetchJob::retry_policy().attempts, 1);
    assert_eq!(FetchJob::retry_policy().retry_delay(1, None, 0.0), None);
    let runner = campfire_jobs::start(app.db().clone(), queue, registry, app.booted.app.clone(), config);
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if app.db().read(campfire_jobs::inspect::all).await.unwrap().is_empty() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("jobs performed");
    runner.shutdown(Duration::from_secs(1)).await;
    let embed = app.db().read(move |conn| Embed::find(conn, embed.id)).await.unwrap();
    assert_eq!(embed.fetch_error.as_deref(), Some("is not public"));
    assert!(!embed.needs_fetch(campfire_db::Timestamp::from_jiff(app.booted.app.clock.now())));
}
