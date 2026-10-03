//! The sign-in and step-up state our Rails app keeps in the cookie session, read and written the
//! way it does so a session carries over between the apps in both directions:
//!
//! - the pending second factor (`Authentication`, app/controllers/concerns/authentication.rb):
//!   `two_factor_pending_user_id`, `two_factor_pending_expires_at` (epoch seconds) and
//!   `two_factor_pending_method`;
//! - sudo mode (`SudoMode`, app/controllers/concerns/sudo_mode.rb): `sudo_verified_at` (epoch
//!   seconds) and the stashed `sudo_pending_request`;
//! - Google step-up (`TwoFactorReauthentication`): `two_factor_reauthenticated_at`;
//! - `return_to_after_authenticating`.
//!
//! These work on the session alone, with the time passed in; the database lookups that go with
//! them (the pending user must still be active) are the caller's.

// The plumbing the two-step sign-in and sudo screens build on; parts are unused until they land.
#![allow(dead_code)]

use campfire_kit::{Param, ParamMap, Session};
use jiff::{SignedDuration, Timestamp};
use regex::Regex;
use serde_json::{Map, Value, json};
use std::sync::LazyLock;

use super::ruby_to_i;

pub const TWO_FACTOR_PENDING_USER_KEY: &str = "two_factor_pending_user_id";
pub const TWO_FACTOR_PENDING_EXPIRY_KEY: &str = "two_factor_pending_expires_at";
pub const TWO_FACTOR_PENDING_METHOD_KEY: &str = "two_factor_pending_method";
/// `TWO_FACTOR_PENDING_TTL = 10.minutes`
pub const TWO_FACTOR_PENDING_TTL: SignedDuration = SignedDuration::from_mins(10);

/// `SudoMode::VERIFIED_SESSION_KEY`
pub const SUDO_VERIFIED_KEY: &str = "sudo_verified_at";
/// `SudoMode::PENDING_SESSION_KEY`
pub const SUDO_PENDING_KEY: &str = "sudo_pending_request";
/// `SudoMode::SUDO_TIMEOUT = 15.minutes`
pub const SUDO_TIMEOUT: SignedDuration = SignedDuration::from_mins(15);
/// `SudoMode::MAX_STORED_PARAMS_BYTES`
pub const MAX_STORED_PARAMS_BYTES: usize = 2048;

/// `TwoFactorReauthentication::REAUTH_SESSION_KEY`
pub const REAUTH_KEY: &str = "two_factor_reauthenticated_at";
/// `TwoFactorReauthentication::REAUTH_TTL = 10.minutes`
pub const REAUTH_TTL: SignedDuration = SignedDuration::from_mins(10);

pub const RETURN_TO_KEY: &str = "return_to_after_authenticating";

// --- Pending second factor ---------------------------------------------------------------------

/// `stash_two_factor_pending(user, method)`
pub fn stash_two_factor_pending(session: &mut Session, user_id: i64, method: &str, now: Timestamp) {
    session.insert(TWO_FACTOR_PENDING_USER_KEY, user_id);
    session.insert(TWO_FACTOR_PENDING_EXPIRY_KEY, (now + TWO_FACTOR_PENDING_TTL).as_second());
    session.insert(TWO_FACTOR_PENDING_METHOD_KEY, method);
}

/// `two_factor_pending_user` up to its `User.active.find_by(id:)`: the id to look up, or `None`
/// when nothing is pending or it expired.
pub fn two_factor_pending_user_id(session: &Session, now: Timestamp) -> Option<i64> {
    let user_id = session.get(TWO_FACTOR_PENDING_USER_KEY).filter(|value| !blank(value))?;
    let expires_at = session.get(TWO_FACTOR_PENDING_EXPIRY_KEY).filter(|value| !blank(value))?;
    if to_i(expires_at) < now.as_second() {
        return None;
    }
    // `find_by(id:)` casts the stored value like an integer column.
    match user_id {
        Value::Number(number) => number.as_i64().or_else(|| number.as_f64().map(|float| float as i64)),
        Value::String(string) => super::cast_integer(string),
        _ => None,
    }
}

