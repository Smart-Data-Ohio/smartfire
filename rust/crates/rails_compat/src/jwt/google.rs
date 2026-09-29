//! Google Sign-In ID tokens: `Google::SignIn::IdTokenVerifier.verify!`
//! (`app/models/google/sign_in/id_token_verifier.rb`), the JWKS parsing in
//! `Google::SignIn::KeyStore#fetch_keys!` (`app/models/google/sign_in/key_store.rb`), and
//! `Google::SignIn.allowed_domains` (`app/models/google/sign_in.rb`).
//!
//! Fetching and caching Google's keys (an hour, refetched once on an unknown `kid`) is the app's
//! job: [`verify_id_token`] asks a closure for the key.
use serde_json::{Map, Value};

use super::{Key, RsaPublicKey, Validation, decode, decode_unverified};
use crate::message_verifier::constant_time_eq;
use crate::metadata::ruby_to_s;
use crate::{encoding, ruby};

pub const JWKS_URI: &str = "https://www.googleapis.com/oauth2/v3/certs";
/// `Google::SignIn::ISSUERS`.
pub const ISSUERS: [&str; 2] = ["https://accounts.google.com", "accounts.google.com"];
/// `Google::SignIn::CLOCK_SKEW`, in seconds.
pub const CLOCK_SKEW: i64 = 30;
/// `Google::SignIn::FRESH_LOGIN_MAX_AUTH_AGE`, in seconds (re-auth and sudo).
pub const FRESH_LOGIN_MAX_AUTH_AGE: i64 = 300;
/// `Google::SignIn::DOMAINS_ENV_VAR`.
pub const DOMAINS_ENV_VAR: &str = "GOOGLE_SIGN_IN_DOMAINS";

/// `Google::SignIn::Rejected#reason`. The controllers map these to user-facing messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rejection {
    BadToken,
    UnknownKey,
    BadAudience,
    Expired,
    StaleAuth,
    MissingEmail,
    UnverifiedEmail,
    BadNonce,
    WrongDomain,
}

impl Rejection {
    pub fn as_str(self) -> &'static str {
        match self {
            Rejection::BadToken => "bad_token",
            Rejection::UnknownKey => "unknown_key",
            Rejection::BadAudience => "bad_audience",
            Rejection::Expired => "expired",
            Rejection::StaleAuth => "stale_auth",
            Rejection::MissingEmail => "missing_email",
            Rejection::UnverifiedEmail => "unverified_email",
            Rejection::BadNonce => "bad_nonce",
            Rejection::WrongDomain => "wrong_domain",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum IdTokenError {
    /// `Google::SignIn::Rejected`.
    #[error("Google sign-in rejected ({})", .0.as_str())]
    Rejected(Rejection),
    /// `Google::SignIn::Unavailable`: Google's keys couldn't be fetched.
    #[error("Google sign-in unavailable")]
    Unavailable,
}

impl From<Rejection> for IdTokenError {
    fn from(rejection: Rejection) -> Self {
        IdTokenError::Rejected(rejection)
    }
}

/// Google's signing keys by `kid`, as `KeyStore#fetch_keys!` keeps them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Jwks {
    keys: Vec<(String, RsaPublicKey)>,
}

impl Jwks {
    /// `fetch_keys!` on a response body: a JSON object with a `keys` array, keeping each RSA JWK
    /// whose `kid`, `n` and `e` are present strings and whose `n` and `e` decode. A later JWK
    /// with the same `kid` replaces an earlier one. No usable key (or not that shape) is
    /// [`IdTokenError::Unavailable`].
    pub fn parse(body: &[u8]) -> Result<Self, IdTokenError> {
        let payload: Value = serde_json::from_slice(body).map_err(|_| IdTokenError::Unavailable)?;
        let jwks = payload.get("keys").and_then(Value::as_array).filter(|_| payload.is_object()).ok_or(IdTokenError::Unavailable)?;

        let mut keys: Vec<(String, RsaPublicKey)> = Vec::new();
        for jwk in jwks {
            let Some((kid, key)) = rsa_from_jwk(jwk) else { continue };
            match keys.iter_mut().find(|(existing, _)| *existing == kid) {
                Some(slot) => slot.1 = key,
                None => keys.push((kid, key)),
            }
        }
        if keys.is_empty() {
            return Err(IdTokenError::Unavailable);
        }
        Ok(Self { keys })
    }

