//! Rails' token forgery protection as our app runs it (actionpack
//! `action_controller/metal/request_forgery_protection.rb`, `load_defaults 8.0`):
//! `protect_from_forgery with: :exception`, the raw token in the session under `_csrf_token`,
//! masked tokens in forms and the `csrf-token` meta tag, per-form tokens
//! (`per_form_csrf_tokens`) and the `Origin` check (`forgery_protection_origin_check`).
//!
//! A token is accepted in the `authenticity_token` param or the `X-CSRF-Token` header. It may be:
//! - the raw session token, base64 (32 bytes: tokens issued before masking);
//! - a masked token (64 bytes: a one-time pad, then the pad XOR a 32-byte token), where the
//!   unmasked token is the global HMAC, the raw session token, or the per-form HMAC for this
//!   request's path and method.
//!
//! Both apps derive every token from the same session value, so a page either app rendered posts
//! to the other (the old-tab case across the cutover).

use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use hmac::{Hmac, Mac};
use sha2::Sha256;

/// `session[:_csrf_token]` (`SessionStore`, the default `csrf_token_storage_strategy`).
pub const SESSION_KEY: &str = "_csrf_token";
/// `request_forgery_protection_token`
pub const PARAM: &str = "authenticity_token";
/// `request.x_csrf_token`
pub const HEADER: &str = "x-csrf-token";
/// `AUTHENTICITY_TOKEN_LENGTH`
pub const TOKEN_LENGTH: usize = 32;
/// `GLOBAL_CSRF_TOKEN_IDENTIFIER`
const GLOBAL_IDENTIFIER: &str = "!real_csrf_token";

/// `generate_csrf_token`: `SecureRandom.urlsafe_base64(32)`, the value stored in the session.
pub fn generate() -> String {
    encode(&rand::random::<[u8; TOKEN_LENGTH]>())
}

/// The session's token, decoded (`real_csrf_token`), with the tokens derived from it.
#[derive(Clone)]
pub struct RealToken {
    bytes: Vec<u8>,
}

impl std::fmt::Debug for RealToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RealToken([FILTERED])")
    }
}

impl RealToken {
    /// `decode_csrf_token(session[:_csrf_token])`; `None` when it isn't base64.
    pub fn decode(encoded: &str) -> Option<Self> {
        urlsafe_decode(encoded).map(|bytes| Self { bytes })
    }

    /// `global_csrf_token`
    pub fn global(&self) -> Vec<u8> {
        self.hmac(GLOBAL_IDENTIFIER)
    }

    /// `per_form_csrf_token(session, action_path, method)`
    pub fn per_form(&self, action_path: &str, method: &str) -> Vec<u8> {
        self.hmac(&format!("{action_path}#{}", method.to_lowercase()))
    }

    /// `csrf_token_hmac`
    fn hmac(&self, identifier: &str) -> Vec<u8> {
        let mut mac = Hmac::<Sha256>::new_from_slice(&self.bytes).expect("HMAC takes any key length");
        mac.update(identifier.as_bytes());
        mac.finalize().into_bytes().to_vec()
    }

    /// `masked_authenticity_token(form_options:)`: the global token, or with `form` (an action
    /// path already normalized, and a method) the per-form one, masked with a fresh pad.
    pub fn masked(&self, form: Option<(&str, &str)>) -> String {
        let raw = match form {
            Some((action_path, method)) => self.per_form(action_path, method),
            None => self.global(),
        };
        mask(&raw, rand::random())
    }

    /// `valid_authenticity_token?(session, encoded_masked_token)` for a request to `path` with
    /// `method` (after the `_method` override).
    pub fn is_valid(&self, encoded: &str, path: &str, method: &str) -> bool {
        if encoded.is_empty() {
            return false;
        }
        let Some(token) = urlsafe_decode(encoded) else { return false };
        if token.len() == TOKEN_LENGTH {
            // An unmasked token, as issued before masking.
            fixed_length_secure_compare(&token, &self.bytes)
        } else if token.len() == TOKEN_LENGTH * 2 {
            let (pad, encrypted) = token.split_at(TOKEN_LENGTH);
            let unmasked: Vec<u8> = pad.iter().zip(encrypted).map(|(a, b)| a ^ b).collect();
            fixed_length_secure_compare(&unmasked, &self.global())
                || fixed_length_secure_compare(&unmasked, &self.bytes)
                || fixed_length_secure_compare(&unmasked, &self.per_form(path.strip_suffix('/').unwrap_or(path), method))
        } else {
            false
        }
    }
}

