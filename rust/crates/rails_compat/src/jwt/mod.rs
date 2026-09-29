//! JSON Web Tokens with the semantics of the `jwt` gem (ruby-jwt 3.2, `JWT.encode`/`JWT.decode`)
//! that `app/services/huddle.rb` and `app/models/google/sign_in/id_token_verifier.rb` call. Only
//! HS256 and RS256, the two Smartfire uses. RS256 verification is `ring`'s.
//!
//! Decoding follows `JWT::Decode#decode_segments`: exactly three dot-separated segments; a JSON
//! object header whose `alg` matches the expected algorithm ASCII case-insensitively; the
//! signature over the first two segments as given (each segment Base64url with optional padding,
//! either alphabet); then the claims: `exp` (`exp.to_i <= now - leeway` is expired), `nbf`
//! (`nbf.to_i > now + leeway` is premature), `iss` when asked for, and required claims.
use hmac::{Hmac, Mac};
use serde_json::{Map, Value};
use sha2::Sha256;

use crate::message_verifier::constant_time_eq;
use crate::{encoding, json, ruby};

pub mod google;
pub mod livekit;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum JwtError {
    /// Wrong segment count, bad Base64 or JSON, a header that isn't an object, an empty payload,
    /// or a claim of a type Ruby would raise on.
    #[error("malformed token")]
    Malformed,
    #[error("unexpected algorithm")]
    IncorrectAlgorithm,
    #[error("signature verification failed")]
    Signature,
    #[error("signature has expired")]
    Expired,
    #[error("signature nbf has not been reached")]
    Immature,
    #[error("invalid issuer")]
    InvalidIssuer,
    #[error("missing required claim {0}")]
    MissingClaim(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Decoded {
    pub header: Map<String, Value>,
    pub payload: Value,
}

/// `JWT.decode`'s claim options. [`Default`] is the gem's: `exp` and `nbf` checked with no
/// leeway, no issuer, no required claims.
#[derive(Debug, Clone, Copy)]
pub struct Validation<'a> {
    pub verify_expiration: bool,
    pub verify_not_before: bool,
    pub leeway: i64,
    pub issuer: Option<&'a str>,
    pub required_claims: &'a [&'a str],
}

impl Default for Validation<'_> {
    fn default() -> Self {
        Self { verify_expiration: true, verify_not_before: true, leeway: 0, issuer: None, required_claims: &[] }
    }
}

/// What verifies the signature, which also fixes the algorithm (`algorithm: "HS256"` or
/// `"RS256"`).
#[derive(Debug, Clone, Copy)]
pub enum Key<'a> {
    Hs256(&'a [u8]),
    Rs256(&'a RsaPublicKey),
}

impl Key<'_> {
    fn algorithm(&self) -> &'static str {
        match self {
            Key::Hs256(_) => "HS256",
            Key::Rs256(_) => "RS256",
        }
    }

    fn verify(&self, signing_input: &[u8], signature: &[u8]) -> Result<(), JwtError> {
        match self {
            // JWA::Hmac#verify: the key must be a non-empty string.
            Key::Hs256([]) => Err(JwtError::Malformed),
            Key::Hs256(key) => constant_time_eq(signature, &hs256(key, signing_input)).then_some(()).ok_or(JwtError::Signature),
            Key::Rs256(key) => key.verify(signing_input, signature),
        }
    }
}

/// An RSA public key from its big-endian modulus and exponent (a JWK's `n` and `e`).
///
/// Verification uses `ring`'s `RSA_PKCS1_2048_8192_SHA256`, so moduli under 2048 bits (and over
/// 8192) fail verification. OpenSSL, which Rails uses, would verify with a smaller key; Google's
/// keys are 2048-bit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RsaPublicKey {
    n: Vec<u8>,
    e: Vec<u8>,
}

impl RsaPublicKey {
    /// `None` if either component is zero.
    pub fn from_components(n: &[u8], e: &[u8]) -> Option<Self> {
        let strip = |bytes: &[u8]| bytes.iter().position(|&b| b != 0).map(|start| bytes[start..].to_vec());
        Some(Self { n: strip(n)?, e: strip(e)? })
    }

