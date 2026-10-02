use super::oauth;
use jiff::Timestamp;
use rails_compat::{Secrets, app_verifier};
use serde_json::{Value, json};

/// Exercise queue assertions with the unrelated job main's recurring scheduler enqueues.
/// Use the real durable sink so the fixture cannot pass without a persisted job row.
pub(crate) fn enqueue_retention_job(tx: &mut campfire_db::Tx<'_>) -> campfire_db::Result<()> {
    let count = |tx: &campfire_db::Tx<'_>| {
        tx.conn().query_row(
            "SELECT COUNT(*) FROM background_jobs WHERE job_class='Retention::PruneJob'",
            [],
            |row| row.get::<_, i64>(0),
        )
    };
    let before = count(tx)?;
    tx.emit_after_commit(campfire_db::Event::job(
        &campfire_db::models::retention::PruneJob {},
    ));
    assert_eq!(count(tx)?, before + 1, "the recurring job must be persisted");
    Ok(())
}

fn now() -> Timestamp {
    "2026-01-01T12:00:00Z".parse().unwrap()
}
fn secrets() -> Secrets {
    Secrets::new(&"a".repeat(128))
}

#[test]
fn github_state_rejects_tampering() {
    let secrets = secrets();
    let signed = oauth::sign_state(&secrets, "session-state");
    let mut tampered = signed.clone();
    tampered.pop();
    tampered.push(if signed.ends_with('0') { '1' } else { '0' });
    assert!(!oauth::valid_state(
        &secrets,
        &tampered,
        Some(&json!("session-state")),
        now()
    ));
}

#[test]
fn github_state_rejects_wrong_verifier_and_purpose() {
    let secrets = secrets();
    for (name, purpose) in [
        ("slack_oauth_state", None),
        ("github_app_oauth_state", Some("other")),
    ] {
        let signed = app_verifier(&secrets, name).generate(&json!("session-state"), purpose, None);
        assert!(!oauth::valid_state(
            &secrets,
            &signed,
            Some(&json!("session-state")),
            now()
        ));
    }
}

#[test]
fn github_state_rejects_expired_tokens_and_mismatched_sessions() {
    let secrets = secrets();
    let expired = app_verifier(&secrets, "github_app_oauth_state").generate(
        &json!("session-state"),
        None,
        Some(now()),
    );
    assert!(!oauth::valid_state(
        &secrets,
        &expired,
        Some(&json!("session-state")),
        now()
    ));
    let signed = oauth::sign_state(&secrets, "session-state");
    for stored in [
        None,
        Some(Value::Null),
        Some(json!(true)),
        Some(json!("other-session")),
        Some(json!(["session-state"])),
    ] {
        assert!(!oauth::valid_state(
            &secrets,
            &signed,
            stored.as_ref(),
            now()
        ));
    }
    let non_string = app_verifier(&secrets, "github_app_oauth_state").generate(
        &json!(["session-state"]),
        None,
        None,
    );
    assert!(!oauth::valid_state(
        &secrets,
        &non_string,
        Some(&json!("session-state")),
        now()
    ));
}

use super::client::{API_VERSION, AppClient, ErrorKind, PullRequestKey, ReadClient, WriteClient};
use crate::integrations::{
    net::{self, Network},
    test_support::{FakeResolver, FakeServer, MappingDialer, Route},
};
use std::{
    collections::HashSet,
    net::IpAddr,
    sync::{Arc, Mutex},
};

fn vectors() -> Value {
    serde_json::from_str(include_str!("../../../../../vectors/github.json")).unwrap()
}

pub(crate) async fn fake(routes: Vec<Route>) -> (FakeServer, Network) {
    use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
    let rcgen::CertifiedKey { cert, signing_key } =
        rcgen::generate_simple_self_signed(vec!["github.com".into(), "api.github.com".into()])
            .unwrap();
    let cert = CertificateDer::from(cert.der().to_vec());
    let key = PrivateKeyDer::from(PrivatePkcs8KeyDer::from(signing_key.serialize_der()));
    let mut roots = rustls::RootCertStore::empty();
    roots.add(cert.clone()).unwrap();
    let config = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .unwrap()
    .with_no_client_auth()
    .with_single_cert(vec![cert], key)
    .unwrap();
    let mut listener = None;
    // Allow workers merging this fixture to stay within their reserved port range.
    let (first_port, last_port) = std::env::var("GITHUB_TEST_PORT_RANGE")
        .map(|range| {
            let (first, last) = range.split_once('-').expect("GITHUB_TEST_PORT_RANGE must be FIRST-LAST");
            let first: u16 = first.parse().expect("invalid first GitHub test port");
            let last: u16 = last.parse().expect("invalid last GitHub test port");
            assert!(first > 0 && first <= last, "invalid GitHub test port range");
            (first, last)
        })
        .unwrap_or((51500, 51549));
    for port in first_port..=last_port {
        let socket = tokio::net::TcpSocket::new_v4().unwrap();
        socket.set_reuseaddr(true).unwrap();
        if let Ok(bound) = socket
            .bind(std::net::SocketAddr::from(([127, 0, 0, 1], port)))
            .and_then(|()| socket.listen(128))
        {
            listener = Some(bound);
            break;
        }
    }
    let server = FakeServer::on_listener(
        routes,
        Some(tokio_rustls::TlsAcceptor::from(Arc::new(config))),
        listener.expect("GitHub test port range exhausted"),
    )
    .await;
    let ip: IpAddr = "93.184.216.34".parse().unwrap();
    let resolver = Arc::new(FakeResolver::new([
        ("github.com", vec!["93.184.216.34"]),
        ("api.github.com", vec!["93.184.216.34"]),
    ]));
    let dialer = Arc::new(MappingDialer {
        public: HashSet::from([ip]),
        to: server.addr,
        dialed: Mutex::new(Vec::new()),
    });
    (
        server,
        Network {
            resolver,
            dialer,
            tls: net::tls_config(roots),
        },
    )
}
fn app(network: Network) -> AppClient {
    AppClient::with_network(
        Some("fixture-client-id".into()),
        Some("fixture-client-secret".into()),
        network,
    )
}
fn pr() -> PullRequestKey {
    PullRequestKey {
        owner: "rails".into(),
        repo: "rails".into(),
        number: 12,
    }
}
fn result(value: Result<Value, super::client::Error>, prefix: &str) -> Value {
    match value {
        Ok(value) => json!({"value": value}),
        Err(e) => {
            let kind = match e.kind {
                ErrorKind::KeyError => "KeyError".into(),
                ErrorKind::TypeError => "TypeError".into(),
                ErrorKind::NoMethodError => "NoMethodError".into(),
                other => format!(
                    "{prefix}::{}",
                    match other {
                        ErrorKind::Unauthorized => "Unauthorized",
                        ErrorKind::Refused => "Refused",
                        _ => "Error",
                    }
                ),
            };
            json!({"kind": kind, "message": e.message})
        }
    }
}

