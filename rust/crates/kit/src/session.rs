//! The Rails cookie-store session (`ActionDispatch::Session::CookieStore` over
//! `Rack::Session::Abstract::PersistedSecure`), as configured in
//! `reference/config/initializers/session_store.rb`: key `_campfire_session`, encrypted, and
//! `expire_after: 20.years`.
//!
//! Loading is lazy. Unlike Rails, the cookie is only written when the session changed during the
//! request, and deleted when that left it empty. Rails' `commit_session` rewrites it on every
//! request that loads or carries one (`expire_after` forces the update), which made a session
//! cookie, re-encrypted, part of nearly every response. Without CSRF tokens the session holds only
//! the flash and a return-to URL, so an unchanged session needs no cookie traffic and an empty one
//! no cookie. Cookies Rails wrote are read the same way.

use serde_json::{Map, Value};

use crate::clock;
use crate::cookies::{Cookie, CookieJar};

pub const SESSION_KEY: &str = "_campfire_session";
pub const EXPIRE_AFTER_YEARS: i64 = 20;

#[derive(Debug, Clone)]
pub struct SessionConfig {
    pub key: String,
    pub expire_after_years: Option<i64>,
    pub httponly: bool,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self { key: SESSION_KEY.into(), expire_after_years: Some(EXPIRE_AFTER_YEARS), httponly: true }
    }
}

#[derive(Debug)]
pub struct Session {
    config: SessionConfig,
    loaded: bool,
    /// Whether the data changed during this request, and so the cookie needs writing.
    changed: bool,
    data: Map<String, Value>,
    /// The cookie's decoded contents, read once (`action_dispatch.request.unsigned_session_cookie`).
    cookie_data: Option<Map<String, Value>>,
}

impl Session {
    pub fn new(config: SessionConfig) -> Self {
        Self { config, loaded: false, changed: false, data: Map::new(), cookie_data: None }
    }

    pub fn is_loaded(&self) -> bool {
        self.loaded
    }

    /// Load from the cookie if not yet loaded (`load_for_read!`/`load_for_write!`).
    pub fn load(&mut self, jar: &CookieJar) -> &mut Self {
        if !self.loaded {
            let mut data = self.cookie_data(jar).clone();
            if data.get("session_id").is_none_or(Value::is_null) {
                data.insert("session_id".into(), Value::String(generate_sid()));
            }
            self.data = data;
            self.loaded = true;
        }
        self
    }

    fn cookie_data(&mut self, jar: &CookieJar) -> &Map<String, Value> {
        let key = &self.config.key;
        self.cookie_data.get_or_insert_with(|| match jar.encrypted(key) {
            Some(Value::Object(map)) => map,
            _ => Map::new(),
        })
    }

    fn assert_loaded(&self) {
        debug_assert!(self.loaded, "session used before load(); go through Ctx::session()");
    }

    pub fn id(&self) -> Option<&str> {
        self.assert_loaded();
        self.data.get("session_id").and_then(Value::as_str)
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.assert_loaded();
        self.data.get(key).filter(|v| !v.is_null())
    }

    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(Value::as_str)
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.assert_loaded();
        self.data.contains_key(key)
    }

    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<Value>) {
        self.assert_loaded();
        let value = value.into();
        let key = key.into();
        if self.data.get(&key) != Some(&value) {
            self.data.insert(key, value);
            self.changed = true;
        }
    }

    pub fn remove(&mut self, key: &str) -> Option<Value> {
        self.assert_loaded();
        let removed = self.data.remove(key);
        self.changed |= removed.is_some();
        removed
    }

    /// `reset_session`: drop everything and start a new session id.
    pub fn reset(&mut self) {
        self.data = Map::new();
        self.data.insert("session_id".into(), Value::String(generate_sid()));
        self.cookie_data = Some(self.data.clone());
        self.loaded = true;
        self.changed = true;
    }

    /// Writes the cookie into `jar` if the session changed, or deletes it if that left nothing but
    /// the session id.
    pub fn commit(&mut self, jar: &mut CookieJar, now: jiff::Timestamp) -> crate::Result<()> {
        if !self.changed {
            return Ok(());
        }
        let data: Map<String, Value> = self.data.iter().filter(|(_, v)| !v.is_null()).map(|(k, v)| (k.clone(), v.clone())).collect();
        if data.keys().all(|key| key == "session_id") {
            jar.delete(&self.config.key);
            return Ok(());
        }
        let mut cookie = Cookie::new("");
        cookie.httponly = self.config.httponly;
        if let Some(years) = self.config.expire_after_years {
            cookie = cookie.expires(clock::years_from(now, years));
        }
        let key = self.config.key.clone();
        jar.set_encrypted(&key, &Value::Object(data), cookie)
    }
}

