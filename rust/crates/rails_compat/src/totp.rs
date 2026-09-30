//! `app/models/two_factor_credential.rb` and the reference image's ROTP 6.3.0.

use hmac::{Hmac, Mac};
use rand::Rng;
use sha1::Sha1;
use subtle::ConstantTimeEq;

const BASE32: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
pub const ISSUER: &str = "Smartfire";
pub const STEP_SECONDS: i64 = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum InvalidTotp {
    #[error("invalid TOTP secret")]
    Secret,
    #[error("invalid TOTP time")]
    Time,
}

/// ROTP generates 20 random bytes, which encode to 32 uniformly random base32 symbols.
pub fn generate_secret() -> String {
    let mut rng = rand::rng();
    (0..32)
        .map(|_| BASE32[rng.random_range(0..32)] as char)
        .collect()
}

pub fn at(secret: &str, unix_seconds: i64) -> Result<String, InvalidTotp> {
    code_for_step(
        &decode_base32(secret)?,
        unix_seconds.div_euclid(STEP_SECONDS),
    )
}

/// Our model removes Ruby `/\s+/` (ASCII whitespace), then asks ROTP for the most recent
/// matching step in the ±30-second window. `after` excludes its entire step and earlier.
pub fn verify_code(
    secret: &str,
    code: &str,
    unix_seconds: i64,
    after: Option<i64>,
) -> Result<Option<i64>, InvalidTotp> {
    let code: String = code.chars().filter(|c| !ruby_code_whitespace(*c)).collect();
    if code.is_empty() {
        return Ok(None);
    }
    let key = decode_base32(secret)?;
    let start = unix_seconds
        .checked_sub(STEP_SECONDS)
        .ok_or(InvalidTotp::Time)?
        .div_euclid(STEP_SECONDS);
    let end = unix_seconds
        .checked_add(STEP_SECONDS)
        .ok_or(InvalidTotp::Time)?
        .div_euclid(STEP_SECONDS);
    let mut matched = None;
    for step in start..=end {
        if after.is_some_and(|after| step <= after.div_euclid(STEP_SECONDS)) {
            continue;
        }
        let generated = code_for_step(&key, step)?;
        if bool::from(code.as_bytes().ct_eq(generated.as_bytes())) {
            matched = Some(step.checked_mul(STEP_SECONDS).ok_or(InvalidTotp::Time)?);
        }
    }
    Ok(matched)
}

pub fn ruby_code_whitespace(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r' | '\x0b' | '\x0c')
}

/// ROTP::OTP::URI: a colon in the account label becomes '_'; rstrip is Ruby's ASCII
/// whitespace (including NUL). Default SHA1, digits and period don't appear in the URI.
pub fn provisioning_uri(secret: &str, email_address: &str) -> String {
    let label = email_address
        .trim_end_matches(|c| ruby_code_whitespace(c) || c == '\0')
        .replace(':', "_");
    format!(
        "otpauth://totp/{ISSUER}:{}?secret={}&issuer={ISSUER}",
        uri_component(&label),
        uri_component(secret)
    )
}

fn uri_component(input: &str) -> String {
    let mut output = String::new();
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                output.push(byte as char)
            }
            _ => output.push_str(&format!("%{byte:02X}")),
        }
    }
    output
}

fn decode_base32(secret: &str) -> Result<Vec<u8>, InvalidTotp> {
    let mut output = Vec::new();
    let mut buffer = 0u32;
    let mut bits = 0;
    for c in secret
        .chars()
        .filter(|c| *c != '=')
        .flat_map(char::to_uppercase)
    {
        let value = BASE32
            .iter()
            .position(|byte| char::from(*byte) == c)
            .ok_or(InvalidTotp::Secret)?;
        buffer = (buffer << 5) | value as u32;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            output.push((buffer >> bits) as u8);
        }
    }
    Ok(output)
}

