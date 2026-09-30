//! Replays testdata/opengraph_cases.json (fake DNS, a fake server for the fake public addresses)
//! and compares the response, the DNS lookups and the HTTP requests with what the reference
//! did for the same cases (testdata/opengraph_expected.json, from testdata/oracle/opengraph.rb).

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use base64::Engine;
use serde_json::Value;

use super::*;
use crate::integrations::test_support::{FakeResolver, FakeServer, MappingDialer, Route, gzip_bomb, network, trickling_server_with_ready};

fn route(spec: &Value) -> Route {
    let s = |key: &str| spec[key].as_str().unwrap_or_default().to_string();
    let mut route = Route::new(&s("method"), &s("host"), &s("path"), spec["status"].as_u64().unwrap() as u16);
    route.headers = serde_json::from_value(spec["headers"].clone()).unwrap();
    let mut body = if let Some(b64) = spec["body_b64"].as_str() {
        base64::engine::general_purpose::STANDARD.decode(b64).unwrap()
    } else if let Some(repeat) = spec["body_repeat"].as_array() {
        repeat[0].as_str().unwrap().repeat(repeat[1].as_u64().unwrap() as usize).into_bytes()
    } else {
        s("body").into_bytes()
    };
    if let Some(pad_to) = spec["pad_to"].as_u64() {
        body.resize(pad_to as usize, b' ');
    }
    route.body = body;
    route.chunked = spec["chunked"].as_bool().unwrap_or(false);
    route.gzip = spec["gzip"].as_bool().unwrap_or(false);
    route
}