/// The tokens a request's views embed: its session's token and path, so forms rendered after the
/// action get tokens without the [`crate::Ctx`] (they're cheap to clone and `Send`).
#[derive(Debug, Clone)]
pub struct AuthenticityTokens {
    real: RealToken,
    request_path: String,
}

impl AuthenticityTokens {
    pub fn new(real: RealToken, request_path: &str) -> Self {
        Self { real, request_path: request_path.to_string() }
    }

    /// `form_authenticity_token`: the masked global token (`csrf_meta_tags`).
    pub fn global(&self) -> String {
        self.real.masked(None)
    }

    /// `form_authenticity_token(form_options: { action:, method: })`: the per-form token for a
    /// form posting to `action` (as written in the page) with `method`.
    pub fn for_form(&self, action: &str, method: &str) -> String {
        let action_path = normalize_action_path(action, &self.request_path);
        self.real.masked(Some((&action_path, method)))
    }
}

/// `mask_token` with the given one-time pad: `pad + (pad XOR raw)`, URL-safe base64 unpadded.
pub fn mask(raw: &[u8], pad: [u8; TOKEN_LENGTH]) -> String {
    let mut masked = pad.to_vec();
    masked.extend(pad.iter().zip(raw).map(|(a, b)| a ^ b));
    encode(&masked)
}

/// `ActiveSupport::SecurityUtils.fixed_length_secure_compare`, which raises on a length mismatch
/// (never the case here: every token compared is 32 bytes).
fn fixed_length_secure_compare(a: &[u8], b: &[u8]) -> bool {
    use subtle::ConstantTimeEq;
    a.len() == b.len() && bool::from(a.ct_eq(b))
}

/// `normalize_action_path`: a form's `action` as the path its per-form token is bound to. A
/// relative action (or none) is appended to the page's path, like Rails does, not resolved the
/// way a browser would.
pub fn normalize_action_path(action: &str, request_path: &str) -> String {
    let path = uri_path(action);
    let relative = !has_scheme(action) && !action.starts_with('/');
    let path = if relative {
        // normalize_relative_action_path
        format!("{}/{path}", uri_path(request_path)).replace("/./", "/")
    } else {
        path.to_string()
    };
    path.strip_suffix('/').map(str::to_string).unwrap_or(path)
}

/// `URI.parse(url).path`: what's left once the scheme, authority, query and fragment are gone.
fn uri_path(url: &str) -> &str {
    let rest = if has_scheme(url) { &url[url.find(':').unwrap() + 1..] } else { url };
    let rest = match rest.strip_prefix("//") {
        Some(authority_and_path) => authority_and_path.find('/').map_or("", |i| &authority_and_path[i..]),
        None => rest,
    };
    let end = rest.find(['?', '#']).unwrap_or(rest.len());
    &rest[..end]
}

fn has_scheme(url: &str) -> bool {
    match url.find(':') {
        Some(i) => {
            let scheme = &url[..i];
            scheme.bytes().next().is_some_and(|b| b.is_ascii_alphabetic())
                && scheme.bytes().all(|b| b.is_ascii_alphanumeric() || b"+-.".contains(&b))
                && !url[..i].contains(['/', '?', '#'])
        }
        None => false,
    }
}

/// `valid_request_origin?`: `Err` for the `null` origin (Rails raises), else whether a present
/// `Origin` equals the request's base URL.
pub fn valid_request_origin(origin: Option<&str>, base_url: &str) -> Result<bool, NullOrigin> {
    match origin {
        Some("null") => Err(NullOrigin),
        Some(origin) => Ok(origin == base_url),
        None => Ok(true),
    }
}

/// `Base64.urlsafe_encode64(bytes, padding: false)`
fn encode(bytes: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(bytes)
}

/// `Base64.urlsafe_decode64`: pads a short unpadded string, then decodes strictly, accepting
/// either alphabet.
fn urlsafe_decode(encoded: &str) -> Option<Vec<u8>> {
    let mut translated: String = encoded.chars().map(|c| match c { '-' => '+', '_' => '/', c => c }).collect();
    if !encoded.ends_with('=') {
        while !translated.len().is_multiple_of(4) {
            translated.push('=');
        }
    }
    STANDARD.decode(translated).ok()
}

