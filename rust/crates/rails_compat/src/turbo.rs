//! `Turbo::StreamsChannel.signed_stream_name` / `verified_stream_name` (turbo-rails
//! `app/channels/turbo/streams/stream_name.rb`). The verifier is
//! `MessageVerifier.new(generate_key("turbo/signed_stream_verifier_key"), digest: "SHA256",
//! serializer: JSON)`: strict Base64 of the JSON-dumped stream name, no metadata envelope.
use jiff::Timestamp;
use serde_json::Value;

use crate::Secrets;
use crate::message_verifier::{Digest, Encoding, MessageVerifier, Serializer};

pub const SALT: &str = "turbo/signed_stream_verifier_key";

/// `streamables` are already-resolved stream name parts, joined with `:` like `stream_name_from`:
/// a record becomes its GID param (`GlobalId::to_param`), a symbol or string itself. So
/// `turbo_stream_from @room, :messages` is `[room_gid.to_param(), "messages"]`.
pub fn signed_stream_name(secrets: &Secrets, streamables: &[&str]) -> String {
    verifier(secrets).generate(&Value::String(streamables.join(":")), None, None)
}

/// The stream name, or `None` if the signature doesn't check out. (A validly signed non-string
/// can't come from `signed_stream_name`; numbers are returned as their `to_s` like Rails would.)
pub fn verified_stream_name(secrets: &Secrets, signed: &str) -> Option<String> {
    match verifier(secrets).verify(signed, None, Timestamp::UNIX_EPOCH).ok()? {
        Value::String(name) => Some(name),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

pub fn verifier(secrets: &Secrets) -> MessageVerifier {
    MessageVerifier::new(secrets.key_generator.generate_key(SALT, 64), Digest::Sha256, Encoding::Strict, Serializer::Json)
}
