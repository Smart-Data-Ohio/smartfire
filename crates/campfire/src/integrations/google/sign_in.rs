use std::sync::{Arc, Mutex};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use jiff::Timestamp;
use rails_compat::jwt::{
    RsaPublicKey,
    google::{self, IdTokenError, Jwks},
};
use rails_compat::{MessageVerifier, Secrets};
use rand::RngCore;
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

use crate::net::{Network, http};

pub const FLOW_SESSION_KEY: &str = "google_sign_in_request";
pub const FLOW_TTL: i64 = 600;
const KEY_TTL: i64 = 3600;

/// No token, code or claim values are included in errors or logs.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("Google sign-in rejected ({0})")]
    Rejected(&'static str),
    #[error("Google sign-in unavailable")]
    Unavailable,
}
impl From<IdTokenError> for Error {
    fn from(error: IdTokenError) -> Self {
        match error {
            IdTokenError::Rejected(reason) => Self::Rejected(reason.as_str()),
            IdTokenError::Unavailable => Self::Unavailable,
        }
    }
}

#[derive(Clone)]
pub struct Config {
    pub client_id: String,
    pub client_secret: String,
    pub domains: Vec<String>,
}
impl Config {
    pub fn from_env() -> Self {
        Self {
            client_id: std::env::var("GOOGLE_CLIENT_ID").unwrap_or_default(),
            client_secret: std::env::var("GOOGLE_CLIENT_SECRET").unwrap_or_default(),
            domains: google::allowed_domains(
                std::env::var(google::DOMAINS_ENV_VAR).ok().as_deref(),
            ),
        }
    }
    pub fn configured(&self) -> bool {
        !self.client_id.chars().all(char::is_whitespace)
            && !self.client_secret.chars().all(char::is_whitespace)
            && !self.domains.is_empty()
    }
}

pub struct SignIn {
    pub config: Config,
    client: Arc<dyn super::client::Client>,
    keys: Mutex<Option<(i64, Jwks)>>,
}
impl SignIn {
    pub fn new(config: Config, network: Network) -> Self {
        Self::with_client(config, Arc::new(super::client::HttpClient(network)))
    }
    pub fn with_client(config: Config, client: Arc<dyn super::client::Client>) -> Self {
        Self {
            config,
            client,
            keys: Mutex::new(None),
        }
    }

    pub fn authorize_url(
        &self,
        redirect_uri: &str,
        state: &str,
        nonce: &str,
        challenge: &str,
        purpose: &str,
    ) -> String {
        let mut params = vec![
            ("client_id", self.config.client_id.as_str()),
            ("redirect_uri", redirect_uri),
            ("response_type", "code"),
            ("scope", "openid email profile"),
            ("state", state),
            ("nonce", nonce),
            ("code_challenge", challenge),
            ("code_challenge_method", "S256"),
        ];
        if fresh_login(purpose) {
            params.extend([("prompt", "login"), ("max_age", "0")]);
        }
        format!(
            "https://accounts.google.com/o/oauth2/v2/auth?{}",
            form(&params)
        )
    }

    /// Returns the session hash and redirect, using the Rails app verifier with no purpose/expiry
    /// metadata. The expiry belongs to the browser's session request instead.
    pub fn start(
        &self,
        secrets: &Secrets,
        redirect_uri: &str,
        purpose: &str,
        user_id: Option<i64>,
        now: Timestamp,
    ) -> (Value, String) {
        let mut random = rand::rng();
        let mut state = [0; 16];
        let mut nonce = [0; 16];
        let mut verifier = [0; 32];
        random.fill_bytes(&mut state);
        random.fill_bytes(&mut nonce);
        random.fill_bytes(&mut verifier);
        // WS16 flagged test-only input seam; verification and state signing stay real.
        #[cfg(test)]
        if let Some(bytes) = crate::test_support::oauth_entropy() {
            state.copy_from_slice(&bytes[..16]);
            nonce.copy_from_slice(&bytes[16..32]);
            verifier.copy_from_slice(&bytes[32..]);
        }
        let hex = |bytes: &[u8]| bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
        let (state, nonce, verifier) = (hex(&state), hex(&nonce), URL_SAFE_NO_PAD.encode(verifier));
        let challenge = pkce_challenge(&verifier);
        let signed = state_verifier(secrets).generate(&json!(state), None, None);
        let mut flow = json!({"state":state,"nonce":nonce,"verifier":verifier,"exp":now.as_second()+FLOW_TTL,"purpose":purpose});
        if let Some(user_id) = user_id {
            flow["user_id"] = user_id.into();
        }
        let url = self.authorize_url(redirect_uri, &signed, &nonce, &challenge, purpose);
        (flow, url)
    }

    pub async fn exchange_code(
        &self,
        code: &str,
        redirect_uri: &str,
        verifier: &str,
    ) -> Result<String, Error> {
        let body = form(&[
            ("client_id", &self.config.client_id),
            ("client_secret", &self.config.client_secret),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("grant_type", "authorization_code"),
            ("code_verifier", verifier),
        ]);
        let (status, body) = self
            .request("oauth2.googleapis.com", "/token", Some(body))
            .await?;
        if !(200..300).contains(&status) {
            return Err(Error::Rejected("denied"));
        }
        let payload: Value = serde_json::from_slice(&body).map_err(|_| Error::Unavailable)?;
        if !payload.is_object() {
            return Err(Error::Unavailable);
        }
        payload
            .get("id_token")
            .and_then(Value::as_str)
            .filter(|s| !s.chars().all(char::is_whitespace))
            .map(str::to_owned)
            .ok_or(Error::Rejected("bad_token"))
    }

