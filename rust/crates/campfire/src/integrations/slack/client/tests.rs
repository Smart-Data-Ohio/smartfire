use std::net::Ipv4Addr;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use super::*;
use crate::net::tls_config;
use crate::integrations::test_support::{FakeResolver, FakeServer, MappingDialer, Route};

pub(crate) async fn fake(routes: Vec<Route>) -> (FakeServer, Network) {
    let (mut servers, network) = fake_responses(vec![routes]).await;
    (servers.remove(0), network)
}
async fn fake_responses(responses: Vec<Vec<Route>>) -> (Vec<FakeServer>, Network) {
    use rustls::pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer};
    let rcgen::CertifiedKey { cert, signing_key } =
        rcgen::generate_simple_self_signed(vec![HOST.to_owned()]).unwrap();
    let mut roots = rustls::RootCertStore::empty();
    roots.add(cert.der().clone()).unwrap();
    let config = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .unwrap()
    .with_no_client_auth()
    .with_single_cert(
        vec![cert.der().clone()],
        PrivateKeyDer::from(PrivatePkcs8KeyDer::from(signing_key.serialize_der())),
    )
    .unwrap();
    let config = Arc::new(config);
    let mut servers = Vec::new();
    for routes in responses {
        let mut listener = None;
        for port in 53300..=53399 {
            if let Ok(bound) = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, port)).await {
                listener = Some(bound);
                break;
            }
        }
        servers.push(
            FakeServer::on_listener(
                routes,
                Some(tokio_rustls::TlsAcceptor::from(config.clone())),
                listener.expect("WS16 ports exhausted"),
            )
            .await,
        );
    }
    struct Responses {
        addresses: Vec<std::net::SocketAddr>,
        next: AtomicUsize,
    }
    impl crate::net::Dialer for Responses {
        fn connect(
            &self,
            addr: std::net::SocketAddr,
        ) -> crate::net::BoxFuture<'_, std::io::Result<tokio::net::TcpStream>>
        {
            assert_eq!(
                addr.ip(),
                "203.0.113.16".parse::<std::net::IpAddr>().unwrap()
            );
            let index = self
                .next
                .fetch_add(1, Ordering::SeqCst)
                .min(self.addresses.len() - 1);
            Box::pin(tokio::net::TcpStream::connect(self.addresses[index]))
        }
    }
    let network = Network {
        resolver: Arc::new(FakeResolver::new([(HOST, vec!["203.0.113.16"])])),
        dialer: Arc::new(Responses {
            addresses: servers.iter().map(|server| server.addr).collect(),
            next: AtomicUsize::new(0),
        }),
        tls: tls_config(roots),
    };
    (servers, network)
}

fn route(path: &str, status: u16, body: &str) -> Route {
    Route::new("GET", HOST, path, status).body(body.to_owned())
}
fn client(network: Network) -> Client {
    Client::with_network("fixture-slack-user-token".into(), None, false, network)
}

#[tokio::test]
async fn slack_client_malformed_http_bodies_and_json_comments_match_rails() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../../../vectors/slack/payloads.json"
    ))
    .unwrap();
    let cases = oracle["http"].as_array().unwrap();
    for row in cases {
        let (server, network) = fake(vec![route(
            "/api/users.list?limit=200",
            row["status"].as_u64().unwrap() as u16,
            row["body"].as_str().unwrap(),
        )])
        .await;
        match client(network).users_list(None, 200).await {
            Ok(value) => assert_eq!(value, row["expected"]["result"]),
            Err(error) => {
                assert_eq!(error.ruby_class(), row["expected"]["class"], "{row}");
                assert_eq!(error.message, row["expected"]["message"], "{row}");
            }
        }
        assert_eq!(
            server.received().len(),
            row["attempts"].as_u64().unwrap() as usize
        );
    }
    println!(
        "Slack malformed HTTP parity: {} Rails status/body/retry cases matched",
        cases.len()
    );
}