    pub fn kids(&self) -> impl Iterator<Item = &str> {
        self.keys.iter().map(|(kid, _)| kid.as_str())
    }

    pub fn get(&self, kid: &str) -> Option<&RsaPublicKey> {
        self.keys.iter().find(|(existing, _)| existing == kid).map(|(_, key)| key)
    }

    /// `KeyStore.public_key_for(kid)` against these keys (after any refetch): a missing `kid` is
    /// rejected as `unknown_key`.
    pub fn key_for(&self, kid: &str) -> Result<RsaPublicKey, IdTokenError> {
        self.get(kid).cloned().ok_or(IdTokenError::Rejected(Rejection::UnknownKey))
    }
}

fn rsa_from_jwk(jwk: &Value) -> Option<(String, RsaPublicKey)> {
    let jwk = jwk.as_object()?;
    if jwk.get("kty") != Some(&Value::from("RSA")) {
        return None;
    }
    let field = |name: &str| jwk.get(name).and_then(Value::as_str).filter(|value| !ruby::str_blank(value));
    let (kid, n, e) = (field("kid")?, field("n")?, field("e")?);
    let key = RsaPublicKey::from_components(&encoding::urlsafe_decode(n)?, &encoding::urlsafe_decode(e)?)?;
    Some((kid.to_string(), key))
}

/// `Google::SignIn.allowed_domains` for the value of `GOOGLE_SIGN_IN_DOMAINS`: comma-separated,
/// stripped, downcased, valid hostnames only, deduplicated in order.
pub fn allowed_domains(env: Option<&str>) -> Vec<String> {
    let mut domains: Vec<String> = Vec::new();
    for domain in ruby::split(env.unwrap_or(""), ',') {
        let domain = ruby::strip(domain).to_lowercase();
        if valid_domain(&domain) && !domains.contains(&domain) {
            domains.push(domain);
        }
    }
    domains
}

/// `valid_domain?`: `\A[a-z0-9](?:[a-z0-9-]*[a-z0-9])?(?:\.[a-z0-9](?:[a-z0-9-]*[a-z0-9])?)+\z`,
/// two or more dot-separated labels of letters, digits and inner hyphens.
fn valid_domain(domain: &str) -> bool {
    let labels: Vec<&str> = domain.split('.').collect();
    labels.len() >= 2
        && labels.iter().all(|label| {
            let bytes = label.as_bytes();
            !bytes.is_empty()
                && bytes.iter().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'-')
                && bytes[0] != b'-'
                && bytes[bytes.len() - 1] != b'-'
        })
}

/// The app's Google configuration.
#[derive(Debug, Clone, Copy)]
pub struct Config<'a> {
    /// `Google::Client.client_id`: `ENV["GOOGLE_CLIENT_ID"].presence`.
    pub client_id: Option<&'a str>,
    /// [`allowed_domains`].
    pub allowed_domains: &'a [String],
}

/// `IdTokenVerifier.verify!(id_token, nonce:, max_auth_age:)` at `now` (unix seconds). `key_for`
/// is `KeyStore.public_key_for(kid)`: it returns the key, `Rejected(UnknownKey)`, or
/// `Unavailable`. Returns the verified claims.
pub fn verify_id_token(
    id_token: &str,
    nonce: &str,
    max_auth_age: Option<i64>,
    config: &Config<'_>,
    key_for: impl FnOnce(&str) -> Result<RsaPublicKey, IdTokenError>,
    now: i64,
) -> Result<Map<String, Value>, IdTokenError> {
    if ruby::str_blank(id_token) || ruby::str_blank(nonce) {
        return Err(Rejection::BadToken.into());
    }

    let (header, _) = decode_unverified(id_token).map_err(|_| Rejection::BadToken)?;
    let header = header.as_object().cloned().unwrap_or_default();
    if header.get("alg") != Some(&Value::from("RS256")) || ruby::blank(header.get("kid")) {
        return Err(Rejection::BadToken.into());
    }

    let key = key_for(&ruby_to_s(header.get("kid")))?;
    let decoded = decode(id_token, Key::Rs256(&key), &Validation::default(), now).map_err(|_| Rejection::BadToken)?;
    let Value::Object(payload) = decoded.payload else { return Err(Rejection::BadToken.into()) };

    verify_claims(&payload, nonce, max_auth_age, config, now)?;
    Ok(payload)
}

