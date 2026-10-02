//! `app/models/slack/oauth.rb`: OAuth v2 with user scopes and fixed Slack endpoints.
use crate::integrations::net::{
    Network,
    http::{self, Body, Endpoint, HttpError, Request, Timeouts},
};
use hyper::Method;
use rails_compat::{Secrets, verifiers::OAuthState};
use serde_json::{Value, json};
use std::time::Duration;
use subtle::ConstantTimeEq;

pub const USER_SCOPES: [&str; 11] = [
    "channels:history",
    "channels:read",
    "groups:history",
    "groups:read",
    "im:history",
    "im:read",
    "mpim:history",
    "mpim:read",
    "users:read",
    "users:read.email",
    "team:read",
];
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct Error(pub String);
impl Error {
    fn transport(class: &str) -> Self {
        Self(format!("Could not reach Slack ({class})"))
    }
}
pub fn present(v: &Value) -> bool {
    match v {
        Value::Null | Value::Bool(false) => false,
        Value::String(s) => !campfire_richtext::ruby::is_blank(s),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
        _ => true,
    }
}
pub fn string(v: &Value) -> String {
    campfire_richtext::ruby::json_value_to_s(v)
}
fn truthy(v: &Value) -> bool {
    !v.is_null() && *v != false
}
fn at(v: &Value, key: &str) -> Result<Value, Error> {
    match v {
        Value::Object(m) => Ok(m.get(key).cloned().unwrap_or(Value::Null)),
        Value::Array(_) | Value::Number(_) => Err(Error::transport("Type Error")),
        Value::Null | Value::Bool(_) => Err(Error::transport("No Method Error")),
        Value::String(s) => Ok(if s.contains(key) {
            Value::String(key.into())
        } else {
            Value::Null
        }),
    }
}
pub fn authorize_url(
    client_id: &str,
    redirect_uri: &str,
    state: &str,
    team_id: Option<&str>,
) -> String {
    let mut q = url::form_urlencoded::Serializer::new(String::new());
    q.append_pair("client_id", client_id)
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("state", state)
        .append_pair("user_scope", &USER_SCOPES.join(","));
    if let Some(team) = team_id.filter(|s| !campfire_richtext::ruby::is_blank(s)) {
        q.append_pair("team", team);
    }
    format!("https://slack.com/oauth/v2/authorize?{}", q.finish())
}
pub fn manifest(base_url: &str) -> String {
    // serde's ordered maps preserve the Ruby hash insertion order.
    serde_json::to_string_pretty(&json!({"display_information":{"name":"Smartfire Import"},"settings":{"org_deploy_enabled":false},"oauth_config":{"redirect_urls":[format!("{base_url}/slack/oauth/callback")],"scopes":{"user":USER_SCOPES}}})).expect("manifest contains only JSON literals")
}
pub fn sign_state(secrets: &Secrets, raw: &str) -> String {
    OAuthState::SlackOAuth.generate(secrets, raw)
}
pub fn valid_state(
    secrets: &Secrets,
    signed: &str,
    stored: Option<&Value>,
    user_id: i64,
    now: jiff::Timestamp,
) -> bool {
    let Some(Value::String(verified)) = OAuthState::SlackOAuth.verified(secrets, signed, now)
    else {
        return false;
    };
    let Some(stored) = stored.and_then(Value::as_object) else {
        return false;
    };
    let Some(Value::String(raw)) = stored.get("state") else {
        return false;
    };
    let owner = match stored.get("user_id") {
        Some(Value::Number(n)) => n
            .as_i64()
            .or_else(|| n.as_f64().map(|v| v.trunc() as i64))
            .unwrap_or(0),
        Some(Value::String(s)) => crate::integrations::github::client::ruby_to_i(s),
        None | Some(Value::Null) => 0,
        _ => return false,
    };
    owner == user_id
        && raw.len() == verified.len()
        && bool::from(raw.as_bytes().ct_eq(verified.as_bytes()))
}
pub struct OAuth {
    pub network: Network,
}
impl OAuth {
    pub async fn exchange_code(
        &self,
        client_id: &str,
        client_secret: &str,
        code: &str,
        redirect_uri: &str,
    ) -> Result<Value, Error> {
        let (status, body) = self
            .request(
                "oauth.v2.access",
                Some(&[
                    ("client_id", client_id),
                    ("client_secret", client_secret),
                    ("code", code),
                    ("redirect_uri", redirect_uri),
                ]),
                None,
            )
            .await?;
        exchange_response(status, &body)
    }
    pub async fn revoke(&self, token: &str) -> bool {
        match self
            .request("auth.revoke", Some(&[("token", token)]), None)
            .await
        {
            Ok((_, body)) => serde_json::from_slice::<Value>(&body)
                .ok()
                .and_then(|v| at(&v, "revoked").ok())
                .is_some_and(|v| v == true),
            Err(_) => false,
        }
    }
    pub async fn team_info(&self, token: &str) -> Option<Value> {
        let (_, body) = self.request("team.info", None, Some(token)).await.ok()?;
        let body: Value = serde_json::from_slice(&body).ok()?;
        truthy(&at(&body, "ok").ok()?)
            .then(|| at(&body, "team").ok())
            .flatten()
            .filter(|v| !v.is_null())
    }
    async fn request(
        &self,
        endpoint: &str,
        form: Option<&[(&str, &str)]>,
        token: Option<&str>,
    ) -> Result<(u16, Vec<u8>), Error> {
        let endpoint_config = Endpoint {
            https: true,
            host: "slack.com".into(),
            port: 443,
            pinned_ip: None,
        };
        let mut headers = vec![("Accept".into(), "application/json".into())];
        if form.is_some() {
            headers.insert(
                0,
                (
                    "Content-Type".into(),
                    "application/x-www-form-urlencoded".into(),
                ),
            );
        }
        if let Some(token) = token {
            headers.push(("Authorization".into(), format!("Bearer {token}")));
        }
        let mut request = Request::net_http(
            if form.is_some() {
                Method::POST
            } else {
                Method::GET
            },
            format!("/api/{endpoint}"),
            None,
            headers,
        );
        if let Some(form) = form {
            request.body = url::form_urlencoded::Serializer::new(String::new())
                .extend_pairs(form.iter().copied())
                .finish()
                .into_bytes();
        }
        // Net::HTTP.start is already started, so no implicit Connection: close.
        let request = request.transport(false, &endpoint_config);
        let response = http::exchange(
            &self.network,
            &endpoint_config,
            request,
            &Timeouts {
                open: Duration::from_secs(10),
                read: Duration::from_secs(10),
                write: Duration::from_secs(10),
            },
        )
        .await
        .map_err(transport_error)?;
        let status = response.status;
        let Body::Complete(body) = response
            .read_body(usize::MAX)
            .await
            .map_err(transport_error)?
        else {
            unreachable!("fixed-host unbounded response")
        };
        Ok((status, body))
    }
}
fn exchange_response(status: u16, bytes: &[u8]) -> Result<Value, Error> {
    let body: Value =
        serde_json::from_slice(bytes).map_err(|_| Error::transport("Parser Error"))?;
    if (200..300).contains(&status) && truthy(&at(&body, "ok")?) {
        // Hash#dig is required on the root, even though String#[] accepts substrings.
        if !body.is_object() {
            return Err(Error::transport("No Method Error"));
        }
        let authed = at(&body, "authed_user")?;
        let token = match &authed {
            Value::Null => &Value::Null,
            Value::Object(o) => o.get("access_token").unwrap_or(&Value::Null),
            _ => return Err(Error::transport("Type Error")),
        };
        if present(token) {
            return Ok(body);
        }
    }
    let error = at(&body, "error")?;
    Err(Error(format!(
        "Slack rejected the OAuth grant ({})",
        if present(&error) {
            string(&error)
        } else {
            status.to_string()
        }
    )))
}
fn transport_error(error: HttpError) -> Error {
    let class = match error {
        HttpError::OpenTimeout => "Open Timeout",
        HttpError::ReadTimeout => "Read Timeout",
        HttpError::WriteTimeout => "Write Timeout",
        HttpError::Unresolvable(_) => "Socket Error",
        HttpError::Tls(_) => "Ssl Error",
        HttpError::ConnectionClosed => "Eof Error",
        HttpError::Io(e) => match e.kind() {
            std::io::ErrorKind::ConnectionRefused => "Econnrefused",
            std::io::ErrorKind::ConnectionReset => "Econnreset",
            std::io::ErrorKind::BrokenPipe => "Epipe",
            std::io::ErrorKind::UnexpectedEof => "Eof Error",
            _ => "Io Error",
        },
        HttpError::Inflate(_) => "Error",
        HttpError::Http(_) => "Http Bad Response",
    };
    // Class names only; provider bodies and transport detail may contain credentials.
    tracing::warn!(class, "Slack::OAuth transport failed");
    Error::transport(class)
}
#[cfg(test)]
mod tests;
