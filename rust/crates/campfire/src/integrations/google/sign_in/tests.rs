use super::*;
use std::time::Duration;
use crate::integrations::test_support::{FakeResolver, FakeServer, MappingDialer, Route};
use rustls::pki_types::{CertificateDer, pem::PemObject};
use std::sync::Arc;

fn vectors() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../../vectors/google_sign_in.json"
    ))
    .unwrap()
}
fn jwt_vectors() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../../vectors/rails_compat_smartfire.json"
    ))
    .unwrap()
}
fn config() -> Config {
    Config {
        client_id: "test-client-id".into(),
        client_secret: "FAKE-google-client-secret".into(),
        domains: vec!["smartdata.net".into()],
    }
}
async fn fake(routes: Vec<Route>) -> (SignIn, FakeServer) {
    let server = FakeServer::start_tls_with_ports(
        routes,
        include_bytes!("../test.pem"),
        include_bytes!("../test.key"),
        Some(53100..=53199),
    )
    .await;
    let resolver = Arc::new(FakeResolver::new([
        ("oauth2.googleapis.com", vec!["203.0.113.1"]),
        ("www.googleapis.com", vec!["203.0.113.1"]),
    ]));
    let dialer = Arc::new(MappingDialer {
        public: ["203.0.113.1".parse().unwrap()].into_iter().collect(),
        to: server.addr,
        dialed: Mutex::new(vec![]),
    });
    let mut roots = rustls::RootCertStore::empty();
    roots
        .add(CertificateDer::from_pem_slice(include_bytes!("../ca.pem")).unwrap())
        .unwrap();
    let network = Network {
        resolver,
        dialer,
        tls: crate::net::tls_config(roots),
    };
    (SignIn::new(config(), network), server)
}

#[test]
fn authorize_and_pkce_match_pinned_rails_bytes() {
    let v = vectors();
    let sign_in = SignIn::new(config(), Network::system());
    assert_eq!(
        pkce_challenge(v["verifier"].as_str().unwrap()),
        v["challenge"]
    );
    let secrets = Secrets::new(v["secret_key_base"].as_str().unwrap());
    assert_eq!(
        state_verifier(&secrets).generate(&v["raw_state"], None, None),
        v["signed_state"]
    );
    for purpose in ["sign_in", "link", "reauth", "sudo"] {
        assert_eq!(
            sign_in.authorize_url(
                v["redirect_uri"].as_str().unwrap(),
                v["signed_state"].as_str().unwrap(),
                v["nonce"].as_str().unwrap(),
                v["challenge"].as_str().unwrap(),
                purpose
            ),
            v["authorize"][purpose]
        );
    }
}

#[test]
fn safe_return_paths_match_pinned_rails() {
    for v in vectors()["return_paths"].as_array().unwrap() {
        assert_eq!(
            safe_return_path(v["input"].as_str(), "example.test"),
            v["output"].as_str().map(str::to_owned),
            "{v}"
        );
    }
}

#[test]
fn new_flow_is_browser_bound_random_and_step_up_is_fresh() {
    let sign_in = SignIn::new(config(), Network::system());
    let secrets = Secrets::new("FAKE-flow-secret");
    let now = Timestamp::from_second(1_800_000_000).unwrap();
    let (flow, url) = sign_in.start(
        &secrets,
        "https://chat.example.test/session/google/callback",
        "sudo",
        Some(42),
        now,
    );
    let (_, second) = sign_in.start(
        &secrets,
        "https://chat.example.test/session/google/callback",
        "sudo",
        Some(42),
        now,
    );
    assert_ne!(url, second);
    assert_eq!(flow["state"].as_str().unwrap().len(), 32);
    assert_eq!(flow["nonce"].as_str().unwrap().len(), 32);
    assert_eq!(flow["verifier"].as_str().unwrap().len(), 43);
    assert_eq!(flow["exp"], now.as_second() + 600);
    assert_eq!(flow["user_id"], 42);
    let query: Map<_, _> = url::form_urlencoded::parse(url.split_once('?').unwrap().1.as_bytes())
        .map(|(k, v)| (k.into_owned(), Value::String(v.into_owned())))
        .collect();
    assert_eq!(query["prompt"], "login");
    assert_eq!(query["max_age"], "0");
    assert_eq!(
        query["code_challenge"],
        pkce_challenge(flow["verifier"].as_str().unwrap())
    );
    assert!(valid_flow(
        &flow,
        query["state"].as_str().unwrap(),
        &secrets,
        now
    ));
}