/// `two_factor_pending_method`: `session[KEY].to_s.presence || "unknown"`
pub fn two_factor_pending_method(session: &Session) -> String {
    let method = session.get(TWO_FACTOR_PENDING_METHOD_KEY).map(to_s).unwrap_or_default();
    if method.trim().is_empty() { "unknown".into() } else { method }
}

/// `clear_two_factor_pending!`
pub fn clear_two_factor_pending(session: &mut Session) {
    for key in [TWO_FACTOR_PENDING_USER_KEY, TWO_FACTOR_PENDING_EXPIRY_KEY, TWO_FACTOR_PENDING_METHOD_KEY] {
        session.remove(key);
    }
}

// --- Sudo mode ---------------------------------------------------------------------------------

/// `sudo_verified?`: an Integer `sudo_verified_at` less than `SUDO_TIMEOUT` ago.
pub fn sudo_verified(session: &Session, now: Timestamp) -> bool {
    let Some(verified_at) = session.get(SUDO_VERIFIED_KEY).and_then(Value::as_i64) else { return false };
    Timestamp::from_second(verified_at).is_ok_and(|verified_at| verified_at > now - SUDO_TIMEOUT)
}

/// `mark_sudo_verified!`
pub fn mark_sudo_verified(session: &mut Session, now: Timestamp) {
    session.insert(SUDO_VERIFIED_KEY, now.as_second());
}

/// What `store_sudo_pending_request` stashes: the request to continue once confirmed.
#[derive(Debug, Clone, PartialEq)]
pub struct SudoPendingRequest {
    /// `request.request_method`
    pub method: String,
    /// `request.fullpath`
    pub path: String,
    /// [`sudo_storable_params`]
    pub params: Option<Value>,
    /// [`sudo_origin_path`]
    pub origin: String,
}

/// `store_sudo_pending_request`
pub fn store_sudo_pending_request(session: &mut Session, request: SudoPendingRequest) {
    let value = json!({ "method": request.method, "path": request.path, "params": request.params, "origin": request.origin });
    session.insert(SUDO_PENDING_KEY, value);
}

/// `SudoMode::SECRET_PARAM_PATTERN`
static SECRET_PARAM: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)passw|passwd|pwd|secret|token|api[-_]?key|_key\z|credential|authorization|cookie|session|join[-_]?code|webhook_url|access_token").unwrap()
});

/// `sudo_storable_params`: a POST, PATCH, PUT or DELETE's body params (`request_parameters`)
/// without `controller`, `action` and `authenticity_token`, if they're all scalars (no uploads),
/// none has a secret-looking key, and they encode to at most [`MAX_STORED_PARAMS_BYTES`].
pub fn sudo_storable_params(method: &str, request_parameters: &ParamMap) -> Option<Value> {
    if !matches!(method, "POST" | "PATCH" | "PUT" | "DELETE") {
        return None;
    }
    let mut filtered = request_parameters.clone();
    for key in ["controller", "action", "authenticity_token"] {
        filtered.remove(key);
    }
    let filtered = Param::Hash(filtered);
    if !scalar_params(&filtered, 0) {
        return None;
    }
    let json = filtered.to_json();
    (rails_param_json_bytesize(&filtered) <= MAX_STORED_PARAMS_BYTES).then_some(json)
}

/// `sudo_scalar_params?`: hashes with no secret keys, arrays, and scalars, at most four deep.
fn scalar_params(value: &Param, depth: usize) -> bool {
    if depth > 4 {
        return false;
    }
    match value {
        Param::Hash(map) => map.iter().all(|(key, value)| !SECRET_PARAM.is_match(key) && scalar_params(value, depth + 1)),
        Param::Array(values) => values.iter().all(|value| scalar_params(value, depth + 1)),
        Param::File(_) => false,
        _ => true,
    }
}