fn answers(spec: &Value) -> Vec<Vec<std::net::IpAddr>> {
    spec.as_array()
        .unwrap()
        .iter()
        .map(|list| list.as_array().unwrap().iter().map(|ip| ip.as_str().unwrap().parse().unwrap()).collect())
        .collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn unfurls_like_the_reference() {
    let spec: Value = serde_json::from_str(include_str!("../testdata/opengraph_cases.json")).unwrap();
    let expected: Value = serde_json::from_str(include_str!("../testdata/opengraph_expected.json")).unwrap();
    let server = FakeServer::start(spec["routes"].as_array().unwrap().iter().map(route).collect()).await;
    let public: HashSet<std::net::IpAddr> =
        spec["public_ips"].as_array().unwrap().iter().map(|ip| ip.as_str().unwrap().parse().unwrap()).collect();

    let mut failures = Vec::new();
    for (case, expected) in spec["cases"].as_array().unwrap().iter().zip(expected.as_array().unwrap()) {
        let name = case["name"].as_str().unwrap();
        assert_eq!(name, expected["name"].as_str().unwrap());
        let resolver = Arc::new(FakeResolver::default());
        for (host, list) in spec["hosts"].as_object().unwrap() {
            resolver.set(host, answers(list));
        }
        let dialer = Arc::new(MappingDialer { public: public.clone(), to: server.addr, dialed: Mutex::new(Vec::new()) });
        let net = network(resolver.clone(), dialer);
        let before = server.received().len();

        let response = match unfurl(&net, case["url"].as_str().unwrap()).await {
            Ok(Unfurl::Json(body)) => serde_json::json!({ "status": 200, "body": body }),
            Ok(Unfurl::NoContent) => serde_json::json!({ "status": 204 }),
            Err(UnfurlError::Raised(class)) => serde_json::json!({ "status": 500, "error": class }),
        };
        let requests: Vec<Value> = server.received()[before..]
            .iter()
            .map(|r| {
                serde_json::json!([
                    r.method,
                    r.header("host"),
                    r.target,
                    r.header("accept"),
                    r.header("accept-encoding"),
                    r.header("user-agent")
                ])
            })
            .collect();
        let actual = serde_json::json!({ "response": response, "lookups": resolver.lookups(), "requests": requests });
        let wanted =
            serde_json::json!({ "response": expected["response"], "lookups": expected["lookups"], "requests": expected["requests"] });
        if actual != wanted {
            failures.push(format!("{name}:\n  expected {wanted}\n  actual   {actual}"));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} cases differ:\n{}",
        failures.len(),
        spec["cases"].as_array().unwrap().len(),
        failures.join("\n")
    );
}

/// test/controllers/unfurl_links_controller_test.rb over plain HTTPS: the pinned address,
/// with the certificate verified against the host name.
#[tokio::test]
async fn unfurls_over_https() {
    let page = "<html><head><meta property=\"og:url\" content=\"https://example.com\"><meta property=\"og:title\" content=\"Hey!\"><meta property=\"og:description\" content=\"desc..\"><meta property=\"og:image\" content=\"https://example.com/image.png\"></head></html>";
    let server = FakeServer::start_tls(vec![
        Route::new("GET", "www.example.com", "/", 200).header("Content-Type", "text/html").body(page),
        Route::new("HEAD", "example.com", "/image.png", 200).header("Content-Type", "image/png"),
    ])
    .await;
    let resolver = Arc::new(FakeResolver::new([("www.example.com", vec!["93.184.216.34"]), ("example.com", vec!["93.184.216.35"])]));
    let public = HashSet::from(["93.184.216.34".parse().unwrap(), "93.184.216.35".parse().unwrap()]);
    let dialer = Arc::new(MappingDialer { public, to: server.addr, dialed: Mutex::new(Vec::new()) });
    let net = network(resolver, dialer.clone());

    let Unfurl::Json(body) = unfurl(&net, "https://www.example.com").await.unwrap() else { panic!("no content") };
    assert_eq!(
        body,
        r#"{"title":"Hey!","url":"https://example.com","image":"https://example.com/image.png","description":"desc..","context_for_validation":{"context":null},"errors":{}}"#
    );
    let dialed: Vec<String> = dialer.dialed.lock().unwrap().iter().map(|a| a.to_string()).collect();
    assert_eq!(dialed, ["93.184.216.34:443", "93.184.216.35:443"]);

    // A certificate that doesn't verify is a failed fetch.
    let untrusted = Network { tls: crate::integrations::net::tls_config(rustls::RootCertStore::empty()), ..net };
    assert_eq!(unfurl(&untrusted, "https://www.example.com").await, Ok(Unfurl::NoContent));
}

/// www.example.com, at a fake public address that connects to `server`.
fn network_to(server: std::net::SocketAddr) -> Network {
    let resolver = Arc::new(FakeResolver::new([("www.example.com", vec!["93.184.216.34"])]));
    let dialer =
        Arc::new(MappingDialer { public: HashSet::from(["93.184.216.34".parse().unwrap()]), to: server, dialed: Mutex::new(Vec::new()) });
    network(resolver, dialer)
}

/// Stop once the decoded page exceeds 5MB, before inspecting the malformed gzip tail.
/// Reading the entire body would report an inflation error. A smaller page still unfurls.
#[tokio::test]
async fn stops_reading_a_gzip_bomb_at_the_limit() {
    use std::io::Write;
    let page = "<meta property=\"og:title\" content=\"Hey!\"><meta property=\"og:url\" content=\"http://www.example.com/\"><meta property=\"og:description\" content=\"desc..\">";
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(page.as_bytes()).unwrap();
    let page = encoder.finish().unwrap();
    let gzipped = |path: &str, zeros: Vec<u8>| {
        Route::new("GET", "*", path, 200)
            .header("Content-Type", "text/html")
            .header("Content-Encoding", "gzip")
            .body([page.clone(), zeros].concat())
    };
    let mut oversized = gzip_bomb(1024);
    oversized.extend_from_slice(b"invalid gzip member after the decoded limit");
    let large = gzipped("/", oversized);
    assert!(
        large.body.len() < fetch::MAX_BODY_SIZE,
        "the compressed body must fit, so rejection tests the inflated size"
    );
    let server = FakeServer::start(vec![large, gzipped("/small", gzip_bomb(2))]).await;
    let net = network_to(server.addr);

    assert!(matches!(
        crate::test_support::wait(
            "unfurling the small gzip page",
            unfurl(&net, "http://www.example.com/small")
        )
        .await,
        Ok(Unfurl::Json(_))
    ));
    // This fixture is 200 HTML with a compressed length below the limit. Only the inflated
    // size can produce Ok(None); a stalled body produces a transport error instead.
    let url = campfire_richtext::uri::parse("http://www.example.com/").unwrap();
    let fetched = crate::test_support::wait(
        "rejecting the inflated gzip size",
        fetch::fetch_document(&net, &url, "93.184.216.34".parse().unwrap()),
    )
    .await;
    assert!(
        matches!(fetched, Ok(None)),
        "expected inflated-size rejection, got {fetched:?}"
    );
    assert_eq!(
        crate::test_support::wait(
            "unfurling the oversized gzip page",
            unfurl(&net, "http://www.example.com/")
        )
        .await,
        Ok(Unfurl::NoContent)
    );
}

/// A server that keeps sending a byte at a time never trips a read timeout, but the unfurl as a
/// whole gives up.
#[test]
fn gives_up_on_a_trickling_page() {
    use futures_util::FutureExt;
    use std::time::Duration;

    // The watchdog uses wall time on a different thread; paused Tokio time cannot hide a hang.
    let (finished, result) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        let outcome = std::panic::catch_unwind(|| {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .start_paused(true)
                .build()
                .unwrap();
            runtime.block_on(async {
                // Prevent Tokio's idle-time auto-advance while waiting for real TCP readiness.
                let keep_time_paused = tokio::spawn(async {
                    loop {
                        tokio::task::yield_now().await;
                    }
                });
                let (server, ready) = trickling_server_with_ready("HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nConnection: close\r\n\r\n").await;
                let net = network_to(server);
                let started = tokio::time::Instant::now();
                let deadline = Duration::from_millis(500);
                let unfurling = unfurl_within(&net, "http://www.example.com/", deadline);
                tokio::pin!(unfurling);
                tokio::select! {
                    biased;
                    outcome = &mut unfurling => panic!("trickling fetch finished before its deadline: {outcome:?}"),
                    ready = ready => ready.expect("trickling server must send its first response byte"),
                }
                assert_eq!(started.elapsed(), Duration::ZERO, "clock advanced during TCP setup");

                tokio::time::advance(deadline - Duration::from_millis(1)).await;
                tokio::time::sleep_until(started + deadline - Duration::from_millis(1)).await;
                assert!(unfurling.as_mut().now_or_never().is_none(), "trickling fetch finished before 500 ms");
                tokio::time::advance(Duration::from_millis(1)).await;
                tokio::time::sleep_until(started + deadline).await;
                assert_eq!(unfurling.as_mut().now_or_never(), Some(Ok(Unfurl::NoContent)), "trickling fetch must expire at its 500 ms deadline");
                assert_eq!(started.elapsed(), deadline);
                keep_time_paused.abort();
            });
        });
        let _ = finished.send(outcome);
    });
    let outcome = result
        .recv_timeout(crate::test_support::WAIT)
        .expect("trickling deadline test exceeded its 30 s wall-clock watchdog");
    worker
        .join()
        .expect("deadline worker panicked outside its test");
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

