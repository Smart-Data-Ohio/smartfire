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
    let public: HashSet<std::net::IpAddr> = spec["public_ips"].as_array().unwrap().iter().map(|ip| ip.as_str().unwrap().parse().unwrap()).collect();

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
            .map(|r| serde_json::json!([r.method, r.header("host"), r.target, r.header("accept"), r.header("accept-encoding"), r.header("user-agent")]))
            .collect();
        let actual = serde_json::json!({ "response": response, "lookups": resolver.lookups(), "requests": requests });
        let wanted = serde_json::json!({ "response": expected["response"], "lookups": expected["lookups"], "requests": expected["requests"] });
        if actual != wanted {
            failures.push(format!("{name}:\n  expected {wanted}\n  actual   {actual}"));
        }
    }
    assert!(failures.is_empty(), "{} of {} cases differ:\n{}", failures.len(), spec["cases"].as_array().unwrap().len(), failures.join("\n"));
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
    let dialer = Arc::new(MappingDialer { public: HashSet::from(["93.184.216.34".parse().unwrap()]), to: server, dialed: Mutex::new(Vec::new()) });
    network(resolver, dialer)
}

/// A page followed by a gigabyte of zeros, gzipped to a megabyte, is past the 5MB limit as soon
/// as that much is inflated. The same page followed by less unfurls.
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
    let large = gzipped("/", gzip_bomb(1024));
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