/// `ActionDispatch::Session::Compatibility#generate_sid`: `SecureRandom.hex(16)`.
pub fn generate_sid() -> String {
    hex::encode(rand::random::<[u8; 16]>())
}

/// `ActionDispatch::Flash::FlashHash`, stored in the session under `"flash"` as
/// `{ "discard" => [], "flashes" => { ... } }`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Flash {
    flashes: Vec<(String, Value)>,
    discard: Vec<String>,
}

impl Flash {
    /// `FlashHash.from_session_value`: everything loaded is marked for discard at the end of this
    /// request, minus what the previous request had already discarded.
    pub fn from_session_value(value: Option<&Value>) -> Self {
        let Some(Value::Object(stored)) = value else { return Self::default() };
        let discarded: Vec<&str> =
            stored.get("discard").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).collect()).unwrap_or_default();
        let flashes: Vec<(String, Value)> = match stored.get("flashes") {
            Some(Value::Object(flashes)) => {
                flashes.iter().filter(|(k, _)| !discarded.contains(&k.as_str())).map(|(k, v)| (k.clone(), v.clone())).collect()
            }
            _ => vec![],
        };
        let discard = flashes.iter().map(|(k, _)| k.clone()).collect();
        Self { flashes, discard }
    }

    /// `FlashHash#to_session_value`: `None` when nothing survives.
    pub fn to_session_value(&self) -> Option<Value> {
        let keep: Map<String, Value> =
            self.flashes.iter().filter(|(k, _)| !self.discard.contains(k)).map(|(k, v)| (k.clone(), v.clone())).collect();
        if keep.is_empty() { None } else { Some(serde_json::json!({ "discard": [], "flashes": keep })) }
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.flashes.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(Value::as_str)
    }

    /// `flash[key] = value`: shown on the next request.
    pub fn set(&mut self, key: &str, value: impl Into<Value>) {
        self.discard.retain(|k| k != key);
        match self.flashes.iter_mut().find(|(k, _)| k == key) {
            Some(slot) => slot.1 = value.into(),
            None => self.flashes.push((key.to_string(), value.into())),
        }
    }

    /// `flash.now[key] = value`: shown on this request only.
    pub fn now(&mut self, key: &str, value: impl Into<Value>) {
        self.set(key, value);
        self.discard(Some(key));
    }

    pub fn keep(&mut self, key: Option<&str>) {
        match key {
            Some(key) => self.discard.retain(|k| k != key),
            None => self.discard.clear(),
        }
    }

    pub fn discard(&mut self, key: Option<&str>) {
        let keys: Vec<String> = match key {
            Some(key) => vec![key.to_string()],
            None => self.flashes.iter().map(|(k, _)| k.clone()).collect(),
        };
        for key in keys {
            if !self.discard.contains(&key) {
                self.discard.push(key);
            }
        }
    }

    pub fn delete(&mut self, key: &str) {
        self.discard.retain(|k| k != key);
        self.flashes.retain(|(k, _)| k != key);
    }

    pub fn is_empty(&self) -> bool {
        self.flashes.is_empty()
    }

    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.flashes.iter().map(|(k, _)| k.as_str())
    }

    pub fn notice(&self) -> Option<&str> {
        self.get_str("notice")
    }

    pub fn alert(&self) -> Option<&str> {
        self.get_str("alert")
    }

    pub fn set_notice(&mut self, message: impl Into<Value>) {
        self.set("notice", message);
    }

    pub fn set_alert(&mut self, message: impl Into<Value>) {
        self.set("alert", message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn flash_round_trip_discards_after_one_request() {
        let mut flash = Flash::default();
        flash.set_notice("✓");
        let stored = flash.to_session_value().unwrap();
        assert_eq!(stored, json!({"discard": [], "flashes": {"notice": "✓"}}));

        let next = Flash::from_session_value(Some(&stored));
        assert_eq!(next.notice(), Some("✓"));
        assert_eq!(next.to_session_value(), None);
    }

    #[test]
    fn flash_now_is_not_persisted() {
        let mut flash = Flash::default();
        flash.now("alert", "Too many requests or unauthorized.");
        assert_eq!(flash.alert(), Some("Too many requests or unauthorized."));
        assert_eq!(flash.to_session_value(), None);
    }

    #[test]
    fn flash_keep_and_legacy_discard_lists() {
        let stored = json!({"discard": ["alert"], "flashes": {"notice": "hi", "alert": "gone"}});
        let mut flash = Flash::from_session_value(Some(&stored));
        assert_eq!(flash.alert(), None);
        flash.keep(Some("notice"));
        assert_eq!(flash.to_session_value(), Some(json!({"discard": [], "flashes": {"notice": "hi"}})));
    }

    #[test]
    fn sids_are_32_hex_chars() {
        let sid = generate_sid();
        assert_eq!(sid.len(), 32);
        assert!(sid.bytes().all(|b| b.is_ascii_hexdigit()));
    }
}