#[test]
fn github_state_matches_rails_golden() {
    let vectors = vectors();
    let secrets = Secrets::new(vectors["secret_key_base"].as_str().unwrap());
    for case in vectors["states"].as_array().unwrap() {
        let stored = case.get("stored");
        assert_eq!(
            oauth::valid_state(&secrets, case["signed"].as_str().unwrap(), stored, now()),
            case["accepted"].as_bool().unwrap(),
            "{}",
            case["name"]
        );
    }
}

#[test]
fn app_requires_both_credentials_and_authorizes_with_empty_scope() {
    for (id, secret) in [
        (None, None),
        (Some("x".into()), None),
        (None, Some("x".into())),
        (Some(" \t".into()), Some("x".into())),
        (Some("x".into()), Some("\u{2003}".into())),
    ] {
        assert!(!AppClient::new(id, secret).configured());
    }
    let app = AppClient::new(
        Some("fixture-client-id".into()),
        Some("fixture-client-secret".into()),
    );
    assert!(app.configured());
    let url = url::Url::parse(&app.authorize_url("https://app.test/callback?a=1", "raw + state"))
        .unwrap();
    assert_eq!(url.host_str(), Some("github.com"));
    assert_eq!(url.path(), "/login/oauth/authorize");
    let query: std::collections::HashMap<_, _> = url.query_pairs().collect();
    assert_eq!(query.get("scope").unwrap(), "");
    assert_eq!(query.get("state").unwrap(), "raw + state");
    assert_eq!(
        query.get("redirect_uri").unwrap(),
        "https://app.test/callback?a=1"
    );
}

#[tokio::test]
async fn oauth_response_matrix_matches_rails() {
    for case in vectors()["oauth"].as_array().unwrap() {
        let (server, network) = fake(vec![
            Route::new(
                "POST",
                "github.com",
                "/login/oauth/access_token",
                case["status"].as_u64().unwrap() as u16,
            )
            .body(case["body"].as_str().unwrap()),
        ])
        .await;
        assert_eq!(
            result(
                app(network)
                    .exchange_code("fixture-code", "https://app.test/callback")
                    .await,
                "Github::App"
            ),
            case["expected"]
        );
        let request = &server.received()[0];
        assert_eq!(request.header("Accept"), Some("application/json"));
        assert_eq!(
            request.header("Content-Type"),
            Some("application/x-www-form-urlencoded")
        );
        let form: std::collections::HashMap<_, _> =
            url::form_urlencoded::parse(&request.body).collect();
        assert_eq!(form.get("code").unwrap(), "fixture-code");
        assert_eq!(form.get("client_secret").unwrap(), "fixture-client-secret");
        assert_eq!(
            form.get("redirect_uri").unwrap(),
            "https://app.test/callback"
        );
    }
}

#[tokio::test]
async fn refresh_sends_rotating_grant() {
    let (server, network) = fake(vec![
        Route::new("POST", "github.com", "/login/oauth/access_token", 200).body(
            r#"{"access_token":"new-access","refresh_token":"new-refresh","expires_in":28800}"#,
        ),
    ])
    .await;
    let tokens = app(network)
        .refresh_access_token("old-refresh")
        .await
        .unwrap();
    assert_eq!(tokens["refresh_token"], "new-refresh");
    let requests = server.received();
    let form: std::collections::HashMap<_, _> =
        url::form_urlencoded::parse(&requests[0].body).collect();
    assert_eq!(form.get("grant_type").unwrap(), "refresh_token");
    assert_eq!(form.get("refresh_token").unwrap(), "old-refresh");
}

#[tokio::test]
async fn revoke_distinguishes_token_from_grant_and_uses_basic_auth() {
    use base64::Engine as _;
    for (status, token_expected, grant_expected) in [
        (204, true, true),
        (404, true, false),
        (403, false, false),
        (500, false, false),
    ] {
        let (server, network) = fake(vec![
            Route::new(
                "DELETE",
                "api.github.com",
                "/applications/fixture-client-id/token",
                status,
            ),
            Route::new(
                "DELETE",
                "api.github.com",
                "/applications/fixture-client-id/grant",
                status,
            ),
        ])
        .await;
        let client = app(network);
        assert_eq!(client.revoke_token("fixture-access").await, token_expected);
        assert_eq!(client.revoke_grant("fixture-access").await, grant_expected);
        for request in server.received() {
            let header = format!(
                "Basic {}",
                base64::engine::general_purpose::STANDARD
                    .encode("fixture-client-id:fixture-client-secret")
            );
            assert_eq!(request.header("Authorization"), Some(header.as_str()));
            assert_eq!(
                serde_json::from_slice::<Value>(&request.body).unwrap(),
                json!({"access_token":"fixture-access"})
            );
        }
    }
}

#[tokio::test]
async fn write_status_matrix_matches_rails_and_cannot_inject_mentions() {
    for case in vectors()["write"].as_array().unwrap() {
        let (_server, network) = fake(vec![
            Route::new(
                "GET",
                "api.github.com",
                "/user",
                case["status"].as_u64().unwrap() as u16,
            )
            .body(case["body"].as_str().unwrap()),
        ])
        .await;
        assert_eq!(
            result(
                WriteClient::with_network("fixture-access".into(), network)
                    .get_user()
                    .await,
                "Github::WriteClient"
            ),
            case["expected"]
        );
    }
}

