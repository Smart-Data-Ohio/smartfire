//! The pinned Rails GitHub browser bodies drive the live Rust HTTP/Turbo path.
use crate::integrations::{
    net,
    test_support::{FakeResolver, MappingDialer},
};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

pub(super) struct Server(tokio::task::JoinHandle<()>);
impl Drop for Server {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// Only GitHub's external exchange is forwarded to the original WebMock stubs.
/// The producer still builds its real method, URL, headers, JSON and TLS request.
pub(super) async fn network(control: std::path::PathBuf) -> (net::Network, Server) {
    use rustls::pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer};
    let rcgen::CertifiedKey { cert, signing_key } =
        rcgen::generate_simple_self_signed(vec!["api.github.com".into()]).unwrap();
    let certificate = cert.der().clone();
    let mut roots = rustls::RootCertStore::empty();
    roots.add(certificate.clone()).unwrap();
    let config = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .unwrap()
    .with_no_client_auth()
    .with_single_cert(
        vec![certificate],
        PrivateKeyDer::from(PrivatePkcs8KeyDer::from(signing_key.serialize_der())),
    )
    .unwrap();
    let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));
    let listener = crate::test_support::bind_listener().await;
    let address = listener.local_addr().unwrap();
    let serving = tokio::spawn(async move {
        loop {
            let Ok((socket, _)) = listener.accept().await else {
                break;
            };
            let acceptor = acceptor.clone();
            let control = control.clone();
            tokio::spawn(async move {
                let Ok(socket) = acceptor.accept(socket).await else {
                    return;
                };
                let mut input = BufReader::new(socket);
                let mut line = String::new();
                input.read_line(&mut line).await.unwrap();
                let mut pieces = line.split_whitespace();
                let method = pieces.next().unwrap().to_owned();
                let target = pieces.next().unwrap().to_owned();
                let mut headers = vec![];
                let mut length = 0;
                let mut host = String::new();
                loop {
                    line.clear();
                    input.read_line(&mut line).await.unwrap();
                    if line == "\r\n" {
                        break;
                    }
                    let (name, value) = line.trim_end().split_once(':').unwrap();
                    let value = value.trim().to_owned();
                    if name.eq_ignore_ascii_case("content-length") {
                        length = value.parse().unwrap()
                    }
                    if name.eq_ignore_ascii_case("host") {
                        host = value.clone()
                    }
                    headers.push((name.to_owned(), value));
                }
                let mut body = vec![0; length];
                input.read_exact(&mut body).await.unwrap();
                let port = std::fs::read_to_string(control.join("google-port"))
                    .unwrap()
                    .parse::<u16>()
                    .unwrap();
                let endpoint = net::http::Endpoint {
                    https: false,
                    host: "127.0.0.1".into(),
                    port,
                    pinned_ip: None,
                };
                let mut request = net::http::Request::net_http(
                    hyper::Method::POST,
                    "/".into(),
                    Some("127.0.0.1".into()),
                    vec![("Content-Type".into(), "application/json".into())],
                );
                request.body=serde_json::to_vec(&serde_json::json!({"host":host,"method":method,"target":target,"headers":headers,"body":String::from_utf8(body).unwrap()})).unwrap();
                let reply = net::http::exchange(
                    &net::Network::system(),
                    &endpoint,
                    request,
                    &net::http::Timeouts {
                        open: std::time::Duration::from_secs(10),
                        read: std::time::Duration::from_secs(10),
                        write: std::time::Duration::from_secs(10),
                    },
                )
                .await
                .unwrap();
                let net::http::Body::Complete(bytes) = reply.read_body(usize::MAX).await.unwrap()
                else {
                    unreachable!()
                };
                let result: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
                let bytes = result["body"].as_str().unwrap().as_bytes();
                let response = format!(
                    "HTTP/1.1 {} Status\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    result["status"].as_u64().unwrap(),
                    bytes.len()
                );
                input
                    .get_mut()
                    .write_all(response.as_bytes())
                    .await
                    .unwrap();
                input.get_mut().write_all(bytes).await.unwrap();
                input.get_mut().shutdown().await.unwrap();
            });
        }
    });
    let ip = "93.184.216.34".parse().unwrap();
    (
        net::Network {
            resolver: Arc::new(FakeResolver::new([(
                "api.github.com",
                vec!["93.184.216.34"],
            )])),
            dialer: Arc::new(MappingDialer {
                public: [ip].into(),
                to: address,
                dialed: Default::default(),
            }),
            tls: net::tls_config(roots),
        },
        Server(serving),
    )
}
#[tokio::test]
#[ignore = "requires native Capybara, Docker and Chromium; run parity/system/ws14-original"]
async fn original_browser_ws15g_062() {
    super::ws14_original_browser_tests::original("WS15g-062").await;
}
#[tokio::test]
#[ignore = "requires native Capybara, Docker and Chromium; run parity/system/ws14-original"]
async fn original_browser_ws15g_063() {
    super::ws14_original_browser_tests::original("WS15g-063").await;
}
#[tokio::test]
#[ignore = "requires native Capybara, Docker and Chromium; run parity/system/ws14-original"]
async fn original_browser_ws15g_064() {
    super::ws14_original_browser_tests::original("WS15g-064").await;
}
