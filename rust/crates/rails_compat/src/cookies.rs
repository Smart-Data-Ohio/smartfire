//! `cookies.signed[...]` / `cookies.encrypted[...]` values (`action_dispatch/middleware/cookies.rb`),
//! plus Rack's escaping of cookie values on the wire. Attributes (path, expires, HttpOnly,
//! SameSite) are the HTTP layer's job.
//!
//! The functions here work on the *raw* jar value. On the wire Rack escapes it with
//! `URI.encode_www_form_component` ([`escape`]) and unescapes incoming cookies ([`unescape`]).
//!
//! How the jars work, per Rails main:
//! - the value is first dumped with `cookies_serializer` (`:json` here, so `ActiveSupport::JSON`),
//! - then signed (HMAC-**SHA1**, key `generate_key("signed cookie")`) or encrypted (aes-256-gcm,
//!   key `generate_key("authenticated encrypted cookie", 32)`) with the legacy metadata envelope
//!   carrying `pur: "cookie.<name>"` and `exp` (ISO 8601 with milliseconds, or `null`),
//! - reading tries purpose `cookie.<name>` first, then *no purpose*, so a value signed without
//!   metadata (pre-Rails 5.2) is accepted under any cookie name.
use jiff::{SignedDuration, Timestamp, ToSpan, tz::TimeZone};
use serde_json::Value;

use crate::message_verifier::{Digest, Encoding};
use crate::{MessageEncryptor, MessageVerifier, Secrets, json};
use crate::metadata::Serializer;

pub const SIGNED_COOKIE_SALT: &str = "signed cookie";
pub const AUTHENTICATED_ENCRYPTED_COOKIE_SALT: &str = "authenticated encrypted cookie";

/// `cookies.permanent`: expires 20 years from now (calendar years, like `20.years.from_now`).
pub fn permanent_expires_at(now: Timestamp) -> Timestamp {
    now.to_zoned(TimeZone::UTC).checked_add(20.years()).expect("20 years from now is in range").timestamp()
}

/// The raw value for `cookies.signed[name] = { value:, expires: expires_at }`.
/// `cookies.signed.permanent[...]` is `expires_at: Some(permanent_expires_at(now))`.
pub fn sign(secrets: &Secrets, name: &str, value: &str, expires_at: Option<Timestamp>) -> String {
    let dumped = json::encode(&Value::String(value.to_string()));
    signed_cookie_verifier(secrets).generate(&Value::String(dumped), Some(&purpose(name)), expires_at)
}

/// Reads `cookies.signed[name]`; `None` wherever Rails returns nil. A value that is valid JSON
/// but not a string (Rails would return it) is also `None`.
pub fn verify_signed(secrets: &Secrets, name: &str, raw: &str, now: Timestamp) -> Option<String> {
    match verify_signed_value(secrets, name, raw, now)? {
        Value::String(s) => Some(s),
        _ => None,
    }
}

/// Reads `cookies.signed[name]` as whatever JSON value it holds.
pub fn verify_signed_value(secrets: &Secrets, name: &str, raw: &str, now: Timestamp) -> Option<Value> {
    let verifier = signed_cookie_verifier(secrets);
    let dumped = verifier
        .verify(raw, Some(&purpose(name)), now)
        .or_else(|_| verifier.verify(raw, None, now))
        .ok()?;
    load(dumped)
}

/// The raw value for `cookies.encrypted[name] = { value:, expires: expires_at }`.
/// The session store writes `_campfire_session` this way with a 20-year `expire_after`.
pub fn encrypt(secrets: &Secrets, name: &str, value: &Value, expires_at: Option<Timestamp>) -> String {
    let dumped = json::encode(value);
    encrypted_cookie_encryptor(secrets).encrypt_and_sign(&Value::String(dumped), Some(&purpose(name)), expires_at)
}