fn code_for_step(key: &[u8], step: i64) -> Result<String, InvalidTotp> {
    let counter = u64::try_from(step).map_err(|_| InvalidTotp::Time)?;
    let mut mac = Hmac::<Sha1>::new_from_slice(key).expect("HMAC accepts any key length");
    mac.update(&counter.to_be_bytes());
    let digest = mac.finalize().into_bytes();
    let offset = usize::from(digest[19] & 0x0f);
    let truncated = u32::from_be_bytes(digest[offset..offset + 4].try_into().expect("four bytes"))
        & 0x7fff_ffff;
    Ok(format!("{:06}", truncated % 1_000_000))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn vectors() -> Value {
        serde_json::from_str(include_str!("../../../vectors/two_factor.json")).unwrap()
    }

    #[test]
    fn rails_totp_codes_and_base32_coercions() {
        let v = vectors();
        let secret = v["secret"].as_str().unwrap();
        for case in v["codes"].as_array().unwrap() {
            assert_eq!(
                at(secret, case["at"].as_i64().unwrap()).unwrap(),
                case["code"]
            );
        }
        for case in v["base32"].as_array().unwrap() {
            let result = at(case["secret"].as_str().unwrap(), v["now"].as_i64().unwrap());
            if case["error"].is_null() {
                assert_eq!(result.unwrap(), case["code"]);
            } else {
                assert!(result.is_err());
            }
        }
    }

    #[test]
    fn rails_drift_edges_replay_guard_and_code_normalization() {
        let v = vectors();
        for case in v["checks"].as_array().unwrap() {
            assert_eq!(
                verify_code(
                    v["secret"].as_str().unwrap(),
                    case["code"].as_str().unwrap_or(""),
                    case["at"].as_i64().unwrap(),
                    case["after"].as_i64()
                )
                .unwrap(),
                case["matched_at"].as_i64(),
                "{case}"
            );
        }
    }

    #[test]
    fn newest_colliding_step_matches_rotp() {
        let v = vectors();
        let case = &v["collision"];
        assert_eq!(
            verify_code(
                v["secret"].as_str().unwrap(),
                case["code"].as_str().unwrap(),
                case["at"].as_i64().unwrap(),
                None
            )
            .unwrap(),
            case["matched_at"].as_i64()
        );
    }

    #[test]
    fn provisioning_uris_match_rotp() {
        let v = vectors();
        for case in v["provisioning"].as_array().unwrap() {
            assert_eq!(
                provisioning_uri(
                    v["secret"].as_str().unwrap(),
                    case["email"].as_str().unwrap()
                ),
                case["uri"]
            );
        }
    }

    #[test]
    fn rails_remember_cookie_rejects_tamper_wrong_purpose_and_expiry() {
        use crate::{Secrets, cookies};
        let v = vectors();
        let secrets = Secrets::new(v["secret_key_base"].as_str().unwrap());
        let raw = v["remember"]["raw"].as_str().unwrap();
        let now = jiff::Timestamp::from_second(v["now"].as_i64().unwrap()).unwrap();
        let expires =
            jiff::Timestamp::from_second(v["remember"]["expires_at"].as_i64().unwrap()).unwrap();
        assert_eq!(
            cookies::read_two_factor_remember(&secrets, raw, now).as_deref(),
            v["remember"]["token"].as_str()
        );
        let mut tampered = raw.to_string();
        tampered.replace_range(0..1, if raw.starts_with('a') { "b" } else { "a" });
        assert!(cookies::read_two_factor_remember(&secrets, &tampered, now).is_none());
        assert!(cookies::read_device_id(&secrets, raw, now).is_none());
        assert!(cookies::read_two_factor_remember(&secrets, raw, expires).is_none());
        assert_eq!(
            cookies::two_factor_remember_cookie(
                &secrets,
                v["remember"]["token"].as_str().unwrap(),
                now
            )
            .value,
            raw
        );
    }
}