// Regression tests for our fork's fetch contract (local sockets only).
#[tokio::test]
async fn ws15e_follows_relative_redirects() {
    let server = FakeServer::start_ws15e(vec![
        Route::new("GET", "www.example.com", "/start", 302).header("Location", "/final"),
        Route::new("GET", "www.example.com", "/final", 200).header("Content-Type", "text/html").body("ok"),
    ])
    .await;
    let url = campfire_richtext::uri::parse("http://www.example.com/start").unwrap();
    assert_eq!(
        fetch::fetch_document(&network_to(server.addr), &url, "93.184.216.34".parse().unwrap()).await.unwrap(),
        Some(b"ok".to_vec())
    );
}

#[tokio::test]
async fn ws15e_denies_missing_and_invalid_redirect_locations() {
    for location in [None, Some(""), Some(" "), Some("http://bad host/"), Some("javascript:alert(1)")] {
        let mut route = Route::new("GET", "www.example.com", "/", 302);
        if let Some(location) = location {
            route = route.header("Location", location);
        }
        let server = FakeServer::start_ws15e(vec![route]).await;
        let url = campfire_richtext::uri::parse("http://www.example.com/").unwrap();
        assert!(
            matches!(
                fetch::fetch_document(&network_to(server.addr), &url, "93.184.216.34".parse().unwrap()).await,
                Err(fetch::FetchError::RedirectDenied)
            ),
            "{location:?}"
        );
    }
}

