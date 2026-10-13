//! Playwright over the same seeded TestApp/router as spa_huddle_tests, with the SPA embedded.
use super::presenters::test_support::TestApp;
use campfire_app::net::{
    Network,
    test_support::{FakeResolver, MappingDialer},
};
use std::{
    collections::HashSet,
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::{TlsAcceptor, server::TlsStream};

struct TlsListener {
    tcp: TcpListener,
    tls: TlsAcceptor,
}
impl axum::serve::Listener for TlsListener {
    type Io = TlsStream<TcpStream>;
    type Addr = SocketAddr;
    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        loop {
            let (stream, addr) = self.tcp.accept().await.unwrap();
            if let Ok(stream) = self.tls.accept(stream).await {
                return (stream, addr);
            }
        }
    }
    fn local_addr(&self) -> std::io::Result<SocketAddr> {
        self.tcp.local_addr()
    }
}

#[tokio::test]
#[ignore = "Runs in the OAuth browser CI job with Node, Chromium, and an embedded SPA"]
async fn oauth_browser_acceptance() {
    assert!(
        campfire_spa::built(),
        "Build frontend/dist and set SPA_DIST before compiling"
    );
    assert_eq!(std::env::var("SMARTFIRE_E2E_PORT").as_deref(), Ok("4398"));
    let addr: SocketAddr = "127.0.0.1:4398".parse().unwrap();
    let rcgen::CertifiedKey { cert, signing_key } =
        rcgen::generate_simple_self_signed(vec!["localhost".into(), "client.example".into()])
            .unwrap();
    let mut roots = rustls::RootCertStore::empty();
    roots.add(cert.der().clone()).unwrap();
    let network = Network {
        resolver: Arc::new(FakeResolver::new([(
            "client.example",
            vec!["93.184.216.34"],
        )])),
        dialer: Arc::new(MappingDialer {
            public: HashSet::from(["93.184.216.34".parse().unwrap()]),
            to: addr,
            dialed: Mutex::new(vec![]),
        }),
        tls: campfire_app::net::tls_config(roots),
    };
    let app = TestApp::boot_with_network_clock_and_env(
        network,
        Arc::new(campfire_kit::FrozenClock::new(
            "2026-10-13T16:00:00Z".parse().unwrap(),
        )),
        &[("APP_URL", "https://localhost:4398")],
    )
    .await
    .expect("restored default seed")
    .without_job_runner()
    .await;
    let encryption = app.booted.app.ar_encryption.clone();
    let code = app
        .db()
        .read(move |conn| {
            let factor = campfire_db::TwoFactorCredential::for_user(
                conn,
                super::presenters::test_support::KEVIN,
            )?
            .unwrap();
            Ok(rails_compat::totp::at(
                &factor.secret(&encryption)?,
                "2026-10-13T16:00:00Z"
                    .parse::<jiff::Timestamp>()
                    .unwrap()
                    .as_second(),
            )
            .unwrap())
        })
        .await
        .unwrap();
    let router = axum::Router::new().route("/client.json", axum::routing::get(|| async {
        axum::Json(serde_json::json!({"client_id":"https://client.example:4398/client.json","client_name":"S9 test client","redirect_uris":["https://client.example:4398/callback"],"token_endpoint_auth_method":"none"}))
    })).fallback_service(app.booted.router.clone())
        .layer(axum::Extension(axum::extract::ConnectInfo(addr)));
    let tls = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .unwrap()
    .with_no_client_auth()
    .with_single_cert(
        vec![cert.der().clone()],
        rustls::pki_types::PrivatePkcs8KeyDer::from(signing_key.serialize_der()).into(),
    )
    .unwrap();
    let listener = TlsListener {
        tcp: TcpListener::bind(addr).await.unwrap(),
        tls: TlsAcceptor::from(Arc::new(tls)),
    };
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let status = tokio::process::Command::new("pnpm")
        .args([
            "--dir",
            "frontend",
            "exec",
            "playwright",
            "test",
            "-c",
            "playwright.oauth.config.ts",
            "--reporter=line",
        ])
        .env("SMARTFIRE_E2E_TOTP", code)
        .current_dir(env!("CARGO_MANIFEST_DIR").to_owned() + "/../..")
        .status()
        .await
        .unwrap();
    assert!(
        status.success(),
        "Playwright OAuth acceptance failed: {status}"
    );
    let ca = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(ca.path(), cert.pem()).unwrap();
    let smoke = tokio::process::Command::new("ops/scripts/mcp-oauth-smoke.sh")
        .arg("https://localhost:4398")
        .env("CURL_CA_BUNDLE", ca.path())
        .current_dir(env!("CARGO_MANIFEST_DIR").to_owned() + "/../..")
        .status()
        .await
        .unwrap();
    server.abort();
    assert!(smoke.success(), "OAuth deployment smoke failed: {smoke}");
}
