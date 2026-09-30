//! `app/models/fizzy/client.rb`. The configured origin is trusted (including self-hosted HTTP).
//! Path IDs are checked before DNS. Each request resolves once and pins its connection.
use super::blank;
use crate::integrations::net::{
    Network,
    http::{self, Body, Endpoint, HttpError, Request, Timeouts},
};
use campfire_richtext::uri;
use hyper::Method;
use serde_json::{Value, json};
use std::time::Duration;

pub const DEFAULT_BASE: &str = "https://app.fizzy.do";
pub const TIMEOUTS: Timeouts = Timeouts {
    open: Duration::from_secs(10),
    read: Duration::from_secs(10),
    write: Duration::from_secs(60),
};
pub fn api_base_url() -> String {
    let base = std::env::var("FIZZY_API_BASE_URL").unwrap_or_else(|_| DEFAULT_BASE.into());
    chomp_slash(&base).into()
}
fn chomp_slash(base: &str) -> &str {
    base.strip_suffix('/').unwrap_or(base)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    Unauthorized,
    NotFound,
    Forbidden,
    Refused,
    Other,
}
#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct Error {
    pub kind: ErrorKind,
    pub message: String,
}
impl Error {
    fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
    fn unreachable(class: &str) -> Self {
        Self::new(ErrorKind::Other, format!("Could not reach Fizzy ({class})"))
    }
    fn transport(error: HttpError) -> Self {
        let class = match error {
            HttpError::OpenTimeout => "Open Timeout",
            HttpError::ReadTimeout => "Read Timeout",
            HttpError::WriteTimeout => "Write Timeout",
            HttpError::ConnectionClosed => "Eof Error",
            HttpError::Unresolvable(_) => "Socket Error",
            HttpError::Tls(_) => "Ssl Error",
            HttpError::Io(e) if e.kind() == std::io::ErrorKind::ConnectionRefused => "Econnrefused",
            HttpError::Io(_) => "Io Error",
            HttpError::Http(_) => "Http Error",
            HttpError::Inflate(_) => "Error",
        };
        Self::unreachable(class)
    }
}

