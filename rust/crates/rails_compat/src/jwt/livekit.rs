//! LiveKit access tokens: minting (`Huddle#token` in `app/services/huddle.rb`,
//! `Huddle::RoomService#admin_token` in `app/services/huddle/room_service.rb`) and checking the
//! tokens LiveKit's gateway hands back (`Huddle::TokenVerifier#coordinates`,
//! `app/services/huddle/token_verifier.rb`). HS256 with `LIVEKIT_API_SECRET`, issuer
//! `LIVEKIT_API_KEY`.
use hmac::{Hmac, Mac};
use serde_json::{Map, Value};
use sha2::Sha256;

use super::{Decoded, Key, Validation, decode, encode_hs256};
use crate::ruby;

/// `Huddle::TOKEN_TTL`, in seconds.
pub const TOKEN_TTL: i64 = 120;
/// `Huddle::RoomService::TOKEN_TTL`, in seconds.
pub const ADMIN_TOKEN_TTL: i64 = 60;
/// `nbf` is backdated this far.
pub const NOT_BEFORE_SKEW: i64 = 5;
/// `Huddle::PUBLISH_SOURCES`, in the order the grant lists them.
pub const PUBLISH_SOURCES: [&str; 4] = ["microphone", "screen_share", "screen_share_audio", "camera"];

const REQUIRED_CLAIMS: [&str; 5] = ["exp", "iss", "nbf", "sub", "video"];
const REQUIRED_VIDEO_PERMISSIONS: [&str; 2] = ["roomJoin", "canSubscribe"];
const FORBIDDEN_VIDEO_PERMISSIONS: [&str; 11] = [
    "roomCreate", "roomList", "roomRecord", "roomAdmin", "canPublishData", "canUpdateOwnMetadata", "ingressAdmin", "hidden", "recorder", "agent",
    "canPublishTranscription",
];
const OTHER_VIDEO_CLAIMS: [&str; 3] = ["room", "canPublish", "canPublishSources"];

/// `Huddle.opaque_identifier(kind, record_id)`: `campfire-<kind>-<hex HMAC-SHA256(api_secret,
/// "campfire-huddle:<kind>:<id>")>`.
pub fn opaque_identifier(api_secret: &str, kind: &str, record_id: i64) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(api_secret.as_bytes()).expect("HMAC takes a key of any length");
    mac.update(format!("campfire-huddle:{kind}:{record_id}").as_bytes());
    format!("campfire-{kind}-{}", hex::encode(mac.finalize().into_bytes()))
}

/// `Huddle.room_name(room_id)`.
pub fn room_name(api_secret: &str, room_id: i64) -> String {
    opaque_identifier(api_secret, "room", room_id)
}

/// `Huddle.participant_video_grant(room_name, publish:)`, keys in Ruby's order.
pub fn participant_video_grant(room_name: &str, publish: bool) -> Map<String, Value> {
    let mut grant = Map::new();
    grant.insert("room".into(), room_name.into());
    grant.insert("roomJoin".into(), true.into());
    for permission in ["roomCreate", "roomList", "roomAdmin", "roomRecord"] {
        grant.insert(permission.into(), false.into());
    }
    grant.insert("canPublish".into(), publish.into());
    grant.insert("canPublishData".into(), false.into());
    let sources = if publish { PUBLISH_SOURCES.iter().map(|s| Value::from(*s)).collect() } else { Vec::new() };
    grant.insert("canPublishSources".into(), Value::Array(sources));
    grant.insert("canSubscribe".into(), true.into());
    grant
}

/// `Huddle#can_publish?`: a server mute revokes publishing; in a stage room only hosts and
/// speakers publish.
pub fn can_publish(server_muted: bool, stage_room: bool, stage_role: Option<&str>) -> bool {
    !server_muted && (!stage_room || matches!(stage_role, Some("host" | "speaker")))
}

/// What `Huddle#token` signs.
#[derive(Debug, Clone, Copy)]
pub struct Participant<'a> {
    /// `user.name`.
    pub name: &'a str,
    /// `grant.identity`.
    pub identity: &'a str,
    /// `grant.room_name`.
    pub room_name: &'a str,
    pub can_publish: bool,
}