    /// The modulus, big-endian without leading zeros.
    pub fn modulus(&self) -> &[u8] {
        &self.n
    }

    fn verify(&self, message: &[u8], signature: &[u8]) -> Result<(), JwtError> {
        ring::signature::RsaPublicKeyComponents { n: &self.n, e: &self.e }
            .verify(&ring::signature::RSA_PKCS1_2048_8192_SHA256, message, signature)
            .map_err(|_| JwtError::Signature)
    }
}

fn hs256(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC takes a key of any length");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

/// `JWT.encode(payload, key, "HS256")`: header `{"alg":"HS256"}`, both dumped with
/// `JSON.generate` (keys in insertion order), Base64url without padding.
pub fn encode_hs256(payload: &Map<String, Value>, key: &[u8]) -> String {
    let header = json::generate(&serde_json::json!({ "alg": "HS256" }));
    let signing_input = format!(
        "{}.{}",
        encoding::urlsafe_encode_unpadded(header.as_bytes()),
        encoding::urlsafe_encode_unpadded(json::generate(&Value::Object(payload.clone())).as_bytes())
    );
    let signature = hs256(key, signing_input.as_bytes());
    format!("{signing_input}.{}", encoding::urlsafe_encode_unpadded(&signature))
}

/// The segments as `EncodedToken` splits them (`jwt.split('.')`, trailing empty ones dropped).
struct Segments<'a> {
    header: &'a str,
    payload: Option<&'a str>,
    signature: &'a str,
}

impl<'a> Segments<'a> {
    fn split(token: &'a str, require_signature: bool) -> Result<Self, JwtError> {
        // validate_segment_count!: counts dots, so "a.b." has three segments.
        let count = token.matches('.').count() + 1;
        if !(count == 3 || (!require_signature && count == 2)) {
            return Err(JwtError::Malformed);
        }
        let parts = ruby::split(token, '.');
        Ok(Self {
            header: parts.first().copied().unwrap_or(""),
            payload: parts.get(1).copied(),
            signature: parts.get(2).copied().unwrap_or(""),
        })
    }

    fn signing_input(&self) -> String {
        format!("{}.{}", self.header, self.payload.unwrap_or(""))
    }

    fn header(&self) -> Result<Value, JwtError> {
        parse(&decode_segment(self.header)?)
    }

    /// `EncodedToken#decode_payload`, including the unencoded (`b64: false`, RFC 7797) form.
    fn payload(&self, header: &Map<String, Value>) -> Result<Value, JwtError> {
        let payload = self.payload.unwrap_or("");
        if self.payload == Some("") {
            return Err(JwtError::Malformed);
        }
        if header.get("b64") == Some(&Value::Bool(false)) {
            let crit_has_b64 = header.get("crit").and_then(Value::as_array).is_some_and(|crit| crit.iter().any(|c| c == "b64"));
            return if crit_has_b64 { parse(payload.as_bytes()) } else { Err(JwtError::Malformed) };
        }
        parse(&decode_segment(payload)?)
    }
}

fn decode_segment(segment: &str) -> Result<Vec<u8>, JwtError> {
    encoding::urlsafe_decode(segment).ok_or(JwtError::Malformed)
}

fn parse(bytes: &[u8]) -> Result<Value, JwtError> {
    serde_json::from_slice(bytes).map_err(|_| JwtError::Malformed)
}

/// `JWT.decode(token, key, true, algorithm:, **validation)` at `now` (unix seconds).
pub fn decode(token: &str, key: Key<'_>, validation: &Validation<'_>, now: i64) -> Result<Decoded, JwtError> {
    let segments = Segments::split(token, true)?;

    // verify_algo
    let Value::Object(header) = segments.header()? else { return Err(JwtError::Malformed) };
    match header.get("alg") {
        Some(Value::String(alg)) if alg.eq_ignore_ascii_case(key.algorithm()) => {}
        _ => return Err(JwtError::IncorrectAlgorithm),
    }

    // verify_signature
    key.verify(segments.signing_input().as_bytes(), &decode_segment(segments.signature)?)?;

    // Claims::DecodeVerifier
    let payload = segments.payload(&header)?;
    verify_claims(&payload, validation, now)?;
    Ok(Decoded { header, payload })
}