#[test]
fn slack_client_transport_classes_messages_and_retryability_match_rails() {
    use crate::net::http::HttpError;
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../../../vectors/slack/payloads.json"
    ))
    .unwrap();
    for row in oracle["transport"].as_array().unwrap() {
        let message = row["input_message"].as_str().unwrap();
        let io = |kind, s: &str| HttpError::Io(std::io::Error::new(kind, s.to_owned()));
        let error = match row["input_class"].as_str().unwrap() {
            "IOError" => io(std::io::ErrorKind::Other, message),
            "EOFError" => io(std::io::ErrorKind::UnexpectedEof, message),
            "SocketError" => HttpError::Unresolvable(message.into()),
            "Net::OpenTimeout" => HttpError::OpenTimeout,
            "Net::ReadTimeout" => HttpError::ReadTimeout,
            "Net::WriteTimeout" => HttpError::WriteTimeout,
            "Timeout::Error" => io(std::io::ErrorKind::TimedOut, message),
            "Errno::ECONNRESET" => io(std::io::ErrorKind::ConnectionReset, "fixture reset"),
            "Errno::ECONNREFUSED" => io(std::io::ErrorKind::ConnectionRefused, "fixture refused"),
            "Errno::EPIPE" => io(std::io::ErrorKind::BrokenPipe, "fixture pipe"),
            "OpenSSL::SSL::SSLError" => HttpError::Tls(message.into()),
            _ => unreachable!(),
        };
        let mut error = network_error("users.list", error);
        if error.kind == ErrorKind::Network {
            error.kind = ErrorKind::Request;
        }
        assert_eq!(error.message, row["expected"]["message"]);
        assert_eq!(error.ruby_class(), row["expected"]["class"]);
        assert_eq!(
            row["attempts"],
            if matches!(error.kind, ErrorKind::Exception(_)) {
                1
            } else {
                4
            }
        );
    }
}

#[tokio::test]
async fn slack_client_network_final_message_matches_rails_through_actual_dialer() {
    struct Fail(AtomicUsize);
    impl crate::net::Dialer for Fail {
        fn connect(
            &self,
            _: std::net::SocketAddr,
        ) -> crate::net::BoxFuture<'_, std::io::Result<tokio::net::TcpStream>>
        {
            self.0.fetch_add(1, Ordering::SeqCst);
            Box::pin(async { Err(std::io::Error::other("fixture io")) })
        }
    }
    let (_server, mut network) = fake(vec![]).await;
    let dialer = Arc::new(Fail(AtomicUsize::new(0)));
    network.dialer = dialer.clone();
    let error = client(network).users_list(None, 200).await.unwrap_err();
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../../../vectors/slack/payloads.json"
    ))
    .unwrap();
    assert_eq!(error.message, oracle["transport"][0]["expected"]["message"]);
    assert_eq!(dialer.0.load(Ordering::SeqCst), 4);
}

#[test]
fn slack_client_matches_rails_error_vectors() {
    let corpus: Value =
        serde_json::from_str(include_str!("../../../../../../vectors/slack/client.json")).unwrap();
    for entry in corpus["cases"].as_array().unwrap() {
        let expected = &entry["expected"];
        match check_ok(entry["payload"].clone(), "users.list") {
            Ok(payload) => assert_eq!(payload, expected["payload"]),
            Err(error) => {
                let kind = match error.kind {
                    ErrorKind::Auth => "AuthError",
                    ErrorKind::Scope => "ScopeError",
                    ErrorKind::RateLimited => "RateLimited",
                    ErrorKind::Http => "HttpError",
                    _ => "RequestError",
                };
                assert_eq!(kind, expected["kind"]);
                assert_eq!(error.message, expected["message"]);
                if error.kind == ErrorKind::Scope {
                    assert_eq!(*error.needed, expected["needed"]);
                    assert_eq!(*error.provided, expected["provided"]);
                }
                if error.kind == ErrorKind::RateLimited {
                    assert_eq!(error.retry_after, expected["retry_after"].as_u64());
                }
            }
        }
    }
}

