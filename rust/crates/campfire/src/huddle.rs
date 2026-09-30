//! LiveKit protocols from `app/services/huddle.rb` and `huddle/room_service.rb`.
//! Database lifecycle lives in `campfire_db`; this module has no rendering dependencies.
use std::time::Duration;

use campfire_richtext::uri::{self, Uri};
use rails_compat::jwt::livekit;
use serde_json::{Map, Value};

use crate::integrations::net::Network;
use crate::integrations::net::http::{self, Endpoint, HttpError, Timeouts};

fn blank(value: &str) -> bool {
    value.chars().all(char::is_whitespace)
}

pub const OPEN_TIMEOUT: Duration = Duration::from_secs(3);
pub const READ_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone, Default)]
pub struct Config {
    pub public_url: Option<String>,
    pub internal_url: Option<String>,
    pub api_key: Option<String>,
    pub api_secret: Option<String>,
    pub gateway_secret: Option<String>,
}

impl Config {
    pub fn from_env() -> Self {
        Self::from_lookup(|name| std::env::var(name).ok())
    }

    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Self {
        let present = |name| lookup(name).filter(|s| !blank(s));
        Self {
            public_url: present("LIVEKIT_URL"),
            internal_url: present("LIVEKIT_INTERNAL_URL"),
            api_key: present("LIVEKIT_API_KEY"),
            api_secret: present("LIVEKIT_API_SECRET"),
            gateway_secret: present("LIVEKIT_GATEWAY_SECRET"),
        }
    }

    pub fn signing_configured(&self) -> bool {
        self.api_key.is_some() && self.api_secret.is_some()
    }

    pub fn admin_configured(&self) -> bool {
        self.internal_url.is_some() && self.signing_configured()
    }
    pub fn configured(&self) -> bool {
        if !self.admin_configured() || self.gateway_secret.is_none() {
            return false;
        }
        let public = self.public_url.as_deref().and_then(endpoint_address);
        let internal = self.internal_url.as_deref().and_then(endpoint_address);
        matches!((public, internal), (Some(public), Some(internal)) if public != internal)
    }
}
fn endpoint_address(value: &str) -> Option<(String, u64)> {
    let uri = uri::parse(value).ok()?;
    let default_port = match uri.scheme.as_deref()?.to_ascii_lowercase().as_str() {
        "ws" | "http" => 80,
        "wss" | "https" => 443,
        _ => return None,
    };
    let host = uri.host.filter(|s| !blank(s))?;
    Some((host.to_lowercase(), uri.port.unwrap_or(default_port)))
}

/// Safe errors contain neither upstream bodies nor configured credentials/URLs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerError {
    pub status: Option<u16>,
    pub code: Option<&'static str>,
}

impl std::fmt::Display for ServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let status = self
            .status
            .map_or_else(|| "unknown".into(), |status| status.to_string());
        write!(
            f,
            "LiveKit huddle request failed (status={status}, code={})",
            self.code.unwrap_or("unknown")
        )
    }
}
impl std::error::Error for ServerError {}

impl ServerError {
    fn code(code: &'static str) -> Self {
        Self {
            status: None,
            code: Some(code),
        }
    }
    fn http(error: HttpError) -> Self {
        let code = match error {
            HttpError::OpenTimeout => "Net::OpenTimeout",
            HttpError::ReadTimeout => "Net::ReadTimeout",
            HttpError::Unresolvable(_) => "SocketError",
            HttpError::Tls(_) => "OpenSSL::SSL::SSLError",
            HttpError::Io(error) => match error.raw_os_error() {
                Some(libc::ECONNREFUSED) => "Errno::ECONNREFUSED",
                Some(libc::ECONNRESET) => "Errno::ECONNRESET",
                Some(libc::EHOSTUNREACH) => "Errno::EHOSTUNREACH",
                Some(libc::ENETUNREACH) => "Errno::ENETUNREACH",
                Some(libc::EPIPE) => "Errno::EPIPE",
                _ => "IOError",
            },
            HttpError::Http(_) => "IOError",
            HttpError::Inflate(_) => "Zlib::Error",
        };
        Self::code(code)
    }
}