/// `JWT.decode(token, nil, false)`: both segments parsed, nothing verified. The header may be any
/// JSON value, as in Ruby.
pub fn decode_unverified(token: &str) -> Result<(Value, Value), JwtError> {
    let segments = Segments::split(token, false)?;
    let header = segments.header()?;
    let payload = segments.payload(header.as_object().unwrap_or(&Map::new()))?;
    Ok((header, payload))
}

fn verify_claims(payload: &Value, validation: &Validation<'_>, now: i64) -> Result<(), JwtError> {
    let claims = payload.as_object();
    let claim = |name: &str| claims.and_then(|claims| claims.get(name));

    if validation.verify_expiration
        && let Some(exp) = claim("exp")
        && ruby::to_i(exp).ok_or(JwtError::Malformed)? <= (now - validation.leeway) as i128
    {
        return Err(JwtError::Expired);
    }
    if validation.verify_not_before
        && let Some(nbf) = claim("nbf")
        && ruby::to_i(nbf).ok_or(JwtError::Malformed)? > (now + validation.leeway) as i128
    {
        return Err(JwtError::Immature);
    }
    if let Some(issuer) = validation.issuer {
        // `payload['iss']` raises on an array or number payload; a string payload has no `iss`.
        if !matches!(claim("iss"), Some(Value::String(iss)) if iss == issuer) {
            return Err(if payload.is_array() || payload.is_number() { JwtError::Malformed } else { JwtError::InvalidIssuer });
        }
    }
    for required in validation.required_claims {
        if claim(required).is_none() {
            return Err(JwtError::MissingClaim(required.to_string()));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn payload(value: Value) -> Map<String, Value> {
        value.as_object().unwrap().clone()
    }

    #[test]
    fn hs256_round_trip_and_tamper() {
        let token = encode_hs256(&payload(json!({"exp": 200, "sub": "x"})), b"secret");
        let decoded = decode(&token, Key::Hs256(b"secret"), &Validation::default(), 100).unwrap();
        assert_eq!(decoded.payload["sub"], "x");

        assert_eq!(decode(&token, Key::Hs256(b"other"), &Validation::default(), 100), Err(JwtError::Signature));
        assert_eq!(decode(&token, Key::Hs256(b""), &Validation::default(), 100), Err(JwtError::Malformed));
        assert_eq!(decode(&token, Key::Hs256(b"secret"), &Validation::default(), 200), Err(JwtError::Expired));
        let mut tampered = token.clone().into_bytes();
        let middle = token.find('.').unwrap() + 2;
        tampered[middle] = if tampered[middle] == b'A' { b'B' } else { b'A' };
        assert!(decode(std::str::from_utf8(&tampered).unwrap(), Key::Hs256(b"secret"), &Validation::default(), 100).is_err());
    }

    #[test]
    fn the_algorithm_is_pinned_by_the_key() {
        let token = encode_hs256(&payload(json!({"sub": "x"})), b"secret");
        let key = RsaPublicKey::from_components(&[0xc5; 256], &[1, 0, 1]).unwrap();
        assert_eq!(decode(&token, Key::Rs256(&key), &Validation::default(), 0), Err(JwtError::IncorrectAlgorithm));

        let none = format!("{}.{}.", encoding::urlsafe_encode_unpadded(br#"{"alg":"none"}"#), encoding::urlsafe_encode_unpadded(br#"{"sub":"x"}"#));
        assert_eq!(decode(&none, Key::Hs256(b"secret"), &Validation::default(), 0), Err(JwtError::IncorrectAlgorithm));
    }
}