#[tokio::test]
async fn ws15e_honors_zero_and_three_redirect_budgets() {
    let routes = (0..=4)
        .map(|i| {
            if i == 3 {
                Route::new("GET", "www.example.com", &format!("/{i}"), 200).header("Content-Type", "text/html").body("ok")
            } else {
                Route::new("GET", "www.example.com", &format!("/{i}"), 302).header("Location", &format!("http://www.example.com/{}", i + 1))
            }
        })
        .collect();
    let server = FakeServer::start_ws15e(routes).await;
    let net = network_to(server.addr);
    let ip = "93.184.216.34".parse().unwrap();
    let start = campfire_richtext::uri::parse("http://www.example.com/0").unwrap();
    let options = fetch::FetchOptions { max_redirects: 0, deadline: None };
    assert!(matches!(fetch::fetch_document_with(&net, &start, ip, options).await, Err(fetch::FetchError::TooManyRedirects)));
    assert_eq!(server.received().len(), 1);
    let options = fetch::FetchOptions { max_redirects: 3, deadline: None };
    assert_eq!(fetch::fetch_document_with(&net, &start, ip, options).await.unwrap(), Some(b"ok".to_vec()));
    assert_eq!(server.received().len(), 5);
}

#[test]
fn ws15e_deadline_covers_body_reads() {
    use futures_util::FutureExt;
    use std::time::Duration;
    crate::integrations::test_support::with_paused_time("OpenGraph body-read deadline", async {
        let (server, ready) = trickling_server_with_ready(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nConnection: close\r\n\r\n",
        ).await;
        let net = network_to(server);
        let url = campfire_richtext::uri::parse("http://www.example.com/").unwrap();
        let deadline = Duration::from_millis(150);
        let options = fetch::FetchOptions { max_redirects: 3, deadline: Some(deadline) };
        let started = tokio::time::Instant::now();
        let fetching = fetch::fetch_document_with(&net, &url, "93.184.216.34".parse().unwrap(), options);
        tokio::pin!(fetching);
        tokio::select! {
            biased;
            outcome = &mut fetching => panic!("trickling body fetch finished before its deadline: {outcome:?}"),
            ready = ready => ready.expect("trickling server must send its first response byte"),
        }
        assert_eq!(started.elapsed(), Duration::ZERO, "clock advanced during TCP setup");
        tokio::time::advance(deadline - Duration::from_millis(1)).await;
        tokio::time::sleep_until(started + deadline - Duration::from_millis(1)).await;
        assert!(fetching.as_mut().now_or_never().is_none(), "body fetch finished before 150 ms");
        tokio::time::advance(Duration::from_millis(1)).await;
        tokio::time::sleep_until(started + deadline).await;
        assert!(matches!(fetching.as_mut().now_or_never(), Some(Err(fetch::FetchError::Deadline))), "body fetch ignored its 150 ms deadline");
        assert_eq!(started.elapsed(), deadline);
    });
}

#[tokio::test]
async fn ws15e_pins_each_redirect_and_refuses_private_targets() {
    let server = FakeServer::start_ws15e(vec![
        Route::new("GET", "www.example.com", "/", 302).header("Location", "http://cdn.example.com/image"),
        Route::new("GET", "cdn.example.com", "/image", 200).header("Content-Type", "text/html").body("ok"),
        Route::new("GET", "www.example.com", "/private", 302).header("Location", "http://2130706433/secret"),
        Route::new("GET", "127.0.0.1", "/secret", 200).header("Content-Type", "text/html").body("secret"),
    ])
    .await;
    let resolver = Arc::new(FakeResolver::new([("www.example.com", vec!["93.184.216.34"])]));
    resolver.set("cdn.example.com", vec![vec!["93.184.216.35".parse().unwrap()], vec!["127.0.0.1".parse().unwrap()]]);
    let dialer = Arc::new(MappingDialer {
        public: HashSet::from(["93.184.216.34".parse().unwrap(), "93.184.216.35".parse().unwrap()]),
        to: server.addr,
        dialed: Mutex::new(Vec::new()),
    });
    let net = network(resolver.clone(), dialer.clone());
    let url = campfire_richtext::uri::parse("http://www.example.com/").unwrap();
    assert_eq!(fetch::fetch_document(&net, &url, "93.184.216.34".parse().unwrap()).await.unwrap(), Some(b"ok".to_vec()));
    assert_eq!(resolver.lookups(), ["cdn.example.com"]);
    let dialed: Vec<String> = dialer.dialed.lock().unwrap().iter().map(ToString::to_string).collect();
    assert_eq!(dialed, ["93.184.216.34:80", "93.184.216.35:80"]);
    let url = campfire_richtext::uri::parse("http://www.example.com/private").unwrap();
    assert!(matches!(fetch::fetch_document(&net, &url, "93.184.216.34".parse().unwrap()).await, Err(fetch::FetchError::Guard(_))));
    assert_eq!(server.received().len(), 3);
}