/// Reads `cookies.encrypted[name]`; `None` wherever Rails returns nil.
pub fn decrypt(secrets: &Secrets, name: &str, raw: &str, now: Timestamp) -> Option<Value> {
    let encryptor = encrypted_cookie_encryptor(secrets);
    let dumped = encryptor
        .decrypt_and_verify(raw, Some(&purpose(name)), now)
        .or_else(|_| encryptor.decrypt_and_verify(raw, None, now))
        .ok()?;
    load(dumped)
}

fn purpose(name: &str) -> String {
    format!("cookie.{name}")
}

/// `SerializerWithFallback[:json].load`: Marshal payloads aren't allowed for cookies.
fn load(dumped: Value) -> Option<Value> {
    let Value::String(dumped) = dumped else { return None };
    Serializer::JsonWithFallback { allow_marshal: false }.load(dumped.as_bytes()).ok()
}

/// Smartfire's own signed cookies, set by `app/controllers/concerns/authentication.rb`.
pub const DEVICE_ID: &str = "device_id";
/// `Authentication::TWO_FACTOR_REMEMBER_COOKIE`.
pub const TWO_FACTOR_REMEMBER: &str = "two_factor_remember";
/// `TwoFactorRememberedDevice::REMEMBER_FOR`.
pub const TWO_FACTOR_REMEMBER_FOR: SignedDuration = SignedDuration::from_hours(30 * 24);

/// A cookie as the jar writes it: the raw (unescaped) jar value and the attributes Rails sets.
/// Rails sends no `domain`, and `path` is `/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedCookie {
    pub name: &'static str,
    pub value: String,
    pub expires: Timestamp,
    /// Rails only writes a `secure` cookie on an HTTPS request (`CookieJar#write_cookie?`); on
    /// plain HTTP it silently drops it.
    pub secure: bool,
    pub http_only: bool,
    /// `same_site: :lax` for both.
    pub same_site: &'static str,
}

impl SignedCookie {
    /// The `Set-Cookie` header Rack writes: `name=<escaped>; path=/; expires=<httpdate>;
    /// [secure; ]httponly; samesite=lax`.
    pub fn set_cookie_header(&self) -> String {
        let mut header = format!("{}={}; path=/; expires={}", self.name, escape(&self.value), self.expires.strftime("%a, %d %b %Y %H:%M:%S GMT"));
        if self.secure {
            header.push_str("; secure");
        }
        if self.http_only {
            header.push_str("; httponly");
        }
        header.push_str("; samesite=");
        header.push_str(self.same_site);
        header
    }
}

/// `ensure_device_cookie`: `cookies.signed.permanent[:device_id] = { value: device_id,
/// httponly: true, same_site: :lax }`, the device id being `SecureRandom.hex(16)`. It is only
/// written when the request has no valid one ([`read_device_id`]).
pub fn device_id_cookie(secrets: &Secrets, device_id: &str, now: Timestamp) -> SignedCookie {
    let expires = permanent_expires_at(now);
    SignedCookie { name: DEVICE_ID, value: sign(secrets, DEVICE_ID, device_id, Some(expires)), expires, secure: false, http_only: true, same_site: "lax" }
}

/// `cookies.signed[:device_id]`.
pub fn read_device_id(secrets: &Secrets, raw: &str, now: Timestamp) -> Option<String> {
    verify_signed(secrets, DEVICE_ID, raw, now)
}

/// `remember_two_factor_device!`: `cookies.signed[:two_factor_remember] = { value: token,
/// expires: 30.days, httponly: true, secure: true, same_site: :lax }`. The token is
/// `SecureRandom.hex(32)`; only its SHA256 is stored (`TwoFactorRememberedDevice.digest`).
pub fn two_factor_remember_cookie(secrets: &Secrets, token: &str, now: Timestamp) -> SignedCookie {
    let expires = now + TWO_FACTOR_REMEMBER_FOR;
    SignedCookie { name: TWO_FACTOR_REMEMBER, value: sign(secrets, TWO_FACTOR_REMEMBER, token, Some(expires)), expires, secure: true, http_only: true, same_site: "lax" }
}