fn verify_claims(payload: &Map<String, Value>, nonce: &str, max_auth_age: Option<i64>, config: &Config<'_>, now: i64) -> Result<(), Rejection> {
    if !matches!(payload.get("iss"), Some(Value::String(iss)) if ISSUERS.contains(&iss.as_str())) {
        return Err(Rejection::BadToken);
    }
    verify_audience(payload, config.client_id.unwrap_or(""))?;
    if ruby::numeric_gt(payload.get("exp"), now - CLOCK_SKEW) != Some(true) {
        return Err(Rejection::Expired);
    }
    if let Some(max_auth_age) = max_auth_age
        && ruby::numeric_gt(payload.get("auth_time"), now - max_auth_age - CLOCK_SKEW) != Some(true)
    {
        return Err(Rejection::StaleAuth);
    }
    if ruby::blank(payload.get("sub")) {
        return Err(Rejection::BadToken);
    }
    if ruby::blank(payload.get("email")) {
        return Err(Rejection::MissingEmail);
    }
    if payload.get("email_verified") != Some(&Value::Bool(true)) {
        return Err(Rejection::UnverifiedEmail);
    }
    if !matches!(payload.get("nonce"), Some(Value::String(actual)) if constant_time_eq(actual.as_bytes(), nonce.as_bytes())) {
        return Err(Rejection::BadNonce);
    }
    verify_domain(payload, config.allowed_domains)
}

/// `verify_audience!`: `Array(aud).flatten.compact.map(&:to_s)` must include the client id, and
/// when there are several audiences, or `azp` is present at all, `azp.to_s` must be the client id.
fn verify_audience(payload: &Map<String, Value>, client_id: &str) -> Result<(), Rejection> {
    let mut audiences = Vec::new();
    flatten_audiences(payload.get("aud"), &mut audiences);
    if !audiences.iter().any(|audience| audience == client_id) {
        return Err(Rejection::BadAudience);
    }
    if (audiences.len() > 1 || !ruby::blank(payload.get("azp"))) && ruby_to_s(payload.get("azp")) != client_id {
        return Err(Rejection::BadAudience);
    }
    Ok(())
}

/// `Array(value).flatten.compact.map(&:to_s)`. `Array(hash)` is its `[key, value]` pairs.
fn flatten_audiences(value: Option<&Value>, out: &mut Vec<String>) {
    match value {
        None | Some(Value::Null) => {}
        Some(Value::Array(items)) => items.iter().for_each(|item| flatten_audiences(Some(item), out)),
        Some(Value::Object(pairs)) => pairs.iter().for_each(|(key, value)| {
            out.push(key.clone());
            flatten_audiences(Some(value), out);
        }),
        Some(scalar) => out.push(ruby_to_s(Some(scalar))),
    }
}

/// `verify_domain!`: the stripped, downcased `hd` claim is an allowed domain, and so is the
/// email's (`email.to_s.strip.downcase.split("@").last`).
fn verify_domain(payload: &Map<String, Value>, allowed: &[String]) -> Result<(), Rejection> {
    let domain = ruby::strip(&ruby_to_s(payload.get("hd"))).to_lowercase();
    let email = ruby::strip(&ruby_to_s(payload.get("email"))).to_lowercase();
    let email_domain = ruby::split(&email, '@').last().copied().unwrap_or("").to_string();
    if !domain.is_empty() && allowed.contains(&domain) && allowed.contains(&email_domain) {
        Ok(())
    } else {
        Err(Rejection::WrongDomain)
    }
}