#[test]
fn security_state_mismatch_wrong_signature_expiry_and_odd_session_shapes_fail_closed() {
    let v = vectors();
    let secrets = Secrets::new(v["secret_key_base"].as_str().unwrap());
    let now = Timestamp::from_second(1_800_000_000).unwrap();
    let signed = v["signed_state"].as_str().unwrap();
    let flow = json!({"state":v["raw_state"],"nonce":v["nonce"],"verifier":v["verifier"],"exp":now.as_second()+600,"purpose":"sign_in"});
    assert!(valid_flow(&flow, signed, &secrets, now));
    let mut wrong = flow.clone();
    wrong["state"] = "another-browser".into();
    assert!(!valid_flow(&wrong, signed, &secrets, now), "state mismatch");
    assert!(
        !valid_flow(&flow, "forged-state", &secrets, now),
        "signature mismatch"
    );
    assert!(!valid_flow(
        &flow,
        signed,
        &Secrets::new("wrong-secret"),
        now
    ));
    let expired = Timestamp::from_second(now.as_second() + 600).unwrap();
    assert!(
        !valid_flow(&flow, signed, &secrets, expired),
        "exact expiry boundary"
    );
    for key in ["state", "nonce", "verifier", "exp"] {
        let mut wrong = flow.clone();
        wrong[key] = Value::Null;
        assert!(!valid_flow(&wrong, signed, &secrets, now), "missing {key}");
    }
    let mut float_exp = flow.clone();
    float_exp["exp"] = json!(1_800_000_600.0);
    assert!(!valid_flow(&float_exp, signed, &secrets, now));
    assert!(!valid_flow(&Value::Null, signed, &secrets, now));
}

#[test]
fn configuration_has_no_implicit_company_domains() {
    let mut c = config();
    assert!(c.configured());
    c.domains.clear();
    assert!(!c.configured());
    c = config();
    c.client_id = " \t".into();
    assert!(!c.configured());
    c = config();
    c.client_secret = "".into();
    assert!(!c.configured());
}

#[tokio::test]
async fn exchange_posts_rails_form_and_never_returns_access_or_refresh_tokens() {
    let (client, server) = fake(vec![Route::new("POST", "oauth2.googleapis.com", "/token", 200).body(json!({"id_token":"the-id-token","access_token":"FAKE-access","refresh_token":"FAKE-refresh"}).to_string())]).await;
    assert_eq!(
        client
            .exchange_code("code +/&", "http://test.host/callback", "verifier")
            .await
            .unwrap(),
        "the-id-token"
    );
    let r = &server.received()[0];
    assert_eq!(r.method, "POST");
    assert_eq!(r.target, "/token");
    assert_eq!(
        r.header("Content-Type"),
        Some("application/x-www-form-urlencoded")
    );
    assert_eq!(
        String::from_utf8(r.body.clone()).unwrap(),
        "client_id=test-client-id&client_secret=FAKE-google-client-secret&code=code+%2B%2F%26&redirect_uri=http%3A%2F%2Ftest.host%2Fcallback&grant_type=authorization_code&code_verifier=verifier"
    );
}

#[tokio::test]
async fn exchange_denial_bad_tokens_and_invalid_json_are_distinct() {
    for (status, body, expected) in [
        (
            400,
            "{\"error\":\"invalid_grant\"}",
            Error::Rejected("denied"),
        ),
        (503, "unavailable", Error::Rejected("denied")),
        (200, "not-json", Error::Unavailable),
        (200, "[]", Error::Unavailable),
        (200, "{}", Error::Rejected("bad_token")),
        (200, "{\"id_token\":123}", Error::Rejected("bad_token")),
        (200, "{\"id_token\":\" \"}", Error::Rejected("bad_token")),
    ] {
        let (client, _server) = fake(vec![
            Route::new("POST", "oauth2.googleapis.com", "/token", status).body(body),
        ])
        .await;
        assert_eq!(
            client
                .exchange_code("code", "http://test.host/callback", "v")
                .await,
            Err(expected)
        );
    }
}

#[tokio::test]
async fn real_socket_read_timeouts_are_unavailable_for_both_google_hosts() {
    let mut token = Route::new("POST", "oauth2.googleapis.com", "/token", 200)
        .body(json!({"id_token":"would-succeed-after-the-timeout"}).to_string());
    let mut keys = Route::new("GET", "www.googleapis.com", "/oauth2/v3/certs", 200)
        .body(jwt_vectors()["jwt"]["google"]["jwks"].as_str().unwrap());
    token.delay = Duration::from_secs(11);
    keys.delay = Duration::from_secs(11);
    let (client, _server) = fake(vec![token, keys]).await;
    let started = std::time::Instant::now();
    let (token, key) = tokio::join!(
        client.exchange_code("code", "http://test.host/callback", "verifier"),
        client.public_key_for("kid-1", 1_800_000_000),
    );
    assert_eq!(token, Err(Error::Unavailable));
    assert_eq!(key, Err(Error::Unavailable));
    // An immediate TLS/fixture failure must not make this timeout test pass.
    assert!(started.elapsed() >= Duration::from_secs(9));
}