/// `Huddle#token` at `now` (unix seconds), with a fresh `jti` (`SecureRandom.uuid`).
pub fn participant_token(api_key: &str, api_secret: &str, participant: &Participant<'_>, now: i64) -> String {
    participant_token_with_jti(api_key, api_secret, participant, now, &uuid::Uuid::new_v4().to_string())
}

pub fn participant_token_with_jti(api_key: &str, api_secret: &str, participant: &Participant<'_>, now: i64, jti: &str) -> String {
    let mut claims = Map::new();
    claims.insert("exp".into(), (now + TOKEN_TTL).into());
    claims.insert("iat".into(), now.into());
    claims.insert("iss".into(), api_key.into());
    claims.insert("jti".into(), jti.into());
    claims.insert("name".into(), participant.name.into());
    claims.insert("nbf".into(), (now - NOT_BEFORE_SKEW).into());
    claims.insert("sub".into(), participant.identity.into());
    claims.insert("video".into(), Value::Object(participant_video_grant(participant.room_name, participant.can_publish)));
    encode_hs256(&claims, api_secret.as_bytes())
}

/// `RoomService#admin_token(grant)` at `now`, with a fresh `jti`.
pub fn admin_token(api_key: &str, api_secret: &str, grant: &Map<String, Value>, now: i64) -> String {
    admin_token_with_jti(api_key, api_secret, grant, now, &uuid::Uuid::new_v4().to_string())
}

pub fn admin_token_with_jti(api_key: &str, api_secret: &str, grant: &Map<String, Value>, now: i64, jti: &str) -> String {
    let mut claims = Map::new();
    claims.insert("exp".into(), (now + ADMIN_TOKEN_TTL).into());
    claims.insert("iat".into(), now.into());
    claims.insert("iss".into(), api_key.into());
    claims.insert("jti".into(), jti.into());
    claims.insert("nbf".into(), (now - NOT_BEFORE_SKEW).into());
    claims.insert("video".into(), Value::Object(grant.clone()));
    encode_hs256(&claims, api_secret.as_bytes())
}

/// `TokenVerifier#coordinates`' result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Coordinates {
    pub identity: String,
    pub room_name: String,
}

/// `Huddle::TokenVerifier::Invalid`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("invalid LiveKit token")]
pub struct Invalid;