/// The browser sent `Origin: null` (Rails' `NULL_ORIGIN_MESSAGE`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NullOrigin;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn vectors() -> Value {
        let json: Value = serde_json::from_str(include_str!("../../../vectors/kit_security.json")).unwrap();
        json["csrf"].clone()
    }

    fn session_token(vectors: &Value) -> RealToken {
        RealToken::decode(vectors["session_token"].as_str().unwrap()).unwrap()
    }

    #[test]
    fn global_token_matches_rails() {
        let vectors = vectors();
        assert_eq!(hex::encode(session_token(&vectors).global()), vectors["global_token_hex"].as_str().unwrap());
    }

    #[test]
    fn per_form_tokens_and_action_paths_match_rails() {
        let vectors = vectors();
        let token = session_token(&vectors);
        for case in vectors["form_tokens"].as_array().unwrap() {
            let action = case["action"].as_str().unwrap();
            let page = case["page_path"].as_str().unwrap();
            let normalized = normalize_action_path(action, page);
            assert_eq!(normalized, case["normalized_action_path"].as_str().unwrap(), "{action:?} on {page}");
            let method = case["method"].as_str().unwrap();
            assert_eq!(hex::encode(token.per_form(&normalized, method)), case["unmasked_hex"].as_str().unwrap(), "{case}");
        }
    }

    #[test]
    fn validity_matrix_matches_rails() {
        let vectors = vectors();
        let token = session_token(&vectors);
        let cases = vectors["validity"].as_array().unwrap();
        assert!(cases.len() > 100);
        for case in cases {
            let valid = token.is_valid(case["token"].as_str().unwrap(), case["path"].as_str().unwrap(), case["method"].as_str().unwrap());
            assert_eq!(valid, case["expected"].as_bool().unwrap(), "{case}");
        }
    }

    #[test]
    fn origin_check_matches_rails() {
        for case in vectors()["origin"].as_array().unwrap() {
            let result = valid_request_origin(case["origin"].as_str(), case["base_url"].as_str().unwrap());
            match &case["expected"] {
                Value::Bool(expected) => assert_eq!(result, Ok(*expected), "{case}"),
                _ => assert_eq!(result, Err(NullOrigin), "{case}"),
            }
        }
    }

    #[test]
    fn masked_tokens_rails_issued_verify_here() {
        let vectors = vectors();
        let token = session_token(&vectors);
        for masked in vectors["global_tokens"].as_array().unwrap() {
            assert!(token.is_valid(masked.as_str().unwrap(), "/anything", "DELETE"));
        }
    }

    #[test]
    fn masking_round_trips_and_varies() {
        let token = RealToken::decode(&generate()).unwrap();
        let first = token.masked(None);
        let second = token.masked(None);
        assert_ne!(first, second, "a fresh pad each time");
        assert_eq!(first.len(), 86, "64 bytes, unpadded URL-safe base64");
        assert!(token.is_valid(&first, "/session", "POST"));
        assert!(token.is_valid(&second, "/rooms/1", "PATCH"));

        let form = token.masked(Some(("/session", "post")));
        assert!(token.is_valid(&form, "/session", "POST"));
        assert!(token.is_valid(&form, "/session/", "POST"), "the request path's trailing slash is ignored");
        assert!(!token.is_valid(&form, "/session", "DELETE"), "bound to the method");
        assert!(!token.is_valid(&form, "/rooms/1", "POST"), "bound to the action");
    }

    #[test]
    fn tampered_and_foreign_tokens_are_rejected() {
        let token = RealToken::decode(&generate()).unwrap();
        let other = RealToken::decode(&generate()).unwrap();
        let masked = token.masked(None);
        assert!(!other.is_valid(&masked, "/session", "POST"), "another session's token");

        let mut bytes = urlsafe_decode(&masked).unwrap();
        for index in [0, 31, 32, 63] {
            bytes[index] ^= 1;
            let tampered = encode(&bytes);
            assert!(!token.is_valid(&tampered, "/session", "POST"), "flipped byte {index}");
            bytes[index] ^= 1;
        }
        for malformed in ["", "!!!!", "QQ", &masked[..masked.len() - 2]] {
            assert!(!token.is_valid(malformed, "/session", "POST"), "{malformed:?}");
        }
    }

    #[test]
    fn generated_tokens_are_32_random_bytes() {
        let encoded = generate();
        assert_eq!(encoded.len(), 43);
        assert!(!encoded.contains('='));
        assert_ne!(encoded, generate());
        assert_eq!(urlsafe_decode(&encoded).unwrap().len(), TOKEN_LENGTH);
    }
}