/// URI::HTTP/HTTPS.build drops userinfo, query and fragment, changes ws[s] to http[s],
/// and removes exactly one trailing slash from the configured prefix.
fn endpoint_uri(source: &str, action: &str) -> Result<Uri, ServerError> {
    let source = uri::parse(source).map_err(|_| ServerError::code("URI::InvalidURIError"))?;
    let scheme = match source
        .scheme
        .as_deref()
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "ws" | "http" => "http",
        "wss" | "https" => "https",
        _ => return Err(ServerError::code("URI::InvalidURIError")),
    };
    let host = source
        .host
        .filter(|s| !blank(s))
        .ok_or_else(|| ServerError::code("URI::InvalidURIError"))?;
    let prefix = source.path.as_deref().unwrap_or("");
    let path = format!(
        "{}/twirp/livekit.RoomService/{action}",
        prefix.strip_suffix('/').unwrap_or(prefix)
    );
    Ok(Uri {
        scheme: Some(scheme.into()),
        host: Some(host),
        port: source.port,
        path: Some(path),
        userinfo: None,
        opaque: None,
        query: None,
        fragment: None,
    })
}

#[derive(Clone)]
pub struct RoomService {
    config: Config,
    network: Network,
}

impl RoomService {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            network: Network::system(),
        }
    }

    pub fn admin_configured(&self) -> bool {
        self.config.admin_configured()
    }

    pub async fn remove_participant(
        &self,
        room_name: &str,
        identity: &str,
        now: i64,
    ) -> Result<(), ServerError> {
        let body = serde_json::json!({ "room": room_name, "identity": identity });
        let grant = serde_json::json!({ "roomAdmin": true, "room": room_name });
        self.post(
            "RemoveParticipant",
            body,
            grant.as_object().unwrap(),
            now,
            &Timeouts {
                open: OPEN_TIMEOUT,
                read: READ_TIMEOUT,
            },
        )
        .await
    }

    pub async fn delete_room(&self, room_name: &str, now: i64) -> Result<(), ServerError> {
        let body = serde_json::json!({ "room": room_name });
        let grant = serde_json::json!({ "roomCreate": true });
        self.post(
            "DeleteRoom",
            body,
            grant.as_object().unwrap(),
            now,
            &Timeouts {
                open: OPEN_TIMEOUT,
                read: READ_TIMEOUT,
            },
        )
        .await
    }

    async fn post(
        &self,
        action: &str,
        body: Value,
        grant: &Map<String, Value>,
        now: i64,
        timeouts: &Timeouts,
    ) -> Result<(), ServerError> {
        let source = self
            .config
            .internal_url
            .as_deref()
            .ok_or_else(|| ServerError::code("KeyError"))?;
        let uri = endpoint_uri(source, action)?;
        let api_key = self
            .config
            .api_key
            .as_deref()
            .ok_or_else(|| ServerError::code("KeyError"))?;
        let secret = self
            .config
            .api_secret
            .as_deref()
            .ok_or_else(|| ServerError::code("KeyError"))?;
        let https = uri.scheme.as_deref() == Some("https");
        let port = uri
            .port
            .and_then(|p| u16::try_from(p).ok())
            .ok_or_else(|| ServerError::code("URI::InvalidURIError"))?;
        let endpoint = Endpoint {
            https,
            host: uri.host.clone().unwrap(),
            port,
            pinned_ip: None,
        };
        let headers = vec![
            ("Accept".into(), "application/json".into()),
            (
                "Authorization".into(),
                format!(
                    "Bearer {}",
                    livekit::admin_token(api_key, secret, grant, now)
                ),
            ),
            ("Content-Type".into(), "application/json".into()),
        ];
        let mut request = http::Request::net_http(
            hyper::Method::POST,
            http::request_uri(&uri),
            Some(endpoint.host_header()),
            headers,
        );
        request.body = serde_json::to_string(&body)
            .expect("JSON body")
            .into_bytes();
        let response = http::exchange(&self.network, &endpoint, request, timeouts)
            .await
            .map_err(ServerError::http)?;
        let status = response.status;
        // Net::HTTP reads the response before returning to RoomService. It does not follow
        // redirects. The body is discarded even when the status is an error.
        response
            .read_body(usize::MAX)
            .await
            .map_err(ServerError::http)?;
        if (200..300).contains(&status) || status == 404 {
            Ok(())
        } else {
            Err(ServerError {
                status: Some(status),
                code: None,
            })
        }
    }
}

#[cfg(test)]
mod tests;