#[tokio::test]
async fn security_verified_tokens_match_all_pinned_rails_vectors() {
    let v = jwt_vectors();
    let g = &v["jwt"]["google"];
    let (mut client, _server) = fake(vec![
        Route::new("GET", "www.googleapis.com", "/oauth2/v3/certs", 200)
            .body(g["jwks"].as_str().unwrap()),
    ])
    .await;
    client.config.client_id = g["client_id"].as_str().unwrap().into();
    client.config.domains = g["allowed_domains"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d.as_str().unwrap().to_owned())
        .collect();
    let now: Timestamp = v["now"].as_str().unwrap().parse().unwrap();
    let mut failures = vec![];
    for case in g["verify"].as_array().unwrap() {
        // WS1's accepted exception: RSA keys shorter than 2048 bits fail closed.
        if case["case"] == "1024-bit key" {
            let actual = client
                .verify(
                    case["token"].as_str().unwrap(),
                    case["nonce"].as_str().unwrap(),
                    "sign_in",
                    now.as_second(),
                )
                .await;
            if actual != Err(Error::Rejected("bad_token")) {
                failures.push(case["case"].as_str().unwrap().to_owned());
            }
            continue;
        }
        let purpose = if case["max_auth_age"].is_null() {
            "sign_in"
        } else {
            "sudo"
        };
        let actual = client
            .verify(
                case["token"].as_str().unwrap(),
                case["nonce"].as_str().unwrap(),
                purpose,
                now.as_second(),
            )
            .await;
        if let Some(claims) = case["claims"].as_object() {
            if actual.as_ref().ok() != Some(claims) {
                failures.push(case["case"].as_str().unwrap().to_owned());
            }
        } else {
            if !matches!(actual, Err(Error::Rejected(reason)) if Some(reason) == case["rejected"].as_str())
            {
                failures.push(case["case"].as_str().unwrap().to_owned());
            }
        }
    }
    assert!(
        failures.is_empty(),
        "security vector mismatches: {failures:?}"
    );
    eprintln!(
        "Google ID-token vectors: {} exercised; 0 skipped; 1 accepted fail-closed RSA exception",
        g["verify"].as_array().unwrap().len()
    );
}

#[tokio::test]
async fn key_cache_expires_at_one_hour_and_unknown_kid_refetches_once() {
    let v = jwt_vectors();
    let jwks = v["jwt"]["google"]["jwks"].as_str().unwrap();
    let (client, server) = fake(vec![
        Route::new("GET", "www.googleapis.com", "/oauth2/v3/certs", 200).body(jwks),
    ])
    .await;
    let now = 1_800_000_000;
    client.public_key_for("kid-1", now).await.unwrap();
    client.public_key_for("kid-1", now + 3599).await.unwrap();
    assert_eq!(server.received().len(), 1);
    assert_eq!(
        client.public_key_for("unknown", now + 3599).await,
        Err(Error::Rejected("unknown_key"))
    );
    assert_eq!(server.received().len(), 2);
    client.public_key_for("kid-1", now + 7199).await.unwrap();
    assert_eq!(server.received().len(), 3);
}

#[tokio::test]
async fn empty_cache_unknown_kid_fetches_twice_and_bad_jwks_are_unavailable() {
    let v = jwt_vectors();
    let jwks = v["jwt"]["google"]["jwks"].as_str().unwrap();
    let (client, server) = fake(vec![
        Route::new("GET", "www.googleapis.com", "/oauth2/v3/certs", 200).body(jwks),
    ])
    .await;
    assert_eq!(
        client.public_key_for("unknown", 1_800_000_000).await,
        Err(Error::Rejected("unknown_key"))
    );
    assert_eq!(server.received().len(), 2);
    for (status, body) in [(503, "outage"), (200, "nope"), (200, "{\"keys\":[]}")] {
        let (client, _server) = fake(vec![
            Route::new("GET", "www.googleapis.com", "/oauth2/v3/certs", status).body(body),
        ])
        .await;
        assert_eq!(
            client.public_key_for("kid-1", 1_800_000_000).await,
            Err(Error::Unavailable)
        );
    }
}