// Deliberately no Debug/Serialize: the client owns a plaintext credential.
pub struct Client {
    network: Network,
    base: String,
    token: String,
}
impl Client {
    pub fn new(network: Network, token: String, base: &str) -> Self {
        Self {
            network,
            token,
            base: chomp_slash(base).into(),
        }
    }
    pub async fn identity(&self) -> Result<Value, Error> {
        self.request(Method::GET, "/my/identity.json".into(), None)
            .await
    }
    pub async fn boards(&self, account: &str) -> Result<Value, Error> {
        self.request(
            Method::GET,
            format!("/{}/boards.json", checked_id(account)?),
            None,
        )
        .await
    }
    pub async fn board(&self, account: &str, board: &str) -> Result<Value, Error> {
        self.request(
            Method::GET,
            format!(
                "/{}/boards/{}.json",
                checked_id(account)?,
                checked_id(board)?
            ),
            None,
        )
        .await
    }
    pub async fn columns(&self, account: &str, board: &str) -> Result<Value, Error> {
        self.request(
            Method::GET,
            format!(
                "/{}/boards/{}/columns.json",
                checked_id(account)?,
                checked_id(board)?
            ),
            None,
        )
        .await
    }
    pub async fn card(&self, account: &str, number: &str) -> Result<Value, Error> {
        self.request(
            Method::GET,
            format!(
                "/{}/cards/{}.json",
                checked_id(account)?,
                checked_number(number)?
            ),
            None,
        )
        .await
    }
    pub async fn search(&self, account: &str, query: &str) -> Result<Value, Error> {
        self.request(
            Method::GET,
            format!(
                "/{}/search.json?q={}",
                checked_id(account)?,
                cgi_escape(query)
            ),
            None,
        )
        .await
    }
    pub async fn create_card(
        &self,
        account: &str,
        board: &str,
        title: Value,
        description: Option<Value>,
    ) -> Result<Value, Error> {
        let mut fields = vec![format!("\"title\":{title}")];
        if let Some(description) = description.filter(|v| !blank(v)) {
            fields.push(format!("\"description\":{description}"));
        }
        let body = format!("{{\"card\":{{{}}}}}", fields.join(",")).into_bytes();
        self.request(
            Method::POST,
            format!(
                "/{}/boards/{}/cards.json",
                checked_id(account)?,
                checked_id(board)?
            ),
            Some(body),
        )
        .await
    }
    pub async fn create_comment(
        &self,
        account: &str,
        number: &str,
        body: Value,
    ) -> Result<Value, Error> {
        self.request(
            Method::POST,
            format!(
                "/{}/cards/{}/comments.json",
                checked_id(account)?,
                checked_number(number)?
            ),
            Some(serde_json::to_vec(&json!({"comment":{"body":body}})).unwrap()),
        )
        .await
    }
    pub async fn move_to_column(
        &self,
        account: &str,
        number: &str,
        column: Value,
    ) -> Result<bool, Error> {
        // Rails checks the path IDs; column_id is a JSON field and intentionally unvalidated here.
        self.request(
            Method::POST,
            format!(
                "/{}/cards/{}/triage.json",
                checked_id(account)?,
                checked_number(number)?
            ),
            Some(serde_json::to_vec(&json!({"column_id":column})).unwrap()),
        )
        .await?;
        Ok(true)
    }
    pub async fn close_card(&self, account: &str, number: &str) -> Result<bool, Error> {
        self.request(
            Method::POST,
            format!(
                "/{}/cards/{}/closure.json",
                checked_id(account)?,
                checked_number(number)?
            ),
            Some(b"{}".to_vec()),
        )
        .await?;
        Ok(true)
    }
    pub async fn reopen_card(&self, account: &str, number: &str) -> Result<bool, Error> {
        self.request(
            Method::DELETE,
            format!(
                "/{}/cards/{}/closure.json",
                checked_id(account)?,
                checked_number(number)?
            ),
            None,
        )
        .await?;
        Ok(true)
    }
    async fn request(
        &self,
        method: Method,
        path: String,
        body: Option<Vec<u8>>,
    ) -> Result<Value, Error> {
        let parsed = uri::parse(&self.base).map_err(|_| Error::unreachable("Invalid Uri Error"))?;
        let host = parsed
            .host
            .filter(|h| !h.is_empty())
            .ok_or_else(|| Error::unreachable("Argument Error"))?;
        let https = parsed.scheme.as_deref() == Some("https");
        let ip = self
            .network
            .resolver
            .lookup(host.trim_matches(['[', ']']))
            .await
            .map_err(|_| Error::unreachable("Socket Error"))?
            .into_iter()
            .next()
            .ok_or_else(|| Error::unreachable("Socket Error"))?;
        let endpoint = Endpoint {
            https,
            host,
            port: parsed
                .port
                .unwrap_or(if https { 443 } else { 80 })
                .try_into()
                .map_err(|_| Error::unreachable("Argument Error"))?,
            pinned_ip: Some(ip),
        };
        let headers = vec![
            ("Accept".into(), "application/json".into()),
            ("Content-Type".into(), "application/json".into()),
            ("User-Agent".into(), "Smartfire-Fizzy".into()),
            ("Authorization".into(), format!("Bearer {}", self.token)),
        ];
        let mut request =
            Request::net_http(method, path, None, headers).transport(false, &endpoint);
        request.body = body.unwrap_or_default();
        let response = http::exchange(&self.network, &endpoint, request, &TIMEOUTS)
            .await
            .map_err(Error::transport)?;
        let status = response.status;
        let Body::Complete(body) = response
            .read_body(usize::MAX)
            .await
            .map_err(Error::transport)?
        else {
            unreachable!("Rails has no Fizzy body cap")
        };
        decode(status, &body)
    }
}
fn checked_id(id: &str) -> Result<&str, Error> {
    if !id.is_empty()
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
    {
        Ok(id)
    } else {
        Err(Error::new(ErrorKind::Other, "Invalid Fizzy id"))
    }
}
fn checked_number(number: &str) -> Result<&str, Error> {
    if !number.is_empty() && number.bytes().all(|b| b.is_ascii_digit()) {
        Ok(number)
    } else {
        Err(Error::new(ErrorKind::Other, "Invalid Fizzy card number"))
    }
}
fn cgi_escape(query: &str) -> String {
    query
        .bytes()
        .map(|b| match b {
            b' ' => "+".into(),
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_' | b'.' | b'-' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}
pub fn decode(status: u16, body: &[u8]) -> Result<Value, Error> {
    let parsed = if super::error_body::within_nesting_limit(body) {
        serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}))
    } else {
        json!({})
    };
    match status {
        200..=299 => Ok(parsed),
        401 => Err(Error::new(
            ErrorKind::Unauthorized,
            "Fizzy rejected the linked token",
        )),
        404 => Err(Error::new(ErrorKind::NotFound, "Not found in Fizzy")),
        403 | 422 => {
            let message = super::error_body::message(body).map_err(Error::unreachable)?;
            Err(Error::new(
                if status == 403 {
                    ErrorKind::Forbidden
                } else {
                    ErrorKind::Refused
                },
                format!("Fizzy refused: {message}"),
            ))
        }
        _ => Err(Error::new(
            ErrorKind::Other,
            format!("Fizzy returned {status}"),
        )),
    }
}