#[tokio::test]
async fn ws15e_rejects_private_redirect_before_dialing() {
    let server = FakeServer::start_ws15e(vec![
        Route::new("GET", "www.example.com", "/", 302).header("Location", "http://2130706433/secret"),
        Route::new("GET", "2130706433", "/secret", 200).header("Content-Type", "text/html").body("secret"),
    ])
    .await;
    let net = network_to(server.addr);
    let url = campfire_richtext::uri::parse("http://www.example.com/").unwrap();
    assert!(matches!(fetch::fetch_document(&net, &url, "93.184.216.34".parse().unwrap()).await, Err(fetch::FetchError::Guard(_))));
    assert_eq!(server.received().len(), 1);
}

#[test]
fn ws15e_retries_header_eof_once_unless_a_deadline_is_armed() {
    crate::integrations::test_support::with_paused_time("header EOF retry policy", async {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        for deadline in [None, Some(std::time::Duration::from_secs(1))] {
            let listener = crate::integrations::test_support::ws15e_listener().await;
            let address = listener.local_addr().unwrap();
            let count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let log = count.clone();
            let server = tokio::spawn(async move {
                while let Ok((mut stream, _)) = listener.accept().await {
                    let _ = stream.read(&mut [0; 4096]).await;
                    if log.fetch_add(1, std::sync::atomic::Ordering::SeqCst) > 0 {
                        let _ = stream
                            .write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok")
                            .await;
                    }
                    let _ = stream.shutdown().await;
                }
            });
            let url = campfire_richtext::uri::parse("http://www.example.com/").unwrap();
            let options = fetch::FetchOptions { max_redirects: 3, deadline };
            let result = fetch::fetch_document_with(&network_to(address), &url, "93.184.216.34".parse().unwrap(), options).await;
            if deadline.is_some() {
                assert!(result.is_err());
                assert_eq!(count.load(std::sync::atomic::Ordering::SeqCst), 1);
            } else {
                assert_eq!(result.unwrap(), Some(b"ok".to_vec()));
                assert_eq!(count.load(std::sync::atomic::Ordering::SeqCst), 2);
            }
            server.abort();
        }
    });
}

#[tokio::test]
async fn ws15e_composer_skips_github_and_fizzy_cards_without_dns() {
    let page = r#"<meta property="og:title" content="Title"><meta property="og:description" content="Description">"#;
    let server = FakeServer::start_ws15e(vec![
        Route::new("GET", "github.com", "/acme/repo/pull/12", 200).header("Content-Type", "text/html").body(page),
        Route::new("GET", "app.fizzy.do", "/account/cards/12", 200).header("Content-Type", "text/html").body(page),
    ])
    .await;
    let resolver = Arc::new(FakeResolver::new([("github.com", vec!["93.184.216.34"]), ("app.fizzy.do", vec!["93.184.216.34"])]));
    let dialer = Arc::new(MappingDialer {
        public: HashSet::from(["93.184.216.34".parse().unwrap()]),
        to: server.addr,
        dialed: Mutex::new(Vec::new()),
    });
    let net = network(resolver.clone(), dialer);
    for url in ["https://github.com/acme/repo/pull/12", "https://app.fizzy.do/account/cards/12"] {
        // Plain transport to the fake server suffices to prove the skip; a TLS failure
        // would also return no content, but would still resolve and connect.
        assert_eq!(unfurl(&net, url).await.unwrap(), Unfurl::NoContent);
    }
    assert_eq!(resolver.lookups(), Vec::<String>::new());
    assert!(server.received().is_empty());
}