#[tokio::test]
async fn write_paths_payloads_identity_and_headers_match_rails() {
    let routes = [
        "/user",
        "/repos/rails/rails",
        "/repos/rails/rails/issues/12/comments",
        "/repos/rails/rails/pulls/12/reviews",
        "/repos/rails/rails/pulls/12/requested_reviewers",
    ]
    .into_iter()
    .enumerate()
    .map(|(i, path)| {
        Route::new(
            if i < 2 { "GET" } else { "POST" },
            "api.github.com",
            path,
            200,
        )
        .body(r#"{"login":"octocat"}"#)
    })
    .collect();
    let (server, network) = fake(routes).await;
    let client = WriteClient::with_network("fixture-member-token".into(), network);
    assert_eq!(client.authenticated_login().await.unwrap(), "octocat");
    assert!(client.repository_readable("rails", "rails").await.unwrap());
    client
        .create_issue_comment(&pr(), "Looks good")
        .await
        .unwrap();
    client
        .create_review(&pr(), "APPROVE", Some(" \t"))
        .await
        .unwrap();
    client
        .create_review(&pr(), "REQUEST_CHANGES", Some("Fix this"))
        .await
        .unwrap();
    client
        .request_reviewers(&pr(), &["alice".into(), "bob".into()])
        .await
        .unwrap();
    let requests = server.received();
    for request in &requests {
        assert_eq!(request.header("X-GitHub-Api-Version"), Some(API_VERSION));
        assert_eq!(
            request.header("User-Agent"),
            Some("Smartfire-GitHub-Writes")
        );
        assert_eq!(
            request.header("Accept"),
            Some("application/vnd.github+json")
        );
        assert_eq!(
            request.header("Authorization"),
            Some(format!("Bearer {}", "fixture-member-token").as_str())
        );
    }
    let bodies: Vec<Value> = requests[2..]
        .iter()
        .map(|r| serde_json::from_slice(&r.body).unwrap())
        .collect();
    assert_eq!(
        bodies,
        vec![
            json!({"body":"Looks good"}),
            json!({"event":"APPROVE"}),
            json!({"event":"REQUEST_CHANGES","body":"Fix this"}),
            json!({"reviewers":["alice","bob"]})
        ]
    );
}

#[tokio::test]
async fn repository_access_denies_refusals_but_propagates_unauthorized_and_errors() {
    for (status, expected) in [
        (200, None),
        (403, None),
        (404, None),
        (422, None),
        (401, Some(ErrorKind::Unauthorized)),
        (500, Some(ErrorKind::Other)),
    ] {
        let (_server, network) = fake(vec![
            Route::new("GET", "api.github.com", "/repos/rails/rails", status).body("{}"),
        ])
        .await;
        let result = WriteClient::with_network("fixture-member-token".into(), network)
            .repository_readable("rails", "rails")
            .await;
        match expected {
            Some(kind) => assert_eq!(result.unwrap_err().kind, kind),
            None => assert_eq!(result.unwrap(), status == 200),
        }
    }
}

#[tokio::test]
async fn reads_use_workspace_token_and_distinct_card_headers() {
    for token in [
        None,
        Some("fixture-workspace-token".into()),
        Some(" \t".into()),
    ] {
        let paths = [
            "/repos/rails/rails/pulls/12",
            "/repos/rails/rails/pulls/12/reviews?per_page=100",
            "/repos/rails/rails/pulls/12/files?per_page=100",
            "/repos/rails/rails/commits/abc/check-runs?per_page=100",
            "/repos/rails/rails/commits/abc/status",
        ];
        let (server, network) = fake(
            paths
                .map(|p| Route::new("GET", "api.github.com", p, 200).body("{}"))
                .to_vec(),
        )
        .await;
        let client = ReadClient::with_network(token.clone(), network);
        client.pull_request(&pr()).await.unwrap();
        client.reviews(&pr()).await.unwrap();
        client.files(&pr()).await.unwrap();
        client.check_runs(&pr(), "abc").await.unwrap();
        client.status(&pr(), "abc").await.unwrap();
        for request in server.received() {
            assert_eq!(request.header("User-Agent"), Some("Smartfire-GitHub-Cards"));
            assert_eq!(request.header("X-GitHub-Api-Version"), Some(API_VERSION));
            let expected = token
                .as_deref()
                .filter(|t| !super::blank(t))
                .map(|t| format!("Bearer {t}"));
            assert_eq!(request.header("Authorization"), expected.as_deref());
        }
    }
}

#[tokio::test]
async fn read_errors_do_not_expose_tokens_or_bodies() {
    for (status, message) in [
        (404, "Pull request not found on GitHub"),
        (401, "GitHub authentication failed"),
        (403, "GitHub request forbidden"),
        (429, "GitHub request forbidden"),
        (500, "GitHub returned 500"),
    ] {
        let (_server, network) = fake(vec![
            Route::new(
                "GET",
                "api.github.com",
                "/repos/rails/rails/pulls/12",
                status,
            )
            .body(r#"{"message":"fixture-token"}"#),
        ])
        .await;
        assert_eq!(
            ReadClient::with_network(Some("fixture-token".into()), network)
                .pull_request(&pr())
                .await
                .unwrap_err()
                .message,
            message
        );
    }
}

#[tokio::test]
async fn transport_errors_and_revocations_never_expose_credentials() {
    let (_server, mut network) = fake(vec![]).await;
    network.resolver = Arc::new(FakeResolver::default());
    let client = app(network.clone());
    assert!(!client.revoke_token("fixture-access").await);
    assert!(!client.revoke_grant("fixture-access").await);
    for error in [
        client
            .exchange_code("fixture-code", "https://app.test/callback")
            .await
            .unwrap_err(),
        WriteClient::with_network("fixture-member-token".into(), network)
            .get_user()
            .await
            .unwrap_err(),
    ] {
        assert!(error.message.starts_with("Could not reach GitHub"));
        for secret in [
            "fixture-client-secret",
            "fixture-member-token",
            "fixture-code",
        ] {
            assert!(!error.to_string().contains(secret));
        }
    }
}

use super::accounts::{
    Account, AccountInput, Accounts, REJECTED_TOKEN_REASON, UNREADABLE_TOKEN_REASON,
};
use crate::integrations::test_support::TestDb;
use campfire_db::Timestamp as DbTimestamp;
use rails_compat::ar_encryption::ArEncryption;

async fn test_database() -> TestDb {
    test_database_with_clock(Arc::new(campfire_db::TestClock::new())).await
}
async fn test_database_with_clock(clock: Arc<campfire_db::TestClock>) -> TestDb {
    tokio::task::spawn_blocking(move || {
        let dir =
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../.scratch/ws15g");
        TestDb::in_dir(clock, &dir)
    })
    .await
    .unwrap()
}
pub(crate) fn crypto() -> Arc<ArEncryption> {
    static CRYPTO: std::sync::OnceLock<Arc<ArEncryption>> = std::sync::OnceLock::new();
    CRYPTO
        .get_or_init(|| {
            Arc::new(ArEncryption::new(&Secrets::new(
                vectors()["secret_key_base"].as_str().unwrap(),
            )))
        })
        .clone()
}
fn input(user_id: i64, source: &str) -> AccountInput<'_> {
    AccountInput {
        user_id,
        github_login: "OctoCat",
        access_token: "fixture-access",
        refresh_token: (source == "app").then_some("fixture-refresh"),
        token_expires_at: None,
        token_source: source,
    }
}
async fn create(test: &TestDb, source: &str, expiry: Option<DbTimestamp>) -> i64 {
    let source = source.to_string();
    let crypto = crypto();
    test.db
        .write(move |tx| {
            let mut input = input(TestDb::id("david"), &source);
            input.token_expires_at = expiry;
            Ok(Account::create(tx, &crypto, &input)?.id)
        })
        .await
        .unwrap()
}
fn find(test: &TestDb, id: i64) -> Account {
    test.db
        .read_blocking(|conn| Ok(Account::find(conn, id)?.unwrap()))
        .unwrap()
}
fn store(test: &TestDb, network: Network) -> Accounts {
    Accounts::with_network(test.db.clone(), crypto(), app(network.clone()), network)
}
fn stored_token(test: &TestDb, id: i64, column: &str) -> String {
    let raw: String = test
        .db
        .read_blocking(|conn| {
            Ok(conn.query_row(
                &format!("SELECT {column} FROM github_connected_accounts WHERE id = ?"),
                [id],
                |r| r.get(0),
            )?)
        })
        .unwrap();
    assert!(
        !raw.starts_with("fixture-"),
        "encrypted column was plaintext"
    );
    crypto().decrypt(&raw).unwrap()
}

#[tokio::test]
async fn accounts_enforce_rails_validations() {
    let test = test_database().await;
    let id = create(&test, "pat", None).await;
    let crypto = crypto();
    for (user_id, login, source, error) in [
        (TestDb::id("david"), "another", "pat", "user_id"),
        (-1, "login", "pat", "user"),
        (TestDb::id("jason"), " \u{2003}", "pat", "github_login"),
        (TestDb::id("jason"), "login", "bad", "token_source"),
    ] {
        let crypto = crypto.clone();
        let result = test
            .db
            .write(move |tx| {
                let mut data = input(user_id, source);
                data.github_login = login;
                Account::create(tx, &crypto, &data).map(|_| ())
            })
            .await;
        let campfire_db::Error::RecordInvalid(errors) = result.unwrap_err() else {
            panic!("expected validation error")
        };
        assert!(!errors.on(error).is_empty());
    }
    assert_eq!(find(&test, id).github_login, "OctoCat");
}

#[tokio::test]
async fn accounts_encrypt_both_columns_and_read_rails_rows() {
    let test = test_database().await;
    let id = create(&test, "app", None).await;
    assert_eq!(stored_token(&test, id, "access_token"), "fixture-access");
    assert_eq!(stored_token(&test, id, "refresh_token"), "fixture-refresh");
    let vectors = vectors();
    let access = vectors["row"]["access_token"].as_str().unwrap().to_string();
    let refresh = vectors["row"]["refresh_token"]
        .as_str()
        .unwrap()
        .to_string();
    test.db.write(move |tx| { tx.conn().execute("UPDATE github_connected_accounts SET access_token = ?, refresh_token = ? WHERE id = ?", rusqlite::params![access, refresh, id])?; Ok(()) }).await.unwrap();
    assert_eq!(
        stored_token(&test, id, "access_token"),
        vectors["plaintext"]["access_token"]
    );
    assert_eq!(
        stored_token(&test, id, "refresh_token"),
        vectors["plaintext"]["refresh_token"]
    );
    let (_server, network) = fake(vec![]).await;
    assert_eq!(
        store(&test, network)
            .access_token_for_use(id)
            .await
            .unwrap()
            .unwrap(),
        vectors["plaintext"]["access_token"]
    );
}

#[tokio::test]
async fn rust_account_rows_are_exported_for_rails_rollback_verification() {
    let test = test_database().await;
    let id = create(
        &test,
        "app",
        Some(
            test.db
                .env()
                .now()
                .since(jiff::SignedDuration::from_hours(1)),
        ),
    )
    .await;
    assert_eq!(stored_token(&test, id, "access_token"), "fixture-access");
    assert_eq!(stored_token(&test, id, "refresh_token"), "fixture-refresh");
    if let Ok(path) = std::env::var("GITHUB_RUST_OUTPUT") {
        let row: Value = test.db.read_blocking(|conn| Ok(conn.query_row("SELECT access_token, refresh_token, github_login, token_source, token_expires_at, created_at, updated_at FROM github_connected_accounts WHERE id = ?", [id], |r| Ok(json!({"access_token":r.get::<_,String>(0)?,"refresh_token":r.get::<_,String>(1)?,"github_login":r.get::<_,String>(2)?,"token_source":r.get::<_,String>(3)?,"token_expires_at":r.get::<_,String>(4)?,"created_at":r.get::<_,String>(5)?,"updated_at":r.get::<_,String>(6)?})))?)).unwrap();
        let path=std::path::Path::new(&path);
        if let Some(parent)=path.parent().filter(|p|!p.as_os_str().is_empty()){std::fs::create_dir_all(parent).unwrap();}
        std::fs::write(path, json!({"row":row,"plaintext":{"access_token":"fixture-access","refresh_token":"fixture-refresh"},"signed_state":oauth::sign_state(&Secrets::new(vectors()["secret_key_base"].as_str().unwrap()),"fixture-session-state")}).to_string()).unwrap();
    }
}

#[tokio::test]
async fn unreadable_or_tampered_tokens_disconnect_without_panicking() {
    for tamper in [false, true] {
        let test = test_database().await;
        let id = create(&test, "pat", None).await;
        test.db
            .write(move |tx| {
                let raw = if tamper {
                    let raw: String = tx.conn().query_row(
                        "SELECT access_token FROM github_connected_accounts WHERE id = ?",
                        [id],
                        |r| r.get(0),
                    )?;
                    let mut raw: Value = serde_json::from_str(&raw).unwrap();
                    raw["h"]["at"] = json!("AAAAAAAAAAAAAAAAAAAAAA==");
                    raw.to_string()
                } else {
                    "bogus-ciphertext".into()
                };
                tx.conn().execute(
                    "UPDATE github_connected_accounts SET access_token = ? WHERE id = ?",
                    rusqlite::params![raw, id],
                )?;
                Ok(())
            })
            .await
            .unwrap();
        let (server, network) = fake(vec![]).await;
        let accounts = store(&test, network);
        assert!(!accounts.usable(id).await.unwrap());
        assert_eq!(accounts.access_token_for_use(id).await.unwrap(), None);
        assert_eq!(
            find(&test, id).disconnected_reason.as_deref(),
            Some(UNREADABLE_TOKEN_REASON)
        );
        assert!(server.received().is_empty());
    }
}

#[tokio::test]
async fn pats_are_used_as_is_and_disconnected_accounts_are_unusable() {
    let test = test_database().await;
    let id = create(&test, "pat", None).await;
    let (server, network) = fake(vec![]).await;
    let accounts = store(&test, network);
    assert!(accounts.usable(id).await.unwrap());
    assert_eq!(
        accounts.access_token_for_use(id).await.unwrap(),
        Some("fixture-access".into())
    );
    assert_eq!(accounts.app_token_for_revoke(id).await.unwrap(), None);
    accounts.revoke_remote_token(id).await.unwrap();
    test.db
        .write(move |tx| Account::mark_disconnected(tx, id, "Disconnected"))
        .await
        .unwrap();
    assert!(!accounts.usable(id).await.unwrap());
    assert_eq!(accounts.access_token_for_use(id).await.unwrap(), None);
    assert!(server.received().is_empty());
}

#[tokio::test]
async fn expired_app_tokens_rotate_with_early_refresh_and_keep_old_refresh_if_omitted() {
    for response in [
        r#"{"access_token":"new-access","refresh_token":"new-refresh","expires_in":28800}"#,
        r#"{"access_token":"new-access","expires_in":"28_800seconds"}"#,
    ] {
        let test = test_database().await;
        let now = test.db.env().now();
        let id = create(
            &test,
            "app",
            Some(now.since(jiff::SignedDuration::from_secs(60))),
        )
        .await;
        let (server, network) = fake(vec![
            Route::new("POST", "github.com", "/login/oauth/access_token", 200).body(response),
        ])
        .await;
        assert_eq!(
            store(&test, network)
                .access_token_for_use(id)
                .await
                .unwrap(),
            Some("new-access".into())
        );
        assert_eq!(
            stored_token(&test, id, "refresh_token"),
            if response.contains("new-refresh") {
                "new-refresh"
            } else {
                "fixture-refresh"
            }
        );
        assert!(find(&test, id).connected());
        assert!(
            find(&test, id).token_expires_at.unwrap()
                > now.since(jiff::SignedDuration::from_hours(7))
        );
        assert_eq!(server.received().len(), 1);
    }
}

#[tokio::test]
async fn rejected_refreshes_disconnect_but_transient_failures_preserve_credentials_and_timestamp() {
    for (status, body, rejected) in [
        (200, r#"{"error":"invalid_grant"}"#, true),
        (200, r#"{"error":"bad_refresh_token"}"#, true),
        (500, "{}", false),
    ] {
        let test = test_database().await;
        let id = create(
            &test,
            "app",
            Some(test.db.env().now().ago(jiff::SignedDuration::from_mins(1))),
        )
        .await;
        let before = find(&test, id).updated_at;
        let (_server, network) = fake(vec![
            Route::new("POST", "github.com", "/login/oauth/access_token", status).body(body),
        ])
        .await;
        assert_eq!(
            store(&test, network)
                .access_token_for_use(id)
                .await
                .unwrap(),
            None
        );
        let current = find(&test, id);
        assert_eq!(current.connected(), !rejected);
        if rejected {
            assert_eq!(
                current.disconnected_reason.as_deref(),
                Some(REJECTED_TOKEN_REASON)
            );
        } else {
            assert!(current.last_error.is_some());
            assert_eq!(current.updated_at, before);
        }
        assert_eq!(stored_token(&test, id, "access_token"), "fixture-access");
        assert_eq!(stored_token(&test, id, "refresh_token"), "fixture-refresh");
    }
}

#[tokio::test]
async fn refresh_races_reuse_the_winner_without_holding_a_database_transaction() {
    for loser_response in [
        r#"{"access_token":"loser-access","refresh_token":"loser-refresh","expires_in":28800}"#,
        r#"{"error":"bad_refresh_token"}"#,
    ] {
        let test = test_database().await;
        let id = create(
            &test,
            "app",
            Some(test.db.env().now().ago(jiff::SignedDuration::from_mins(1))),
        )
        .await;
        let mut route =
            Route::new("POST", "github.com", "/login/oauth/access_token", 200).body(loser_response);
        route.delay = std::time::Duration::from_millis(100);
        let (server, network) = fake(vec![route]).await;
        let accounts = store(&test, network);
        let pending = tokio::spawn(async move { accounts.access_token_for_use(id).await });
        tokio::time::timeout(std::time::Duration::from_secs(3), async {
            while server.received().is_empty() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        let crypto = crypto();
        // This write must finish while the HTTP request is pending. A network-held write lock deadlocks.
        tokio::time::timeout(std::time::Duration::from_secs(2),test.db.write(move |tx| {
            tx.conn().execute("UPDATE github_connected_accounts SET access_token = ?, refresh_token = ?, token_expires_at = ?, updated_at = ? WHERE id = ?",rusqlite::params![crypto.encrypt("winner-access"),crypto.encrypt("winner-refresh"),tx.now().since(jiff::SignedDuration::from_hours(1)),tx.now(),id])?; Ok(())
        })).await.unwrap().unwrap();
        assert_eq!(
            pending.await.unwrap().unwrap(),
            Some("winner-access".into())
        );
        assert_eq!(stored_token(&test, id, "refresh_token"), "winner-refresh");
        assert!(find(&test, id).connected());
    }
}

#[tokio::test]
async fn missing_refresh_and_unreadable_refresh_are_handled_like_rails() {
    for refresh in [None, Some("bogus-ciphertext")] {
        let test = test_database().await;
        let id = create(&test, "app", Some(test.db.env().now())).await;
        test.db
            .write(move |tx| {
                tx.conn().execute(
                    "UPDATE github_connected_accounts SET refresh_token = ? WHERE id = ?",
                    rusqlite::params![refresh, id],
                )?;
                Ok(())
            })
            .await
            .unwrap();
        let (server, network) = fake(vec![]).await;
        assert_eq!(
            store(&test, network)
                .access_token_for_use(id)
                .await
                .unwrap(),
            None
        );
        assert_eq!(find(&test, id).connected(), refresh.is_none());
        assert!(server.received().is_empty());
    }
}

#[tokio::test]
async fn disconnect_refreshes_before_revoking_the_whole_grant() {
    let test = test_database().await;
    let id = create(&test, "app", Some(test.db.env().now())).await;
    let (server, network) = fake(vec![
        Route::new("POST", "github.com", "/login/oauth/access_token", 200).body(
            r#"{"access_token":"fresh-access","refresh_token":"fresh-refresh","expires_in":28800}"#,
        ),
        Route::new(
            "DELETE",
            "api.github.com",
            "/applications/fixture-client-id/grant",
            204,
        ),
    ])
    .await;
    store(&test, network).revoke_remote_token(id).await.unwrap();
    assert_eq!(server.received().len(), 2);
    assert_eq!(
        serde_json::from_slice::<Value>(&server.received()[1].body).unwrap(),
        json!({"access_token":"fresh-access"})
    );
}

#[tokio::test]
async fn failed_refresh_skips_grant_revocation_but_records_error() {
    let test = test_database().await;
    let id = create(&test, "app", Some(test.db.env().now())).await;
    let (server, network) = fake(vec![
        Route::new("POST", "github.com", "/login/oauth/access_token", 500).body("{}"),
    ])
    .await;
    store(&test, network).revoke_remote_token(id).await.unwrap();
    assert_eq!(server.received().len(), 1);
    assert!(find(&test, id).last_error.is_some());
}

#[tokio::test]
async fn verified_logins_displace_unverified_claimants_but_keep_verified_ones() {
    for verified in [false, true] {
        let test = test_database().await;
        let crypto = crypto();
        test.db
            .write(move |tx| {
                if verified {
                    let data = input(TestDb::id("jason"), "pat");
                    Account::create(tx, &crypto, &data)?;
                } else {
                    tx.conn().execute(
                        "UPDATE users SET github_login = 'octocat' WHERE id = ?",
                        [TestDb::id("jason")],
                    )?;
                }
                Ok(())
            })
            .await
            .unwrap();
        create(&test, "pat", None).await;
        let names: Vec<Option<String>> = test
            .db
            .read_blocking(|conn| {
                Ok(["david", "jason"]
                    .into_iter()
                    .map(|u| {
                        conn.query_row(
                            "SELECT github_login FROM users WHERE id = ?",
                            [TestDb::id(u)],
                            |r| r.get(0),
                        )
                    })
                    .collect::<rusqlite::Result<_>>()?)
            })
            .unwrap();
        assert_eq!(
            names,
            if verified {
                vec![None, Some("octocat".into())]
            } else {
                vec![Some("octocat".into()), None]
            }
        );
    }
}

#[tokio::test]
async fn unicode_parity_verified_login_claim_uses_ruby_downcase() {
    let test = test_database().await;
    let crypto = crypto();
    test.db
        .write(move |tx| {
            let mut data = input(TestDb::id("david"), "pat");
            data.github_login = " ΟΣ ";
            Account::create(tx, &crypto, &data)?;
            let login: String = tx.conn().query_row(
                "SELECT github_login FROM users WHERE id=?",
                [TestDb::id("david")],
                |r| r.get(0),
            )?;
            assert_eq!(login, "οσ");
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn unicode_parity_verified_claimant_identity_uses_full_fold() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/unicode_casing_parity.json"
    ))
    .unwrap();
    for case in oracle["comparisons"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|case| ["ß", "ς"].contains(&case["left"].as_str().unwrap()))
    {
        let test = test_database().await;
        let crypto = crypto();
        let existing = case["left"].as_str().unwrap().to_owned();
        let incoming = case["right"].as_str().unwrap().to_owned();
        test.db
            .write(move |tx| {
                let mut claimant = input(TestDb::id("jason"), "pat");
                claimant.github_login = &existing;
                Account::create(tx, &crypto, &claimant)?;
                // Legacy manual profile value reaches Rails' unchanged SQL LOWER predicate.
                tx.conn().execute(
                    "UPDATE users SET github_login=? WHERE id=?",
                    rusqlite::params![
                        rails_compat::unicode::downcase(&incoming),
                        TestDb::id("jason")
                    ],
                )?;
                let mut newcomer = input(TestDb::id("david"), "pat");
                newcomer.github_login = &incoming;
                Account::create(tx, &crypto, &newcomer)?;
                let login: Option<String> = tx.conn().query_row(
                    "SELECT github_login FROM users WHERE id=?",
                    [TestDb::id("david")],
                    |r| r.get(0),
                )?;
                assert_eq!(login, None, "verified claimant must keep its identity");
                Ok(())
            })
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn repository_access_caches_grants_denials_but_not_transport_failures() {
    for status in [200, 403, 500, 401] {
        let test = test_database().await;
        let id = create(&test, "pat", None).await;
        let (server, network) = fake(vec![
            Route::new("GET", "api.github.com", "/repos/rails/rails", status).body("{}"),
        ])
        .await;
        let accounts = store(&test, network);
        assert_eq!(
            accounts
                .can_read_repository(id, "Rails", "RAILS")
                .await
                .unwrap(),
            status == 200
        );
        assert_eq!(
            accounts
                .can_read_repository(id, "rails", "rails")
                .await
                .unwrap(),
            status == 200
        );
        assert_eq!(server.received().len(), if status == 500 { 2 } else { 1 });
        if status == 401 {
            assert_eq!(
                find(&test, id).disconnected_reason.as_deref(),
                Some(REJECTED_TOKEN_REASON)
            );
        }
    }
}

#[tokio::test]
async fn relink_retires_repository_decisions_and_clears_app_credentials() {
    let test = test_database().await;
    let id = create(&test, "app", None).await;
    let (server, network) = fake(vec![
        Route::new("GET", "api.github.com", "/repos/rails/rails", 404).body("{}"),
    ])
    .await;
    let accounts = store(&test, network);
    assert!(
        !accounts
            .can_read_repository(id, "rails", "rails")
            .await
            .unwrap()
    );
    let crypto = crypto();
    test.db
        .write(move |tx| {
            Account::relink(tx, &crypto, &input(TestDb::id("david"), "pat")).map(|_| ())
        })
        .await
        .unwrap();
    assert!(
        !accounts
            .can_read_repository(id, "rails", "rails")
            .await
            .unwrap()
    );
    assert_eq!(server.received().len(), 2);
    assert_eq!(find(&test, id).token_source, "pat");
    assert_eq!(accounts.app_token_for_revoke(id).await.unwrap(), None);
}

#[tokio::test]
async fn agent_identity_prefers_owner_app_but_falls_back_to_machine_pat() {
    for (source, disconnected) in [("app", false), ("pat", false), ("app", true)] {
        let test = test_database().await;
        let owner = create(&test, source, None).await;
        let crypto = crypto();
        let machine = test
            .db
            .write(move |tx| {
                if disconnected {
                    Account::mark_disconnected(tx, owner, REJECTED_TOKEN_REASON)?;
                }
                let mut data = input(TestDb::id("jason"), "pat");
                data.github_login = "machine-user";
                Ok(Account::create(tx, &crypto, &data)?.id)
            })
            .await
            .unwrap();
        let (_server, network) = fake(vec![]).await;
        let chosen = store(&test, network)
            .agent_identity(Some(TestDb::id("david")), TestDb::id("jason"))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            chosen.id,
            if source == "app" && !disconnected {
                owner
            } else {
                machine
            }
        );
    }
}

#[tokio::test]
async fn repository_cache_expires_at_ten_minutes_and_relink_bumps_microseconds() {
    let clock = Arc::new(campfire_db::TestClock::frozen_at(DbTimestamp::from_jiff(
        now(),
    )));
    let test = test_database_with_clock(clock.clone()).await;
    let id = create(&test, "pat", None).await;
    let (server, network) = fake(vec![
        Route::new("GET", "api.github.com", "/repos/rails/rails", 404).body("{}"),
    ])
    .await;
    let accounts = store(&test, network);
    assert!(
        !accounts
            .can_read_repository(id, "rails", "rails")
            .await
            .unwrap()
    );
    clock.travel(jiff::SignedDuration::from_secs(599));
    assert!(
        !accounts
            .can_read_repository(id, "rails", "rails")
            .await
            .unwrap()
    );
    assert_eq!(server.received().len(), 1);
    clock.travel(jiff::SignedDuration::from_secs(1));
    assert!(
        !accounts
            .can_read_repository(id, "rails", "rails")
            .await
            .unwrap()
    );
    assert_eq!(server.received().len(), 2);
    clock.travel(jiff::SignedDuration::from_micros(1));
    let crypto = crypto();
    test.db
        .write(move |tx| {
            Account::relink(tx, &crypto, &input(TestDb::id("david"), "pat")).map(|_| ())
        })
        .await
        .unwrap();
    assert!(
        !accounts
            .can_read_repository(id, "rails", "rails")
            .await
            .unwrap()
    );
    assert_eq!(server.received().len(), 3);
}

#[tokio::test]
async fn stale_account_lookups_reuse_rotated_credentials_without_http() {
    let test = test_database().await;
    let id = create(&test, "app", Some(test.db.env().now())).await;
    let crypto = crypto();
    test.db
        .write(move |tx| {
            let mut data = input(TestDb::id("david"), "app");
            data.access_token = "winner-access";
            data.refresh_token = Some("winner-refresh");
            data.token_expires_at = Some(tx.now().since(jiff::SignedDuration::from_hours(1)));
            Account::relink(tx, &crypto, &data).map(|_| ())
        })
        .await
        .unwrap();
    let (server, network) = fake(vec![]).await;
    assert_eq!(
        store(&test, network)
            .access_token_for_use(id)
            .await
            .unwrap(),
        Some("winner-access".into())
    );
    assert!(server.received().is_empty());
}

#[tokio::test]
async fn rejected_owner_refresh_falls_back_to_agent_pat() {
    let test = test_database().await;
    let id = create(&test, "app", Some(test.db.env().now())).await;
    let crypto = crypto();
    let machine = test
        .db
        .write(move |tx| {
            let mut data = input(TestDb::id("jason"), "pat");
            data.github_login = "machine-login";
            Ok(Account::create(tx, &crypto, &data)?.id)
        })
        .await
        .unwrap();
    let (_server, network) = fake(vec![
        Route::new("POST", "github.com", "/login/oauth/access_token", 200)
            .body(r#"{"error":"bad_refresh_token"}"#),
    ])
    .await;
    let accounts = store(&test, network);
    assert_eq!(accounts.access_token_for_use(id).await.unwrap(), None);
    assert_eq!(
        accounts
            .agent_identity(Some(TestDb::id("david")), TestDb::id("jason"))
            .await
            .unwrap()
            .unwrap()
            .id,
        machine
    );
}

#[tokio::test]
async fn transport_failure_leaves_app_connected_and_skips_remote_revoke() {
    let test = test_database().await;
    let id = create(&test, "app", Some(test.db.env().now())).await;
    let (_server, mut network) = fake(vec![]).await;
    network.resolver = Arc::new(FakeResolver::default());
    let accounts = store(&test, network);
    assert_eq!(accounts.access_token_for_use(id).await.unwrap(), None);
    accounts.revoke_remote_token(id).await.unwrap();
    assert!(find(&test, id).connected());
    assert!(find(&test, id).last_error.is_some());
}

#[tokio::test]
async fn real_http_read_timeout_is_ten_seconds_and_never_leaks_credentials() {
    assert_eq!(super::client::TIMEOUT, std::time::Duration::from_secs(10));
    let mut route = Route::new("GET", "api.github.com", "/user", 200).body("{}");
    route.delay = std::time::Duration::from_secs(20);
    let (_server, network) = fake(vec![route]).await;
    let error = WriteClient::with_network("fixture-member-token".into(), network)
        .get_user()
        .await
        .unwrap_err();
    assert_eq!(error.message, "Could not reach GitHub (Read Timeout)");
}

#[tokio::test]
async fn pat_validation_matches_rails_for_login_coercions_and_unexpected_exceptions() {
    for case in vectors()["login"].as_array().unwrap() {
        let (_server, network) = fake(vec![
            Route::new("GET", "api.github.com", "/user", 200).body(case["body"].as_str().unwrap()),
        ])
        .await;
        let value = WriteClient::with_network("fixture-member-token".into(), network)
            .authenticated_login()
            .await
            .map(Value::String);
        assert_eq!(result(value, "Github::WriteClient"), case["expected"]);
    }
}

#[tokio::test]
async fn read_status_and_rate_limit_errors_match_rails() {
    for case in vectors()["read"].as_array().unwrap() {
        let mut route = Route::new(
            "GET",
            "api.github.com",
            "/repos/rails/rails/pulls/12",
            case["status"].as_u64().unwrap() as u16,
        )
        .body(case["body"].as_str().unwrap());
        for (key, value) in case["headers"].as_object().unwrap() {
            route = route.header(key, value.as_str().unwrap());
        }
        let (_server, network) = fake(vec![route]).await;
        let error = ReadClient::with_network(Some("fixture-workspace-token".into()), network)
            .pull_request(&pr())
            .await
            .unwrap_err();
        if case["expected"]["kind"] == "JSON::ParserError" {
            // The enclosing Rails fetcher persists the class, never the raw JSON parser message.
            assert_eq!(error.message, "Could not reach GitHub (Parser Error)");
        } else {
            assert_eq!(error.message, case["expected"]["message"]);
        }
    }
}

#[tokio::test]
async fn refresh_disconnects_if_a_concurrent_writer_corrupts_the_stored_refresh_token() {
    let test = test_database().await;
    let id = create(&test, "app", Some(test.db.env().now())).await;
    let mut route = Route::new("POST", "github.com", "/login/oauth/access_token", 200)
        .body(r#"{"access_token":"new-access","refresh_token":"new-refresh","expires_in":28800}"#);
    route.delay = std::time::Duration::from_millis(100);
    let (server, network) = fake(vec![route]).await;
    let accounts = store(&test, network);
    let pending = tokio::spawn(async move { accounts.access_token_for_use(id).await });
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        while server.received().is_empty() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    test.db.write(move |tx| {tx.conn().execute("UPDATE github_connected_accounts SET refresh_token = 'bogus-ciphertext' WHERE id = ?",[id])?; Ok(())}).await.unwrap();
    assert_eq!(pending.await.unwrap().unwrap(), None);
    assert_eq!(
        find(&test, id).disconnected_reason.as_deref(),
        Some(UNREADABLE_TOKEN_REASON)
    );
    assert_eq!(stored_token(&test, id, "access_token"), "fixture-access");
}

#[tokio::test]
async fn agent_identity_checks_unreadable_owner_pat_before_falling_back() {
    let test = test_database().await;
    let owner = create(&test, "pat", None).await;
    let crypto = crypto();
    let machine=test.db.write(move |tx| {
        tx.conn().execute("UPDATE github_connected_accounts SET access_token = 'bogus-ciphertext' WHERE id = ?",[owner])?;
        let mut data=input(TestDb::id("jason"),"pat"); data.github_login="machine-login";
        Ok(Account::create(tx,&crypto,&data)?.id)
    }).await.unwrap();
    let (_server, network) = fake(vec![]).await;
    assert_eq!(
        store(&test, network)
            .agent_identity(Some(TestDb::id("david")), TestDb::id("jason"))
            .await
            .unwrap()
            .unwrap()
            .id,
        machine
    );
    assert_eq!(
        find(&test, owner).disconnected_reason.as_deref(),
        Some(UNREADABLE_TOKEN_REASON)
    );
}

#[tokio::test]
async fn boot_installs_owner_repository_reader_and_reuses_its_permission_cache() {
    use campfire_db::{Agent, ChannelThread, NewChannelThread};
    use crate::controllers::presenters::test_support::{TestApp, BENDER, DAVID, ALL_TALK};
    let (server, network) = fake(vec![Route::new("GET", "api.github.com", "/repos/mixed/repo", 200).body("{}").header("content-type", "application/json")]).await;
    let test = TestApp::boot_with_network(network).await.expect("default seed");
    let crypto = test.booted.app.ar_encryption.clone();
    let (agent,thread) = test.booted.app.db.write(move |tx| {
        let agent=Agent::for_user(tx.conn(),BENDER)?.unwrap();
        super::accounts::Account::create(tx,&crypto,&super::accounts::AccountInput {
            user_id: DAVID, github_login: "owner-fixture", access_token: "owner-repository-fixture-token",
            refresh_token: None, token_expires_at: None, token_source: "pat",
        })?;
        let thread=ChannelThread::create(tx,NewChannelThread {room_id:ALL_TALK,creator_id:DAVID,name:Some("Private owner read".into()),..Default::default()})?;
        tx.conn().execute("INSERT INTO github_pull_requests(id,owner,repo,number,title,private,created_at,updated_at) VALUES (900130009,'Mixed','Repo',12,'Private fixture',1,?,?)",rusqlite::params![tx.now(),tx.now()])?;
        tx.conn().execute("INSERT INTO work_thread_links(channel_thread_id,kind,github_pull_request_id,created_by_id,created_at,updated_at) VALUES (?,'pull_request',900130009,?,?,?)",rusqlite::params![thread.id,DAVID,tx.now(),tx.now()])?;
        Ok((agent.id,thread.id))
    }).await.unwrap();
    for _ in 0..2 {
        let access=test.booted.app.agent_repositories.resolve_threads(&test.booted.app.db,agent,vec![thread]).await.unwrap();
        assert!(access.contains(&(DAVID,"mixed".into(),"repo".into())));
    }
    assert_eq!(server.received.lock().unwrap().len(),1,"the owner's ten-minute cache must survive callers");
    let requests=server.received.lock().unwrap().clone();
    assert_eq!(requests[0].header("authorization"),Some(format!("Bearer {}", "owner-repository-fixture-token").as_str()));
    test.booted.app.db.write(move |tx| {tx.conn().execute("UPDATE agents SET owner_id=NULL WHERE id=?",[agent])?;Ok(())}).await.unwrap();
    assert!(test.booted.app.agent_repositories.resolve_threads(&test.booted.app.db,agent,vec![thread]).await.unwrap().is_empty());
    assert_eq!(server.received.lock().unwrap().len(),1);
}
