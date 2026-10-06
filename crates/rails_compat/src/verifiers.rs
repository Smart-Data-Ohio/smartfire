//! Smartfire's named `Rails.application.message_verifier(name)`s. Each is [`crate::app_verifier`]:
//! key `generate_key(name, 64)` from the app's (SHA1) key generator, HMAC-SHA1, strict Base64,
//! `:json_allow_marshal`. None of them sets a purpose. `verified` returns nil on any failure,
//! which is `None` here.
use jiff::{SignedDuration, Timestamp};
use serde_json::{Map, Value};

use crate::{Secrets, app_verifier, metadata, uri};

/// The OAuth `state` round trips. Each controller signs a random `SecureRandom.hex(16)` state
/// with `generate(raw_state)` (no purpose, no expiry) and reads `params[:state]` back with
/// `verified(params[:state].to_s)`, then compares it to the one kept in the session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OAuthState {
    /// `app/controllers/concerns/google_sign_in_flow.rb` (sign-in, re-auth, sudo, linking).
    GoogleSignIn,
    /// `app/controllers/google/connections_controller.rb` (Calendar connection).
    GoogleOAuth,
    /// `app/controllers/github/app_connections_controller.rb`.
    GithubAppOAuth,
    /// `app/controllers/slack/oauth_controller.rb`.
    SlackOAuth,
}

impl OAuthState {
    pub const ALL: [OAuthState; 4] = [OAuthState::GoogleSignIn, OAuthState::GoogleOAuth, OAuthState::GithubAppOAuth, OAuthState::SlackOAuth];

    pub fn verifier_name(self) -> &'static str {
        match self {
            OAuthState::GoogleSignIn => "google_sign_in_state",
            OAuthState::GoogleOAuth => "google_oauth_state",
            OAuthState::GithubAppOAuth => "github_app_oauth_state",
            OAuthState::SlackOAuth => "slack_oauth_state",
        }
    }

    /// `state_verifier.generate(raw_state)`.
    pub fn generate(self, secrets: &Secrets, raw_state: &str) -> String {
        app_verifier(secrets, self.verifier_name()).generate(&Value::String(raw_state.to_string()), None, None)
    }

    /// `state_verifier.verified(state)`: whatever value was signed (the controllers then compare
    /// it with the session's state, so a non-string simply fails that comparison).
    pub fn verified(self, secrets: &Secrets, state: &str, now: Timestamp) -> Option<Value> {
        app_verifier(secrets, self.verifier_name()).verify(state, None, now).ok()
    }
}

/// `Embeds::ImageProxy` (`app/models/embeds/image_proxy.rb`): link preview image URLs signed at
/// render time so the proxy only fetches what the server rendered.
pub mod embed_image {
    use super::*;

    pub const VERIFIER_NAME: &str = "embed_image";

    /// `verifier.generate(url.to_s)`: the `:signed` segment of `embed_image_path`.
    pub fn sign(secrets: &Secrets, url: &str) -> String {
        app_verifier(secrets, VERIFIER_NAME).generate(&Value::String(url.to_string()), None, None)
    }

    /// `verifier.verified(signed)`.
    pub fn verified(secrets: &Secrets, signed: &str, now: Timestamp) -> Option<Value> {
        app_verifier(secrets, VERIFIER_NAME).verify(signed, None, now).ok()
    }

    /// `Embeds::ImageProxy.verified_url(signed)`: a signed string that `URI.parse`s as http(s).
    pub fn verified_url(secrets: &Secrets, signed: &str, now: Timestamp) -> Option<String> {
        match verified(secrets, signed, now)? {
            Value::String(url) if uri::is_http(&url) => Some(url),
            _ => None,
        }
    }
}

/// Bot webhook reply tokens (`app/models/user/bot.rb`): `reply_token_for(room)` signs
/// `{bot_id:, room_id:}` for `REPLY_URL_EXPIRY` (15 minutes).
pub mod bot_reply {
    use super::*;

    pub const VERIFIER_NAME: &str = "bot_reply";
    pub const EXPIRES_IN: SignedDuration = SignedDuration::from_mins(15);