/// `Huddle::TokenVerifier.new(token).coordinates` at `now`: a token we minted (or LiveKit
/// refreshed), signed with our secret, current, with exactly a participant's grant shape.
pub fn verify(token: &str, api_key: &str, api_secret: &str, now: i64) -> Result<Coordinates, Invalid> {
    let validation = Validation { issuer: Some(api_key), required_claims: &REQUIRED_CLAIMS, ..Validation::default() };
    let Decoded { payload, .. } = decode(token, Key::Hs256(api_secret.as_bytes()), &validation, now).map_err(|_| Invalid)?;
    let claims = payload.as_object().ok_or(Invalid)?;

    let is_integer = |key: &str| claims.get(key).is_some_and(|value| value.is_i64() || value.is_u64());
    if !(is_integer("exp") && is_integer("nbf")) {
        return Err(Invalid);
    }
    let video = claims.get("video").and_then(Value::as_object).ok_or(Invalid)?;
    let room_name = video.get("room").ok_or(Invalid)?;

    let present_string = |value: Option<&Value>| value.and_then(Value::as_str).filter(|s| !ruby::str_blank(s)).map(str::to_string);
    let identity = present_string(claims.get("sub")).ok_or(Invalid)?;
    let room_name = present_string(Some(room_name)).ok_or(Invalid)?;

    let allowed = |key: &str| REQUIRED_VIDEO_PERMISSIONS.contains(&key) || FORBIDDEN_VIDEO_PERMISSIONS.contains(&key) || OTHER_VIDEO_CLAIMS.contains(&key);
    if !video.keys().all(|key| allowed(key)) {
        return Err(Invalid);
    }
    if !REQUIRED_VIDEO_PERMISSIONS.iter().all(|permission| video.get(*permission) == Some(&Value::Bool(true))) {
        return Err(Invalid);
    }
    if FORBIDDEN_VIDEO_PERMISSIONS.iter().any(|permission| ruby::truthy(video.get(*permission))) {
        return Err(Invalid);
    }

    // Publishers carry exactly the publish sources; listeners (LiveKit's refreshed tokens drop
    // false permissions and empty lists) carry no publish permission and no sources.
    let can_publish = match video.get("canPublish") {
        None | Some(Value::Null) => None,
        Some(Value::Bool(publish)) => Some(*publish),
        Some(_) => return Err(Invalid),
    };
    let sources = match video.get("canPublishSources") {
        None | Some(Value::Null) => None,
        Some(Value::Array(sources)) => Some(sources.iter().map(|s| s.as_str().ok_or(Invalid)).collect::<Result<Vec<_>, _>>()?),
        Some(_) => return Err(Invalid),
    };
    if can_publish == Some(true) {
        let mut sources = sources.ok_or(Invalid)?;
        let mut expected = PUBLISH_SOURCES.to_vec();
        sources.sort_unstable();
        expected.sort_unstable();
        if sources != expected {
            return Err(Invalid);
        }
    } else if sources.is_some_and(|sources| !sources.is_empty()) {
        return Err(Invalid);
    }

    Ok(Coordinates { identity, room_name })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoding;

    const KEY: &str = "APIkey";
    const SECRET: &str = "livekit-secret";
    const NOW: i64 = 1_767_268_800;

    fn token(can_publish: bool) -> String {
        participant_token(KEY, SECRET, &Participant { name: "N", identity: "campfire-participant-x", room_name: "campfire-room-y", can_publish }, NOW)
    }

    #[test]
    fn minted_tokens_verify_until_they_expire() {
        let expected = Coordinates { identity: "campfire-participant-x".into(), room_name: "campfire-room-y".into() };
        for publish in [true, false] {
            assert_eq!(verify(&token(publish), KEY, SECRET, NOW), Ok(expected.clone()));
            assert_eq!(verify(&token(publish), KEY, SECRET, NOW + TOKEN_TTL - 1), Ok(expected.clone()));
            assert_eq!(verify(&token(publish), KEY, SECRET, NOW + TOKEN_TTL), Err(Invalid));
            assert_eq!(verify(&token(publish), KEY, SECRET, NOW - NOT_BEFORE_SKEW - 1), Err(Invalid));
        }
    }

    #[test]
    fn wrong_keys_tampering_and_admin_tokens_are_refused() {
        let token = token(true);
        assert_eq!(verify(&token, KEY, "other-secret", NOW), Err(Invalid));
        assert_eq!(verify(&token, "otherKey", SECRET, NOW), Err(Invalid));
        for index in 0..token.len() {
            let mut tampered = token.clone().into_bytes();
            tampered[index] = if tampered[index] == b'A' { b'B' } else { b'A' };
            let tampered = String::from_utf8(tampered).unwrap();
            if tampered != token && !tampered.contains('=') {
                assert_eq!(verify(&tampered, KEY, SECRET, NOW), Err(Invalid), "byte {index}");
            }
        }
        let mut grant = Map::new();
        grant.insert("roomAdmin".into(), true.into());
        grant.insert("room".into(), "campfire-room-y".into());
        assert_eq!(verify(&admin_token(KEY, SECRET, &grant, NOW), KEY, SECRET, NOW), Err(Invalid));
    }

    #[test]
    fn a_signed_grant_with_more_power_is_refused() {
        let mut claims: Map<String, Value> = serde_json::from_slice(&encoding::urlsafe_decode(token(true).split('.').nth(1).unwrap()).unwrap()).unwrap();
        for (permission, value) in [("roomAdmin", Value::Bool(true)), ("canPublishData", Value::Bool(true)), ("hidden", Value::from(1)), ("unknown", Value::Bool(false))] {
            let mut claims = claims.clone();
            claims["video"].as_object_mut().unwrap().insert(permission.into(), value);
            assert_eq!(verify(&encode_hs256(&claims, SECRET.as_bytes()), KEY, SECRET, NOW), Err(Invalid), "{permission}");
        }
        claims["video"]["canPublish"] = false.into();
        assert_eq!(verify(&encode_hs256(&claims, SECRET.as_bytes()), KEY, SECRET, NOW), Err(Invalid), "sources without canPublish");
    }
}
