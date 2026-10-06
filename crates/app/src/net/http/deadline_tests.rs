//! The shared connector also serves GitHub; every connection stage consumes one budget.
use super::*;
use crate::net::{BoxFuture, Dialer, Resolver};
use crate::net::test_support::{FakeResolver, MappingDialer, ws15e_listener};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

fn test_network(resolver: Arc<dyn Resolver>, dialer: Arc<dyn Dialer>) -> Network {
    Network {
        resolver,
        dialer,
        tls: Network::system().tls,
    }
}

struct SlowDns;
impl Resolver for SlowDns {
    fn lookup<'a>(&'a self, _: &'a str) -> BoxFuture<'a, io::Result<Vec<IpAddr>>> {
        Box::pin(async {
            tokio::time::sleep(Duration::from_millis(300)).await;
            Ok(vec!["93.184.216.34".parse().unwrap()])
        })
    }
}
struct UnreachableDialer(Arc<AtomicUsize>);
impl Dialer for UnreachableDialer {
    fn connect(&self, _: SocketAddr) -> BoxFuture<'_, io::Result<tokio::net::TcpStream>> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Box::pin(async { Err(io::Error::from(io::ErrorKind::ConnectionRefused)) })
    }
}
fn endpoint(https: bool) -> Endpoint {
    Endpoint {
        https,
        host: "budget.example.test".into(),
        port: if https { 443 } else { 80 },
        pinned_ip: None,
    }
}
fn timeouts() -> Timeouts {
    Timeouts {
        open: Duration::from_millis(200),
        read: Duration::from_secs(1),
        write: Duration::from_secs(1),
    }
}
async fn exchange_get(net: &Network, endpoint: &Endpoint) -> Result<Response, HttpError> {
    let request =
        Request::net_http(hyper::Method::GET, "/".into(), None, vec![]).transport(false, endpoint);
    exchange(net, endpoint, request, &timeouts()).await
}
#[tokio::test]
async fn ws15e_shared_open_timeout_cancels_dns_before_dial() {
    let calls = Arc::new(AtomicUsize::new(0));
    let net = test_network(
        Arc::new(SlowDns),
        Arc::new(UnreachableDialer(calls.clone())),
    );
    assert!(matches!(
        exchange_get(&net, &endpoint(false)).await,
        Err(HttpError::OpenTimeout)
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}
#[tokio::test]
async fn ws15e_shared_open_timeout_covers_all_address_attempts() {
    struct SlowAttempts(Arc<AtomicUsize>);
    impl Dialer for SlowAttempts {
        fn connect(&self, _: SocketAddr) -> BoxFuture<'_, io::Result<tokio::net::TcpStream>> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Box::pin(async {
                tokio::time::sleep(Duration::from_millis(120)).await;
                Err(io::Error::from(io::ErrorKind::ConnectionRefused))
            })
        }
    }
    let calls = Arc::new(AtomicUsize::new(0));
    let net = test_network(
        Arc::new(FakeResolver::new([(
            "budget.example.test",
            vec!["93.184.216.34", "93.184.216.35", "93.184.216.36"],
        )])),
        Arc::new(SlowAttempts(calls.clone())),
    );
    assert!(matches!(
        exchange_get(&net, &endpoint(false)).await,
        Err(HttpError::OpenTimeout)
    ));
    assert_eq!(
        calls.load(Ordering::SeqCst),
        2,
        "address fallback cannot restart the budget"
    );
}
#[tokio::test]
async fn ws15e_shared_open_timeout_includes_tls_handshake() {
    let listener = ws15e_listener().await;
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buffer = [0; 1024];
        tokio::io::AsyncReadExt::read(&mut socket, &mut buffer)
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_secs(2)).await;
    });
    let net = test_network(
        Arc::new(FakeResolver::new([(
            "budget.example.test",
            vec!["93.184.216.34"],
        )])),
        Arc::new(MappingDialer {
            public: ["93.184.216.34".parse().unwrap()].into(),
            to: address,
            dialed: Default::default(),
        }),
    );
    let result = exchange_get(&net, &endpoint(true)).await;
    server.abort();
    assert!(matches!(result, Err(HttpError::OpenTimeout)));
}