/// `to_json.bytesize`: `ActiveSupport::JSON` escapes `<`, `>` and `&` as `\u00XX` (six bytes).
fn active_support_json_bytesize(value: &Value) -> usize {
    let json = serde_json::to_string(value).expect("a Value serializes");
    json.len() + json.bytes().filter(|byte| matches!(byte, b'<' | b'>' | b'&')).count() * 5
}

/// Scoped exact Integers are stored as digit strings because serde's ordinary
/// Value cannot hold Bignum. Rails counts their raw JSON digits at this boundary.
fn rails_param_json_bytesize(value: &Param) -> usize {
    match value {
        Param::BigInteger(digits) => digits.len(),
        Param::Number(number) => rails_compat::numbers::number_to_s(number).len(),
        Param::Array(values) => 2 + values.len().saturating_sub(1) + values.iter().map(rails_param_json_bytesize).sum::<usize>(),
        Param::Hash(values) => 2 + values.len().saturating_sub(1) + values.iter().map(|(key,value)| active_support_json_bytesize(&Value::String(key.clone())) + 1 + rails_param_json_bytesize(value)).sum::<usize>(),
        _ => active_support_json_bytesize(&value.to_json()),
    }
}

/// `sudo_origin_path`: the referrer's path when it's on this host, else the root.
pub fn sudo_origin_path(referer: Option<&str>, host: &str, root_path: &str) -> String {
    let Some(referer) = referer.filter(|referer| !referer.trim().is_empty()) else { return root_path.into() };
    match crate::security::ruby_uri::parse(referer) {
        Some(uri) if uri.host.as_deref() == Some(host) => {
            if uri.path.is_empty() { root_path.into() } else { uri.path }
        }
        _ => root_path.into(),
    }
}

/// Where `continue_after_sudo!` goes once confirmed.
#[derive(Debug, Clone, PartialEq)]
pub enum SudoContinuation {
    /// A stashed GET: `redirect_to path`.
    Redirect(String),
    /// A stashed non-GET with replayable params: `render "sudos/continue"`.
    Replay { method: String, path: String, params: Value },
    /// Nothing safe to continue: `redirect_to origin.presence || root_path`.
    Origin(String),
}

/// `continue_after_sudo!` without the response: takes the stashed request out of the session.
pub fn continue_after_sudo(session: &mut Session, root_path: &str) -> SudoContinuation {
    let pending = match session.remove(SUDO_PENDING_KEY) {
        Some(Value::Object(pending)) => pending,
        _ => Map::new(),
    };
    let path = pending.get("path").map(to_s).unwrap_or_default();
    let path = (path.starts_with('/') && !path.starts_with("//")).then_some(path);
    let params = pending.get("params").filter(|params| !matches!(params, Value::Null | Value::Bool(false)));
    match (path, params) {
        (Some(path), _) if pending.get("method").map(to_s).unwrap_or_default().to_uppercase() == "GET" => {
            SudoContinuation::Redirect(path)
        }
        (Some(path), Some(params)) => SudoContinuation::Replay {
            method: pending.get("method").map(to_s).unwrap_or_default().to_lowercase(),
            path,
            params: params.clone(),
        },
        _ => {
            let origin = pending.get("origin").map(to_s).filter(|origin| !origin.trim().is_empty());
            SudoContinuation::Origin(origin.unwrap_or_else(|| root_path.into()))
        }
    }
}

// --- Google step-up ----------------------------------------------------------------------------

/// `session[REAUTH_SESSION_KEY] = Time.current.to_i` (Sessions::GoogleController#callback).
pub fn mark_reauthenticated(session: &mut Session, now: Timestamp) {
    session.insert(REAUTH_KEY, now.as_second());
}

/// `consume_google_reauthentication!`: single use, and good for [`REAUTH_TTL`].
pub fn consume_google_reauthentication(session: &mut Session, now: Timestamp) -> bool {
    let Some(confirmed_at) = session.remove(REAUTH_KEY) else { return false };
    !blank(&confirmed_at) && to_i(&confirmed_at) > (now - REAUTH_TTL).as_second()
}