/// `cookies.signed[TWO_FACTOR_REMEMBER_COOKIE]`.
pub fn read_two_factor_remember(secrets: &Secrets, raw: &str, now: Timestamp) -> Option<String> {
    verify_signed(secrets, TWO_FACTOR_REMEMBER, raw, now)
}

pub fn signed_cookie_verifier(secrets: &Secrets) -> MessageVerifier {
    // `signed_cookie_digest` is unset, so the jar falls back to "SHA1".
    let secret = secrets.key_generator.generate_key(SIGNED_COOKIE_SALT, 64);
    MessageVerifier::new(secret, Digest::Sha1, Encoding::Strict, Serializer::Null)
}

pub fn encrypted_cookie_encryptor(secrets: &Secrets) -> MessageEncryptor {
    let secret = secrets.key_generator.generate_key(AUTHENTICATED_ENCRYPTED_COOKIE_SALT, 32);
    MessageEncryptor::new(&secret, Serializer::Null)
}

/// `Rack::Utils.escape` (`URI.encode_www_form_component`), which Rack applies to every cookie
/// value it writes: `*-._` and alphanumerics stay, a space becomes `+`, the rest is `%XX`.
pub fn escape(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for &byte in raw.as_bytes() {
        match byte {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'*' | b'-' | b'.' | b'_' => out.push(byte as char),
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// `Rack::Utils.parse_cookies_header`'s `unescape(value) rescue value`: `+` is a space, `%XX` is
/// decoded, and a malformed escape leaves the value untouched.
pub fn unescape(wire: &str) -> String {
    let bytes = wire.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => out.push(b' '),
            b'%' => match bytes.get(i + 1..i + 3).filter(|hex| hex.iter().all(u8::is_ascii_hexdigit)) {
                Some(hex) => {
                    out.push(u8::from_str_radix(std::str::from_utf8(hex).expect("hex digits"), 16).expect("hex digits"));
                    i += 2;
                }
                None => return wire.to_string(),
            },
            byte => out.push(byte),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_and_remember_cookies_expire_and_stay_under_their_names() {
        let secrets = Secrets::new(&"a".repeat(128));
        let now: Timestamp = "2026-01-01T12:00:00Z".parse().unwrap();
        let device = device_id_cookie(&secrets, "device", now);
        let remember = two_factor_remember_cookie(&secrets, "token", now);
        assert!(!device.secure && remember.secure && device.http_only && remember.http_only);

        assert_eq!(read_device_id(&secrets, &device.value, device.expires - SignedDuration::from_secs(1)).as_deref(), Some("device"));
        assert_eq!(read_device_id(&secrets, &device.value, device.expires + SignedDuration::from_secs(1)), None);
        assert_eq!(read_two_factor_remember(&secrets, &remember.value, now + TWO_FACTOR_REMEMBER_FOR - SignedDuration::from_secs(1)).as_deref(), Some("token"));
        assert_eq!(read_two_factor_remember(&secrets, &remember.value, now + TWO_FACTOR_REMEMBER_FOR), None);

        assert_eq!(read_two_factor_remember(&secrets, &device.value, now), None);
        assert_eq!(read_device_id(&secrets, &remember.value, now), None);
        assert_eq!(verify_signed(&secrets, "session_token", &remember.value, now), None);
        assert_eq!(read_two_factor_remember(&Secrets::new(&"b".repeat(128)), &remember.value, now), None);

        let (data, digest) = remember.value.split_once("--").unwrap();
        let tampered = format!("{data}--{}", if digest.starts_with('0') { digest.replacen('0', "1", 1) } else { format!("0{}", &digest[1..]) });
        assert_eq!(read_two_factor_remember(&secrets, &tampered, now), None);
    }
}