    /// `bot.reply_token_for(room)` at `now`.
    pub fn token_for(secrets: &Secrets, bot_id: i64, room_id: i64, now: Timestamp) -> String {
        let mut data = Map::new();
        data.insert("bot_id".into(), bot_id.into());
        data.insert("room_id".into(), room_id.into());
        app_verifier(secrets, VERIFIER_NAME).generate(&Value::Object(data), None, Some(now + EXPIRES_IN))
    }

    /// The token half of `User.authenticate_bot_reply_token(token, room_id:)`: the stripped token
    /// verifies, holds a hash, and its `room_id` matches `room_id` as strings (`to_s == to_s`).
    /// Returns the claimed `bot_id`. The caller must still do the rest of that method: find an
    /// *active bot* with that id (`active_bots.find_by(id:)`, which casts the value like Active
    /// Record does) that is a member of the room.
    pub fn verify(secrets: &Secrets, token: &str, room_id: &str, now: Timestamp) -> Option<Value> {
        let data = app_verifier(secrets, VERIFIER_NAME).verify(crate::ruby::strip(token), None, now).ok()?;
        let data = data.as_object()?;
        (metadata::ruby_to_s(data.get("room_id")) == room_id).then(|| data.get("bot_id").cloned().unwrap_or(Value::Null))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secrets() -> Secrets {
        Secrets::new(&"a".repeat(128))
    }

    fn now() -> Timestamp {
        "2026-01-01T12:00:00Z".parse().unwrap()
    }

    #[test]
    fn a_state_only_verifies_under_its_own_verifier_and_secret() {
        let secrets = secrets();
        for state in OAuthState::ALL {
            let message = state.generate(&secrets, "raw");
            assert_eq!(state.verified(&secrets, &message, now()), Some(Value::from("raw")));
            assert_eq!(state.verified(&Secrets::new(&"b".repeat(128)), &message, now()), None);
            for other in OAuthState::ALL.into_iter().filter(|other| *other != state) {
                assert_eq!(other.verified(&secrets, &message, now()), None, "{state:?} under {other:?}");
            }
            assert_eq!(embed_image::verified(&secrets, &message, now()), None);
            let mut tampered = message.clone();
            tampered.pop();
            tampered.push(if message.ends_with('0') { '1' } else { '0' });
            assert_eq!(state.verified(&secrets, &tampered, now()), None);
        }
    }

    #[test]
    fn embed_urls_must_be_signed_http() {
        let secrets = secrets();
        let signed = embed_image::sign(&secrets, "https://example.com/a.png");
        assert_eq!(embed_image::verified_url(&secrets, &signed, now()).as_deref(), Some("https://example.com/a.png"));
        assert_eq!(embed_image::verified_url(&secrets, &embed_image::sign(&secrets, "file:///etc/passwd"), now()), None);
        let bot_signed = app_verifier(&secrets, bot_reply::VERIFIER_NAME).generate(&Value::from("https://example.com/a.png"), None, None);
        assert_eq!(embed_image::verified_url(&secrets, &bot_signed, now()), None);
    }

    #[test]
    fn bot_reply_tokens_expire_and_are_bound_to_the_room() {
        let secrets = secrets();
        let token = bot_reply::token_for(&secrets, 3, 1, now());
        assert_eq!(bot_reply::verify(&secrets, &token, "1", now()), Some(Value::from(3)));
        assert_eq!(bot_reply::verify(&secrets, &format!(" {token}\n"), "1", now()), Some(Value::from(3)));
        assert_eq!(bot_reply::verify(&secrets, &token, "2", now()), None);
        assert_eq!(bot_reply::verify(&secrets, &token, "1", now() + bot_reply::EXPIRES_IN - SignedDuration::from_secs(1)), Some(Value::from(3)));
        assert_eq!(bot_reply::verify(&secrets, &token, "1", now() + bot_reply::EXPIRES_IN), None);
        assert_eq!(bot_reply::verify(&Secrets::new(&"b".repeat(128)), &token, "1", now()), None);
    }
}
