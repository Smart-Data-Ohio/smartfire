//! Injectable transport for fixed Google hosts, matching `Google::Client#request`.
use crate::integrations::net::{
    BoxFuture, Network,
    http::{self, Body, Endpoint, Request, Timeouts},
};
use std::time::Duration;
#[derive(Debug, thiserror::Error)]
#[error("Google transport unavailable")]
pub struct Unavailable;
pub trait Client: Send + Sync {
    fn request<'a>(
        &'a self,
        host: &'a str,
        method: hyper::Method,
        target: &'a str,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
    ) -> BoxFuture<'a, Result<(u16, Vec<u8>), Unavailable>>;
}
pub struct HttpClient(pub Network);
impl Client for HttpClient {
    fn request<'a>(
        &'a self,
        host: &'a str,
        method: hyper::Method,
        target: &'a str,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
    ) -> BoxFuture<'a, Result<(u16, Vec<u8>), Unavailable>> {
        Box::pin(async move {
            if !matches!(host, "www.googleapis.com" | "oauth2.googleapis.com") {
                return Err(Unavailable);
            }
            let endpoint = Endpoint {
                https: true,
                host: host.into(),
                port: 443,
                pinned_ip: None,
            };
            let mut request = Request::net_http(method, target.into(), Some(host.into()), headers);
            request.body = body;
            let response = http::exchange(
                &self.0,
                &endpoint,
                request,
                &Timeouts {
                    open: Duration::from_secs(10),
                    read: Duration::from_secs(10),
                },
            )
            .await
            .map_err(|_| Unavailable)?;
            let status = response.status;
            match response
                .read_body(usize::MAX)
                .await
                .map_err(|_| Unavailable)?
            {
                Body::Complete(body) => Ok((status, body)),
                Body::TooLarge => Err(Unavailable),
            }
        })
    }
}
