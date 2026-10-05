//! OAuth, refresh and Calendar/Drive API semantics from `app/models/google/client.rb`.
//! HTTP uses an injectable transport, outside the database writer.
use super::client::{Client, HttpClient};
use crate::integrations::net::Network;
use base64::{Engine, engine::general_purpose::URL_SAFE};
use campfire_db::{
    Database, Timestamp,
    models::google_account::{CALENDAR_SCOPE, DRIVE_SCOPE, GoogleAccount, UNREADABLE_TOKEN_REASON},
};
use hyper::Method;
use rails_compat::{Secrets, ar_encryption::ArEncryption, calendar_credentials::Snapshot};
use serde_json::{Value, json};
use std::sync::Arc;
#[derive(Debug, Clone, Default)]
pub struct Config {
    pub client_id: String,
    pub client_secret: String,
    pub webhook_url: Option<String>,
}
impl Config {
    pub fn from_env() -> Self {
        Self {
            client_id: std::env::var("GOOGLE_CLIENT_ID").unwrap_or_default(),
            client_secret: std::env::var("GOOGLE_CLIENT_SECRET").unwrap_or_default(),
            webhook_url: std::env::var("GOOGLE_CALENDAR_WEBHOOK_URL")
                .ok()
                .filter(|s| !blank(s)),
        }
    }
    pub fn configured(&self) -> bool {
        !blank(&self.client_id) && !blank(&self.client_secret)
    }
}
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Unavailable(String),
    #[error("{0}")]
    RateLimited(String),
    #[error("{0}")]
    Unauthorized(String),
    #[error("{0}")]
    Forbidden(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Conflict(String),
    #[error("{0}")]
    Rejected(String),
    #[error(transparent)]
    Storage(#[from] campfire_db::Error),
}
impl Error {
    pub fn unavailable(&self) -> bool {
        matches!(self, Self::Unavailable(_) | Self::RateLimited(_))
    }
    pub fn class(&self) -> &'static str {
        match self {
            Self::Unavailable(_) => "Google::Client::Unavailable",
            Self::RateLimited(_) => "Google::Client::RateLimited",
            Self::Unauthorized(_) => "Google::Client::Unauthorized",
            Self::Forbidden(_) => "Google::Client::Forbidden",
            Self::NotFound(_) => "Google::Client::NotFound",
            Self::Conflict(_) => "Google::Client::Conflict",
            _ => "Google::Client::Error",
        }
    }
}
pub type Result<T> = std::result::Result<T, Error>;
pub fn blank(s: &str) -> bool {
    campfire_richtext::ruby::is_blank(s)
}
pub fn integer(v: &Value) -> i64 {
    if let Some(n) = v.as_i64() {
        return n;
    }
    if let Some(n) = v.as_f64() {
        return n as i64;
    }
    campfire_db::models::google_calendar::message_number(v.as_str().unwrap_or_default())
        .unwrap_or(0)
}
pub const EVENTS: &str = "/calendar/v3/calendars/primary/events";
pub const MEETING_FIELDS: &str =
    "items(eventType,start,end,status,transparency,attendees(self,responseStatus)),nextPageToken";
pub const DRIVE_FILE_FIELDS: &str =
    "id,name,mimeType,modifiedTime,owners(displayName),webViewLink,iconLink";
pub const DRIVE_LIST_FIELDS: &str =
    "files(id,name,mimeType,modifiedTime,owners(displayName),webViewLink)";