// --- Starting a session ------------------------------------------------------------------------

/// What `start_new_session_for` drops first: a different member signing in on this browser must
/// not inherit the previous one's confirmations.
pub fn clear_confirmations(session: &mut Session) {
    for key in [SUDO_VERIFIED_KEY, SUDO_PENDING_KEY, REAUTH_KEY] {
        session.remove(key);
    }
}

// --- Ruby value semantics ------------------------------------------------------------------------

/// `blank?`
fn blank(value: &Value) -> bool {
    match value {
        Value::Null | Value::Bool(false) => true,
        Value::String(string) => string.chars().all(char::is_whitespace),
        Value::Array(values) => values.is_empty(),
        Value::Object(map) => map.is_empty(),
        _ => false,
    }
}

/// `to_i` for the values a JSON session holds (`nil.to_i` is 0; a boolean has none, and counts
/// as 0 here where Rails would raise).
fn to_i(value: &Value) -> i64 {
    match value {
        Value::Number(number) => number.as_i64().unwrap_or_else(|| number.as_f64().map_or(0, |float| float as i64)),
        Value::String(string) => ruby_to_i(string),
        _ => 0,
    }
}

/// `to_s`
fn to_s(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(string) => string.clone(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use campfire_kit::session::SessionConfig;
    use campfire_kit::{CookieJar, Crypto, RailsCrypto, SharedClock};
    use std::sync::Arc;

    fn vectors() -> Value {
        serde_json::from_str(include_str!("../../../../vectors/kit_security.json")).unwrap()
    }

    fn at(time: &str) -> Timestamp {
        time.parse().unwrap()
    }

    fn crypto(vectors: &Value) -> Arc<RailsCrypto> {
        Arc::new(RailsCrypto::new(Arc::new(rails_compat::Secrets::new(vectors["secret_key_base"].as_str().unwrap()))))
    }

    fn jar(vectors: &Value, raw: Option<&str>) -> CookieJar {
        let clock: SharedClock = Arc::new(campfire_kit::clock::FrozenClock::new(at(vectors["now"].as_str().unwrap())));
        let header = raw.map(|raw| format!("_campfire_session={}", campfire_kit::cookies::escape(raw)));
        CookieJar::from_headers(header.as_deref(), crypto(vectors), clock)
    }

    /// The session Rails wrote, as the kit loads it.
    fn rails_session(vectors: &Value) -> Session {
        let jar = jar(vectors, vectors["session_keys"]["raw"].as_str());
        let mut session = Session::new(SessionConfig::default());
        session.load(&jar);
        session
    }

    #[test]
    fn reads_the_keys_rails_wrote_as_rails_does() {
        let vectors = vectors();
        let keys = &vectors["session_keys"];
        for check in keys["checks"].as_array().unwrap() {
            let now = at(check["now"].as_str().unwrap());
            let mut session = rails_session(&vectors);
            assert_eq!(two_factor_pending_user_id(&session, now), check["two_factor_pending_user_id"].as_i64(), "{check}");
            assert_eq!(two_factor_pending_method(&session), "password");
            assert_eq!(sudo_verified(&session, now), check["sudo_verified"].as_bool().unwrap(), "{check}");
            assert_eq!(consume_google_reauthentication(&mut session, now), check["reauthenticated"].as_bool().unwrap(), "{check}");
            assert!(!consume_google_reauthentication(&mut session, now), "single use");
        }
        let mut session = rails_session(&vectors);
        assert_eq!(session.get_str(RETURN_TO_KEY), keys["data"][RETURN_TO_KEY].as_str());
        assert_eq!(
            continue_after_sudo(&mut session, "/"),
            SudoContinuation::Replay { method: "patch".into(), path: "/account?x=1".into(), params: json!({ "account": { "name": "New" } }) }
        );
        assert_eq!(continue_after_sudo(&mut session, "/"), SudoContinuation::Origin("/".into()), "taken");
        assert_eq!(keys["two_factor_pending_ttl_seconds"], TWO_FACTOR_PENDING_TTL.as_secs());
        assert_eq!(keys["sudo_timeout_seconds"], SUDO_TIMEOUT.as_secs());
        assert_eq!(keys["reauth_ttl_seconds"], REAUTH_TTL.as_secs());
    }

    /// The same state written here decrypts, with Rails' keys, to exactly what Rails wrote.
    /// (`reference-tools/kit/security_verify_rust.rb` then has Rails read this cookie too.)
    #[test]
    fn writes_the_keys_as_rails_does() {
        let vectors = vectors();
        let data = &vectors["session_keys"]["data"];
        let now = at(vectors["now"].as_str().unwrap());
        let mut jar = jar(&vectors, None);
        let mut session = Session::new(SessionConfig::default());
        session.load(&jar);
        session.insert("session_id", data["session_id"].clone());
        session.insert("_csrf_token", data["_csrf_token"].clone());
        stash_two_factor_pending(&mut session, data[TWO_FACTOR_PENDING_USER_KEY].as_i64().unwrap(), "password", now);
        mark_sudo_verified(&mut session, now);
        let mut params = ParamMap::new();
        params.insert("account", Param::from_json(json!({ "name": "New" })));
        params.insert("authenticity_token", Param::Str("dropped".into()));
        store_sudo_pending_request(
            &mut session,
            SudoPendingRequest {
                method: "PATCH".into(),
                path: "/account?x=1".into(),
                params: sudo_storable_params("PATCH", &params),
                origin: sudo_origin_path(Some("http://campfire.test/account/edit"), "campfire.test", "/"),
            },
        );
        mark_reauthenticated(&mut session, now);
        session.insert(RETURN_TO_KEY, data[RETURN_TO_KEY].clone());
        session.commit(&mut jar, now).unwrap();

        let header = jar.set_cookie_headers(false, "campfire.test").into_iter().next().unwrap();
        let raw = rails_compat::cookies::unescape(header.strip_prefix("_campfire_session=").unwrap().split(';').next().unwrap());
        assert_eq!(&crypto(&vectors).decrypt_cookie("_campfire_session", &raw, now).unwrap(), data);

        let output = json!({ "now": vectors["now"], "raw": raw, "data": data, "checks": vectors["session_keys"]["checks"] });
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/campfire_session_keys_rust_output.json");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, serde_json::to_string_pretty(&output).unwrap()).unwrap();
    }

    #[test]
    fn a_new_session_drops_the_previous_members_confirmations() {
        let vectors = vectors();
        let mut session = rails_session(&vectors);
        clear_confirmations(&mut session);
        for key in [SUDO_VERIFIED_KEY, SUDO_PENDING_KEY, REAUTH_KEY] {
            assert!(!session.contains_key(key), "{key}");
        }
        assert!(session.contains_key(TWO_FACTOR_PENDING_USER_KEY));
        assert!(session.contains_key(RETURN_TO_KEY));
    }

    #[test]
    fn only_scalar_params_without_secrets_are_stored() {
        let params = |value: Value| match Param::from_json(value) {
            Param::Hash(map) => map,
            _ => unreachable!(),
        };
        let plain = params(json!({ "account": { "name": "N", "tags": ["a", 1, null, true] }, "controller": "x", "action": "y" }));
        assert_eq!(sudo_storable_params("PATCH", &plain), Some(json!({ "account": { "name": "N", "tags": ["a", 1, true] } })));
        assert_eq!(sudo_storable_params("GET", &plain), None);
        assert_eq!(sudo_storable_params("POST", &params(json!({ "user": { "password": "x" } }))), None);
        assert_eq!(sudo_storable_params("POST", &params(json!({ "bot_key": "x" }))), None);
        assert_eq!(sudo_storable_params("POST", &params(json!({ "a": { "b": { "c": { "d": { "e": 1 } } } } }))), None, "five deep");
        assert!(sudo_storable_params("POST", &params(json!({ "a": { "b": { "c": { "d": 1 } } } }))).is_some());
        // 2048 bytes as ActiveSupport encodes it: each `<` is six.
        let fits = "x".repeat(2048 - r#"{"a":""}"#.len());
        assert!(sudo_storable_params("POST", &params(json!({ "a": fits }))).is_some());
        let escaped = format!("<{}", "x".repeat(2048 - r#"{"a":""}"#.len() - 6));
        assert!(sudo_storable_params("POST", &params(json!({ "a": escaped }))).is_some());
        let over = format!("<{}", "x".repeat(2048 - r#"{"a":""}"#.len() - 5));
        assert_eq!(sudo_storable_params("POST", &params(json!({ "a": over }))), None);
    }

    #[test]
    fn continues_like_the_reference() {
        let vectors = vectors();
        let mut session = rails_session(&vectors);
        let mut stash = |value: Value| {
            session.insert(SUDO_PENDING_KEY, value);
            continue_after_sudo(&mut session, "/")
        };
        assert_eq!(stash(json!({ "method": "get", "path": "/account/edit", "params": null })), SudoContinuation::Redirect("/account/edit".into()));
        assert_eq!(stash(json!({ "method": "GET", "path": "//evil.test/x", "origin": "/o" })), SudoContinuation::Origin("/o".into()));
        assert_eq!(stash(json!({ "method": "POST", "path": "/x", "params": null, "origin": "" })), SudoContinuation::Origin("/".into()));
        assert_eq!(stash(json!("not a hash")), SudoContinuation::Origin("/".into()));
        assert_eq!(
            stash(json!({ "method": "DELETE", "path": "/x", "params": {} })),
            SudoContinuation::Replay { method: "delete".into(), path: "/x".into(), params: json!({}) }
        );
    }

    #[test]
    fn origin_is_the_same_host_referrer_path() {
        assert_eq!(sudo_origin_path(None, "campfire.test", "/"), "/");
        assert_eq!(sudo_origin_path(Some(" "), "campfire.test", "/"), "/");
        assert_eq!(sudo_origin_path(Some("http://campfire.test/rooms/1?x=1"), "campfire.test", "/"), "/rooms/1");
        assert_eq!(sudo_origin_path(Some("https://campfire.test"), "campfire.test", "/"), "/");
        assert_eq!(sudo_origin_path(Some("https://evil.test/rooms/1"), "campfire.test", "/"), "/");
        assert_eq!(sudo_origin_path(Some("/rooms/1"), "campfire.test", "/"), "/");
        assert_eq!(sudo_origin_path(Some("http://campfire.test/a b"), "campfire.test", "/"), "/", "InvalidURIError");
    }

    #[test]
    fn pending_second_factor_values_are_read_like_ruby() {
        let vectors = vectors();
        let now = at(vectors["now"].as_str().unwrap());
        let mut session = rails_session(&vectors);
        session.insert(TWO_FACTOR_PENDING_USER_KEY, "7abc");
        session.insert(TWO_FACTOR_PENDING_EXPIRY_KEY, now.as_second().to_string());
        assert_eq!(two_factor_pending_user_id(&session, now), Some(7), "expiring this second still counts");
        session.insert(TWO_FACTOR_PENDING_EXPIRY_KEY, " ");
        assert_eq!(two_factor_pending_user_id(&session, now), None);
        session.insert(TWO_FACTOR_PENDING_METHOD_KEY, " ");
        assert_eq!(two_factor_pending_method(&session), "unknown");
        clear_two_factor_pending(&mut session);
        assert_eq!(two_factor_pending_user_id(&session, now), None);
        assert!(!session.contains_key(TWO_FACTOR_PENDING_METHOD_KEY));
    }
}
