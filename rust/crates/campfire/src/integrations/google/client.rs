//! Injectable transport for fixed Google hosts, matching `Google::Client#request`.
use crate::integrations::net::{
    BoxFuture, Network,
    http::{self, Body, Endpoint, Request, Timeouts},
};
use std::time::Duration;
#[derive(Debug, thiserror::Error)]
#[error("Google transport unavailable")]
pub struct Unavailable(&'static str);
impl Unavailable {
    /// Google::Client persists only the Ruby exception class, never transport details.
    pub fn class(&self) -> &'static str {
        self.0
    }
}
impl From<http::HttpError> for Unavailable {
    fn from(error: http::HttpError) -> Self {
        use http::HttpError;
        Self(match error {
            HttpError::OpenTimeout => "Net::OpenTimeout",
            HttpError::ReadTimeout => "Net::ReadTimeout",
            HttpError::WriteTimeout => "Net::WriteTimeout",
            HttpError::Unresolvable(_) => "SocketError",
            HttpError::Tls(_) => "OpenSSL::SSL::SSLError",
            HttpError::ConnectionClosed => "EOFError",
            HttpError::Http(_) => "Net::HTTPBadResponse",
            HttpError::Inflate(_) => "Zlib::DataError",
            HttpError::Io(error) => match error.raw_os_error() {
                Some(libc::ECONNREFUSED) => "Errno::ECONNREFUSED",
                Some(libc::ECONNRESET) => "Errno::ECONNRESET",
                Some(libc::ECONNABORTED) => "Errno::ECONNABORTED",
                Some(libc::ETIMEDOUT) => "Errno::ETIMEDOUT",
                Some(libc::EPIPE) => "Errno::EPIPE",
                Some(libc::ENETDOWN) => "Errno::ENETDOWN",
                Some(libc::ENETUNREACH) => "Errno::ENETUNREACH",
                Some(libc::EHOSTUNREACH) => "Errno::EHOSTUNREACH",
                Some(libc::EADDRNOTAVAIL) => "Errno::EADDRNOTAVAIL",
                Some(libc::EADDRINUSE) => "Errno::EADDRINUSE",
                Some(libc::EACCES) => "Errno::EACCES",
                Some(libc::EPERM) => "Errno::EPERM",
                Some(libc::EINVAL) => "Errno::EINVAL",
                Some(libc::EMFILE) => "Errno::EMFILE",
                Some(libc::ENFILE) => "Errno::ENFILE",
                Some(libc::ENOMEM) => "Errno::ENOMEM",
                Some(libc::ENOBUFS) => "Errno::ENOBUFS",
                Some(libc::EINTR) => "Errno::EINTR",
                Some(libc::EAGAIN) => "Errno::EAGAIN",
                Some(_) => "SystemCallError",
                None => "IOError",
            },
        })
    }
}
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
                return Err(http::HttpError::Unresolvable(host.into()).into());
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
                    write: Duration::from_secs(10),
                },
            )
            .await
            .map_err(Unavailable::from)?;
            let status = response.status;
            match response
                .read_body(usize::MAX)
                .await
                .map_err(Unavailable::from)?
            {
                Body::Complete(body) => Ok((status, body)),
                Body::TooLarge => Err(http::HttpError::Http("body too large".into()).into()),
            }
        })
    }
}