pub fn query(params: &[(&str, String)]) -> String {
    let mut q = url::form_urlencoded::Serializer::new(String::new());
    for (k, v) in params {
        q.append_pair(k, v);
    }
    q.finish()
}
pub struct Api {
    pub config: Config,
    client: Arc<dyn Client>,
}
impl Default for Api {
    fn default() -> Self {
        Self::new(Config::from_env(), Arc::new(HttpClient(Network::system())))
    }
}
impl Api {
    pub fn new(config: Config, client: Arc<dyn Client>) -> Self {
        Self { config, client }
    }
    pub fn authorize_url(&self, redirect_uri: &str, state: &str, drive: bool) -> String {
        let scope = format!(
            "openid email {CALENDAR_SCOPE}{}",
            if drive {
                format!(" {DRIVE_SCOPE}")
            } else {
                String::new()
            }
        );
        format!(
            "https://accounts.google.com/o/oauth2/v2/auth?{}",
            query(&[
                ("client_id", self.config.client_id.clone()),
                ("redirect_uri", redirect_uri.into()),
                ("response_type", "code".into()),
                ("scope", scope),
                ("access_type", "offline".into()),
                ("prompt", "consent".into()),
                ("state", state.into())
            ])
        )
    }
    async fn form(&self, path: &str, params: &[(&str, String)]) -> Result<(u16, Vec<u8>)> {
        self.client
            .request(
                "oauth2.googleapis.com",
                Method::POST,
                path,
                vec![(
                    "Content-Type".into(),
                    "application/x-www-form-urlencoded".into(),
                )],
                query(params).into_bytes(),
            )
            .await
            .map_err(|error| {
                Error::Unavailable(format!(
                    "{} ({})",
                    if path == "/revoke" {
                        "Google token revoke failed"
                    } else {
                        "Google Calendar request failed"
                    },
                    error.class()
                ))
            })
    }
    pub async fn exchange_code(&self, code: &str, redirect_uri: &str) -> Result<Value> {
        let (status, body) = self
            .form(
                "/token",
                &[
                    ("client_id", self.config.client_id.clone()),
                    ("client_secret", self.config.client_secret.clone()),
                    ("code", code.into()),
                    ("redirect_uri", redirect_uri.into()),
                    ("grant_type", "authorization_code".into()),
                ],
            )
            .await?;
        if (200..300).contains(&status) {
            parse(&body, "Calendar")
        } else {
            let code = serde_json::from_slice::<Value>(&body)
                .ok()
                .and_then(|v| v["error"].as_str().map(str::to_owned))
                .unwrap_or_default();
            Err(Error::Rejected(
                format!("Google token exchange failed ({status} {code})")
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" "),
            ))
        }
    }
    pub fn email_from_id_token(&self, token: &str, now: Timestamp) -> Result<String> {
        let reject = || Error::Rejected("Google rejected the connection".into());
        let segments = token.trim_end_matches('.').split('.').collect::<Vec<_>>();
        if segments.len() != 3 {
            return Err(reject());
        }
        let segment = format!(
            "{}{}",
            segments[1],
            "=".repeat((4 - segments[1].len() % 4) % 4)
        );
        let body = URL_SAFE.decode(segment).map_err(|_| reject())?;
        let v: Value = serde_json::from_slice(&body).map_err(|_| reject())?;
        if !v.is_object()
            || !matches!(
                v["iss"].as_str(),
                Some("https://accounts.google.com" | "accounts.google.com")
            )
            || v["aud"].as_str() != Some(&self.config.client_id)
            || integer(&v["exp"]) <= now.as_second()
        {
            return Err(reject());
        }
        Ok(v["email"]
            .as_str()
            .filter(|s| !blank(s))
            .ok_or_else(reject)?
            .into())
    }
    pub async fn revoke_token(&self, token: Option<&str>) -> Result<bool> {
        let (status, _) = self
            .form("/revoke", &[("token", token.unwrap_or_default().into())])
            .await?;
        match status {
            200..=299 | 400 => Ok(true),
            429 | 500..=599 => Err(Error::Unavailable(format!(
                "Google token revoke failed ({status})"
            ))),
            _ => Ok(false),
        }
    }
    async fn refresh(
        &self,
        credentials: &mut Credentials,
        db: &Database,
        secrets: &Secrets,
        now: Timestamp,
    ) -> Result<()> {
        let (status, body) = self
            .form(
                "/token",
                &[
                    ("client_id", self.config.client_id.clone()),
                    ("client_secret", self.config.client_secret.clone()),
                    (
                        "refresh_token",
                        credentials.tokens.refresh_token.clone().unwrap_or_default(),
                    ),
                    ("grant_type", "refresh_token".into()),
                ],
            )
            .await?;
        if (200..300).contains(&status) {
            let tokens = parse(&body, "Calendar")?;
            let access = tokens["access_token"].as_str().map(str::to_owned);
            let expiry = now.since(jiff::SignedDuration::from_secs(integer(
                &tokens["expires_in"],
            )));
            if let Some(mut account) = credentials.account.clone() {
                let enc = ArEncryption::new(secrets);
                let access = access.clone();
                credentials.account = Some(
                    db.write(move |tx| {
                        account.refresh_access(tx, &enc, access.as_deref(), expiry)?;
                        Ok(account)
                    })
                    .await?,
                );
            }
            credentials.tokens.access_token = access;
            credentials.tokens.access_token_expires_at = Some(expiry.jiff());
            return Ok(());
        }
        let code = serde_json::from_slice::<Value>(&body)
            .ok()
            .and_then(|v| v["error"].as_str().map(str::to_owned));
        if status == 400 && code.as_deref() == Some("invalid_grant") {
            if let Some(mut account) = credentials.account.clone() {
                db.write(move |tx| account.mark_disconnected(tx, "Google rejected the connection"))
                    .await?;
            }
            return Err(Error::Unauthorized(
                "Google rejected the refresh token".into(),
            ));
        }
        Err(match status {
            429 => Error::Unavailable("Google token refresh rate limited (429)".into()),
            500..=599 => Error::Unavailable(format!("Google token refresh failed ({status})")),
            _ => Error::Rejected(format!("Google token refresh failed ({status})")),
        })
    }
    pub async fn credentials(
        &self,
        db: &Database,
        secrets: &Secrets,
        user_id: i64,
    ) -> Result<Credentials> {
        let account = db
            .read(move |c| GoogleAccount::for_user(c, user_id))
            .await?
            .ok_or_else(|| Error::Unauthorized("Google rejected the connection".into()))?;
        self.credentials_from_account(db, secrets, account).await
    }
    /// Use a Rails-preloaded account snapshot without repeating its association SELECT.
    pub(crate) async fn credentials_from_account(
        &self,
        db: &Database,
        secrets: &Secrets,
        account: GoogleAccount,
    ) -> Result<Credentials> {
        let enc = ArEncryption::new(secrets);
        match (account.access_token(&enc), account.refresh_token(&enc)) {
            (Ok(access_token), Ok(refresh_token)) => Ok(Credentials {
                tokens: Snapshot {
                    access_token,
                    refresh_token,
                    access_token_expires_at: account.access_token_expires_at.map(Timestamp::jiff),
                },
                account: Some(account),
            }),
            _ => {
                let mut account = account;
                db.write(move |tx| account.mark_disconnected(tx, UNREADABLE_TOKEN_REASON))
                    .await?;
                Err(Error::Unauthorized("Google token could not be read".into()))
            }
        }
    }
    pub async fn request(
        &self,
        db: &Database,
        secrets: &Secrets,
        user_id: i64,
        request: ApiRequest<'_>,
        now: Timestamp,
    ) -> Result<Value> {
        let mut credentials = self.credentials(db, secrets, user_id).await?;
        self.request_with(&mut credentials, db, secrets, request, now)
            .await
    }
    pub async fn request_with(
        &self,
        credentials: &mut Credentials,
        db: &Database,
        secrets: &Secrets,
        request: ApiRequest<'_>,
        now: Timestamp,
    ) -> Result<Value> {
        if credentials.tokens.access_token.as_deref().is_none_or(blank)
            || credentials
                .tokens
                .access_token_expires_at
                .is_none_or(|t| t <= now.jiff())
        {
            self.refresh(credentials, db, secrets, now).await?;
        }
        let (mut status, mut body) = self.send(credentials, &request).await?;
        if status == 401 {
            self.refresh(credentials, db, secrets, now).await?;
            (status, body) = self.send(credentials, &request).await?;
        }
        classify(status, &body, request.drive)
    }
    async fn send(
        &self,
        credentials: &Credentials,
        request: &ApiRequest<'_>,
    ) -> Result<(u16, Vec<u8>)> {
        let target = match request.query {
            Some(q) => format!("{}?{q}", request.path),
            None => request.path.into(),
        };
        let headers = vec![
            ("Content-Type".into(), "application/json".into()),
            (
                "Authorization".into(),
                format!(
                    "Bearer {}",
                    credentials
                        .tokens
                        .access_token
                        .as_deref()
                        .unwrap_or_default()
                ),
            ),
        ];
        self.client
            .request(
                "www.googleapis.com",
                request.method.clone(),
                &target,
                headers,
                request
                    .payload
                    .map(|p| serde_json::to_vec(p).expect("JSON value"))
                    .unwrap_or_default(),
            )
            .await
            .map_err(|error| {
                Error::Unavailable(format!(
                    "Google {} request failed ({})",
                    if request.drive { "Drive" } else { "Calendar" },
                    error.class()
                ))
            })
    }
    pub async fn drive_file(
        &self,
        db: &Database,
        secrets: &Secrets,
        user_id: i64,
        file_id: &str,
        now: Timestamp,
    ) -> Result<Value> {
        let path = format!("/drive/v3/files/{file_id}");
        let q = query(&[
            ("fields", DRIVE_FILE_FIELDS.into()),
            ("supportsAllDrives", "true".into()),
        ]);
        self.request(
            db,
            secrets,
            user_id,
            ApiRequest {
                method: Method::GET,
                path: &path,
                query: Some(&q),
                payload: None,
                drive: true,
            },
            now,
        )
        .await
    }
    pub async fn list_drive_files(
        &self,
        db: &Database,
        secrets: &Secrets,
        user_id: i64,
        term: &str,
        now: Timestamp,
    ) -> Result<Value> {
        let escaped = term
            .chars()
            .flat_map(|c| {
                if matches!(c, '\\' | '\'') {
                    vec!['\\', c]
                } else {
                    vec![c]
                }
            })
            .collect::<String>();
        let drive_query = if blank(term) {
            "trashed=false".into()
        } else {
            format!("name contains '{escaped}' and trashed=false")
        };
        let q = query(&[
            ("q", drive_query),
            ("pageSize", "10".into()),
            ("fields", DRIVE_LIST_FIELDS.into()),
            ("orderBy", "modifiedTime desc".into()),
            ("spaces", "drive".into()),
        ]);
        self.request(
            db,
            secrets,
            user_id,
            ApiRequest {
                method: Method::GET,
                path: "/drive/v3/files",
                query: Some(&q),
                payload: None,
                drive: true,
            },
            now,
        )
        .await
    }
    pub async fn list_events(
        &self,
        db: &Database,
        secrets: &Secrets,
        user_id: i64,
        time_min: Timestamp,
        time_max: Timestamp,
        now: Timestamp,
    ) -> Result<Value> {
        let mut items = vec![];
        let mut page = None;
        for _ in 0..4 {
            let mut params = vec![
                ("singleEvents", "true".into()),
                ("orderBy", "startTime".into()),
                ("maxResults", "250".into()),
                (
                    "timeMin",
                    time_min.jiff().strftime("%Y-%m-%dT%H:%M:%SZ").to_string(),
                ),
                (
                    "timeMax",
                    time_max.jiff().strftime("%Y-%m-%dT%H:%M:%SZ").to_string(),
                ),
                ("fields", MEETING_FIELDS.into()),
            ];
            if let Some(p) = page {
                params.push(("pageToken", p));
            }
            let q = query(&params);
            let response = self
                .request(
                    db,
                    secrets,
                    user_id,
                    ApiRequest {
                        method: Method::GET,
                        path: EVENTS,
                        query: Some(&q),
                        payload: None,
                        drive: false,
                    },
                    now,
                )
                .await?;
            if let Some(a) = response["items"].as_array() {
                items.extend(a.iter().cloned());
            }
            page = response["nextPageToken"]
                .as_str()
                .filter(|s| !blank(s))
                .map(str::to_owned);
            if page.is_none() {
                break;
            }
        }
        Ok(json!({"items":items}))
    }
}
pub struct Credentials {
    pub tokens: Snapshot,
    account: Option<GoogleAccount>,
}
impl Credentials {
    pub fn snapshot(tokens: Snapshot) -> Self {
        Self {
            tokens,
            account: None,
        }
    }
}
pub struct ApiRequest<'a> {
    pub method: Method,
    pub path: &'a str,
    pub query: Option<&'a str>,
    pub payload: Option<&'a Value>,
    pub drive: bool,
}
impl<'a> ApiRequest<'a> {
    pub fn calendar(method: Method, path: &'a str, payload: Option<&'a Value>) -> Self {
        Self {
            method,
            path,
            query: None,
            payload,
            drive: false,
        }
    }
}
fn parse(body: &[u8], service: &str) -> Result<Value> {
    if body.is_empty() {
        return Ok(Value::Bool(true));
    }
    serde_json::from_slice(body).map_err(|_| {
        Error::Unavailable(format!(
            "Google {service} request failed (JSON::ParserError)"
        ))
    })
}
fn classify(status: u16, body: &[u8], drive: bool) -> Result<Value> {
    let service = if drive { "Drive" } else { "Calendar" };
    let missing = || Error::NotFound(format!("Google {} entry not found", service.to_lowercase()));
    Err(match status {
        200..=299 => return parse(body, service),
        401 => Error::Unauthorized("Google rejected the request (401)".into()),
        404 | 410 => missing(),
        403 if drive => missing(),
        403 => {
            let v = serde_json::from_slice::<Value>(body).unwrap_or(Value::Null);
            let reason = v["error"]["errors"].as_array().and_then(|a| {
                a.iter().filter_map(|e| e["reason"].as_str()).find(|s| {
                    matches!(
                        *s,
                        "rateLimitExceeded"
                            | "userRateLimitExceeded"
                            | "quotaExceeded"
                            | "dailyLimitExceeded"
                    )
                })
            });
            match reason {
                Some(r) => {
                    Error::RateLimited(format!("Google Calendar request rate limited ({r})"))
                }
                None => Error::Forbidden("Google Calendar request forbidden (403)".into()),
            }
        }
        429 => Error::RateLimited(format!("Google {service} request rate limited (429)")),
        409 => Error::Conflict("Google calendar entry already exists".into()),
        _ => Error::Rejected(format!("Google {service} request failed ({status})")),
    })
}