#[tokio::test]
async fn slack_client_fixture_endpoints_queries_user_token_and_archived_rooms() {
    let routes = vec![
        route("/api/auth.test?", 200, r#"{"ok":true,"user_id":"UADMIN"}"#),
        route(
            "/api/team.info?",
            200,
            r#"{"ok":true,"team":{"name":"Smart Data"}}"#,
        ),
        route(
            "/api/users.list?cursor=abc&limit=200",
            200,
            include_str!("../fixtures/users.json"),
        ),
        route(
            "/api/conversations.list?types=public_channel%2Cprivate_channel&exclude_archived=false&limit=200",
            200,
            include_str!("../fixtures/conversations_workspace.json"),
        ),
        route(
            "/api/conversations.members?channel=CCHAN&limit=1000",
            200,
            include_str!("../fixtures/members_CCHAN.json"),
        ),
        route(
            "/api/conversations.history?channel=CCHAN&oldest=1700000000.000000&latest=1700000060.000000&cursor=cchan-page-2&limit=200",
            200,
            include_str!("../fixtures/history_CCHAN_p2.json"),
        ),
        route(
            "/api/conversations.replies?channel=CCHAN&ts=1700000002.000002&limit=200",
            200,
            include_str!("../fixtures/replies_CCHAN_parent.json"),
        ),
    ];
    let (server, network) = fake(routes).await;
    let mut c = client(network);
    assert_eq!(c.auth_test().await.unwrap()["user_id"], "UADMIN");
    assert_eq!(c.team_info().await.unwrap()["team"]["name"], "Smart Data");
    assert_eq!(
        c.users_list(Some("abc"), 200).await.unwrap()["members"]
            .as_array()
            .unwrap()
            .len(),
        9
    );
    assert_eq!(
        c.conversations_list("public_channel,private_channel", None, 200)
            .await
            .unwrap()["channels"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    assert_eq!(
        c.conversations_members("CCHAN", None, 1000).await.unwrap()["members"],
        serde_json::json!(["UADMIN", "U001", "U002", "U003"])
    );
    assert_eq!(
        c.conversations_history(
            "CCHAN",
            Bounds {
                oldest: Some("1700000000.000000"),
                latest: Some("1700000060.000000")
            },
            Some("cchan-page-2"),
            200
        )
        .await
        .unwrap()["messages"]
            .as_array()
            .unwrap()
            .len(),
        5
    );
    assert_eq!(
        c.conversations_replies("CCHAN", "1700000002.000002", Bounds::default(), None, 200)
            .await
            .unwrap()["messages"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    assert_eq!(server.received().len(), 7);
    for request in server.received() {
        assert_eq!(
            request.header("Authorization"),
            Some(format!("Bearer {}", "fixture-slack-user-token").as_str())
        );
        assert_eq!(request.header("User-Agent"), Some("Smartfire-Slack-Import"));
    }
}

#[tokio::test]
async fn slack_client_429_does_not_retry_and_maps_retry_after() {
    for (header, expected) in [
        ("17", 17),
        ("0", 60),
        ("-2", 60),
        ("garbage", 60),
        ("17 seconds", 17),
    ] {
        let (server, network) = fake(vec![
            route("/api/users.list?limit=200", 429, "").header("Retry-After", header),
        ])
        .await;
        let error = client(network).users_list(None, 200).await.unwrap_err();
        assert_eq!(error.kind, ErrorKind::RateLimited);
        assert_eq!(error.retry_after, Some(expected));
        assert_eq!(server.received().len(), 1);
    }
}

#[tokio::test]
async fn slack_client_retries_5xx_and_invalid_json_four_times_with_callback_per_attempt() {
    for (status, body) in [(500, "boom"), (200, "invalid-json"), (302, "redirect")] {
        let (server, network) = fake(vec![route("/api/users.list?limit=200", status, body)]).await;
        let calls = Arc::new(AtomicUsize::new(0));
        let count = calls.clone();
        let mut c = Client::with_network(
            "fixture-token".into(),
            Some(Arc::new(move |method| {
                assert_eq!(method, "users.list");
                count.fetch_add(1, Ordering::SeqCst);
            })),
            false,
            network,
        );
        assert_eq!(
            c.users_list(None, 200).await.unwrap_err().kind,
            ErrorKind::Http
        );
        assert_eq!(server.received().len(), 4);
        assert_eq!(calls.load(Ordering::SeqCst), 4);
    }
}

#[tokio::test]
async fn slack_client_maps_non_200_json_errors_without_retrying() {
    for status in [200, 400, 401, 403, 404] {
        let (server, network) = fake(vec![route(
            "/api/users.list?limit=200",
            status,
            r#"{"ok":false,"error":"invalid_auth"}"#,
        )])
        .await;
        assert_eq!(
            client(network)
                .users_list(None, 200)
                .await
                .unwrap_err()
                .kind,
            ErrorKind::Auth
        );
        assert_eq!(server.received().len(), 1);
    }
    let (server, network) = fake(vec![route("/api/users.list?limit=200", 404, "not found")]).await;
    let error = client(network).users_list(None, 200).await.unwrap_err();
    assert_eq!(error.kind, ErrorKind::Request);
    assert_eq!(error.message, "Slack HTTP 404 for users.list");
    assert_eq!(server.received().len(), 1);
}

#[tokio::test(start_paused = true)]
async fn slack_client_pacing_is_shared_per_tier_and_separate_between_tiers() {
    let (_, network) = fake(vec![]).await;
    let mut c = client(network);
    c.pacing = true;
    c.pace(Tier::Two).await;
    let first = Instant::now();
    c.pace(Tier::Three).await;
    assert_eq!(first.elapsed(), Duration::ZERO);
    c.pace(Tier::Two).await;
    assert!(first.elapsed() >= Tier::Two.interval());
    let first = Instant::now();
    c.pace(Tier::Three).await;
    c.pace(Tier::Three).await;
    assert!(first.elapsed() >= Tier::Three.interval());
}

#[test]
fn slack_client_vendored_fixtures_match_our_rails_files() {
    let root = campfire_db::fixtures::reference_root();
    for entry in std::fs::read_dir(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/integrations/slack/fixtures"
    ))
    .unwrap()
    {
        let path = entry.unwrap().path();
        assert_eq!(
            std::fs::read(&path).unwrap(),
            std::fs::read(
                root.join("test/fixtures/files/slack")
                    .join(path.file_name().unwrap())
            )
            .unwrap()
        );
    }
}

#[tokio::test]
async fn slack_client_network_failure_retries_four_times_then_maps_to_request_error() {
    let (server, mut network) = fake(vec![]).await;
    let dialer = Arc::new(MappingDialer {
        public: Default::default(),
        to: server.addr,
        dialed: Default::default(),
    });
    network.dialer = dialer.clone();
    let error = client(network).users_list(None, 200).await.unwrap_err();
    assert_eq!(error.kind, ErrorKind::Request);
    assert_eq!(dialer.dialed.lock().unwrap().len(), 4);
    assert!(server.received().is_empty());
}

#[tokio::test]
async fn slack_client_recovering_5xx_counts_each_attempt_and_returns_payload() {
    let (servers, network) = fake_responses(vec![
        vec![route(
            "/api/users.list?limit=200",
            503,
            "temporarily unavailable",
        )],
        vec![route(
            "/api/users.list?limit=200",
            502,
            "gateway unavailable",
        )],
        vec![route(
            "/api/users.list?limit=200",
            200,
            r#"{"ok":true,"members":[{"id":"URECOVERED"}]}"#,
        )],
    ])
    .await;
    let calls = Arc::new(AtomicUsize::new(0));
    let observed = calls.clone();
    let mut client = Client::with_network(
        "fixture-recovery-grant".into(),
        Some(Arc::new(move |method| {
            assert_eq!(method, "users.list");
            observed.fetch_add(1, Ordering::SeqCst);
        })),
        false,
        network,
    );
    assert_eq!(
        client.users_list(None, 200).await.unwrap()["members"][0]["id"],
        "URECOVERED"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 3);
    assert_eq!(client.request_count(), 3);
    assert!(servers.iter().all(|server| server.received().len() == 1));
}
#[tokio::test]
async fn slack_client_recovering_network_counts_failed_dials_and_success_once() {
    struct RetryDialer {
        inner: Arc<dyn crate::net::Dialer>,
        count: AtomicUsize,
    }
    impl crate::net::Dialer for RetryDialer {
        fn connect(
            &self,
            addr: std::net::SocketAddr,
        ) -> crate::net::BoxFuture<'_, std::io::Result<tokio::net::TcpStream>>
        {
            if self.count.fetch_add(1, Ordering::SeqCst) < 2 {
                Box::pin(async {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::Interrupted,
                        "fixture retry",
                    ))
                })
            } else {
                self.inner.connect(addr)
            }
        }
    }
    let (server, mut network) = fake(vec![route(
        "/api/team.info?",
        200,
        r#"{"ok":true,"team":{"name":"Recovered"}}"#,
    )])
    .await;
    let dialer = Arc::new(RetryDialer {
        inner: network.dialer.clone(),
        count: AtomicUsize::new(0),
    });
    network.dialer = dialer.clone();
    let calls = Arc::new(AtomicUsize::new(0));
    let observed = calls.clone();
    let mut client = Client::with_network(
        "fixture-recovery-grant".into(),
        Some(Arc::new(move |method| {
            assert_eq!(method, "team.info");
            observed.fetch_add(1, Ordering::SeqCst);
        })),
        false,
        network,
    );
    assert_eq!(
        client.team_info().await.unwrap()["team"]["name"],
        "Recovered"
    );
    assert_eq!(dialer.count.load(Ordering::SeqCst), 3);
    assert_eq!(calls.load(Ordering::SeqCst), 3);
    assert_eq!(client.request_count(), 3);
    assert_eq!(server.received().len(), 1);
}
#[tokio::test]
async fn slack_client_history_recording_is_newest_first_on_each_page() {
    let (server, network) = fake(vec![
        route(
            "/api/conversations.history?channel=CCHAN&limit=200",
            200,
            include_str!("../fixtures/history_CCHAN_p1.json"),
        ),
        route(
            "/api/conversations.history?channel=CCHAN&cursor=cchan-page-2&limit=200",
            200,
            include_str!("../fixtures/history_CCHAN_p2.json"),
        ),
    ])
    .await;
    let mut client = client(network);
    let mut previous = None;
    for cursor in [None, Some("cchan-page-2")] {
        let page = client
            .conversations_history("CCHAN", Bounds::default(), cursor, 200)
            .await
            .unwrap();
        for message in page["messages"].as_array().unwrap() {
            let stamp = message["ts"].as_str().unwrap().parse::<f64>().unwrap();
            if let Some(previous) = previous {
                assert!(
                    previous >= stamp,
                    "Slack history changed newest-first order"
                );
            }
            previous = Some(stamp);
        }
    }
    assert_eq!(server.received().len(), 2);
}