    pub async fn verify(
        &self,
        token: &str,
        nonce: &str,
        purpose: &str,
        now: i64,
    ) -> Result<Map<String, Value>, Error> {
        if token.chars().all(char::is_whitespace) || nonce.chars().all(char::is_whitespace) {
            return Err(Error::Rejected("bad_token"));
        }
        let (header, _) = rails_compat::jwt::decode_unverified(token)
            .map_err(|_| Error::Rejected("bad_token"))?;
        let kid = header.get("kid").unwrap_or(&Value::Null);
        if header.get("alg") != Some(&json!("RS256")) || json_blank(kid) {
            return Err(Error::Rejected("bad_token"));
        }
        let key = self
            .public_key_for(&campfire_richtext::ruby::json_value_to_s(kid), now)
            .await?;
        google::verify_id_token(
            token,
            nonce,
            fresh_login(purpose).then_some(google::FRESH_LOGIN_MAX_AUTH_AGE),
            &google::Config {
                client_id: Some(&self.config.client_id),
                allowed_domains: &self.config.domains,
            },
            |_| Ok(key),
            now,
        )
        .map_err(Error::from)
    }

    async fn public_key_for(&self, kid: &str, now: i64) -> Result<RsaPublicKey, Error> {
        let cached = self
            .keys
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .filter(|(at, _)| *at > now - KEY_TTL)
            .map(|(_, keys)| keys.clone());
        let keys = match cached {
            Some(keys) => keys,
            None => self.fetch_keys(now).await?,
        };
        match keys.get(kid) {
            Some(key) => Ok(key.clone()),
            None => self
                .fetch_keys(now)
                .await?
                .key_for(kid)
                .map_err(Error::from),
        }
    }

    async fn fetch_keys(&self, now: i64) -> Result<Jwks, Error> {
        let (status, body) = self
            .request("www.googleapis.com", "/oauth2/v3/certs", None)
            .await?;
        if !(200..300).contains(&status) {
            return Err(Error::Unavailable);
        }
        let keys = Jwks::parse(&body)?;
        *self.keys.lock().unwrap_or_else(|e| e.into_inner()) = Some((now, keys.clone()));
        Ok(keys)
    }

    async fn request(
        &self,
        host: &str,
        target: &str,
        body: Option<String>,
    ) -> Result<(u16, Vec<u8>), Error> {
        let method = if body.is_some() {
            hyper::Method::POST
        } else {
            hyper::Method::GET
        };
        let headers = body
            .as_ref()
            .map(|_| {
                vec![(
                    "Content-Type".into(),
                    "application/x-www-form-urlencoded".into(),
                )]
            })
            .unwrap_or_default();
        self.client
            .request(
                host,
                method,
                target,
                headers,
                body.unwrap_or_default().into_bytes(),
            )
            .await
            .map_err(|_| Error::Unavailable)
    }
}

pub fn pkce_challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}
pub fn state_verifier(secrets: &Secrets) -> MessageVerifier {
    rails_compat::app_verifier(secrets, "google_sign_in_state")
}
pub fn fresh_login(purpose: &str) -> bool {
    matches!(purpose, "reauth" | "sudo")
}

/// The controller must remove the flow from the session BEFORE checking it, including failures.
pub fn valid_flow(flow: &Value, signed: &str, secrets: &Secrets, now: Timestamp) -> bool {
    let Some(flow) = flow.as_object() else {
        return false;
    };
    let Some(state) = flow.get("state").and_then(Value::as_str) else {
        return false;
    };
    if flow.get("nonce").and_then(Value::as_str).is_none()
        || flow.get("verifier").and_then(Value::as_str).is_none()
    {
        return false;
    }
    let expiry = flow.get("exp").and_then(|v| {
        v.as_i64()
            .map(i128::from)
            .or_else(|| v.as_u64().map(i128::from))
    });
    if !expiry.is_some_and(|expiry| expiry > i128::from(now.as_second())) {
        return false;
    }
    let Ok(Value::String(verified)) = state_verifier(secrets).verify(signed, None, now) else {
        return false;
    };
    bool::from(verified.as_bytes().ct_eq(state.as_bytes()))
}

pub fn safe_return_path(stored: Option<&str>, host: &str) -> Option<String> {
    let stored = stored.filter(|s| !s.chars().all(char::is_whitespace))?;
    let uri = campfire_richtext::uri::parse(stored).ok()?;
    let target = if uri.scheme.is_none() {
        uri.to_s()
    } else if uri.host.as_deref() == Some(host) && uri.is_http() {
        http::request_uri(&uri)
    } else {
        return None;
    };
    (target.starts_with('/') && !target.starts_with("//") && !target.starts_with("/\\"))
        .then_some(target)
}

fn json_blank(value: &Value) -> bool {
    match value {
        Value::Null | Value::Bool(false) => true,
        Value::String(s) => s.chars().all(char::is_whitespace),
        Value::Array(a) => a.is_empty(),
        Value::Object(o) => o.is_empty(),
        _ => false,
    }
}

fn form(pairs: &[(&str, &str)]) -> String {
    url::form_urlencoded::Serializer::new(String::new())
        .extend_pairs(pairs.iter().copied())
        .finish()
}

#[cfg(test)]
mod tests;
