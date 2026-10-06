//! `app/models/github/app.rb` and the state check in `github/app_connections_controller.rb`.
use jiff::Timestamp;
use rails_compat::{Secrets, verifiers::OAuthState};
use serde_json::Value;
use subtle::ConstantTimeEq;

pub fn valid_state(
    secrets: &Secrets,
    signed: &str,
    stored: Option<&Value>,
    now: Timestamp,
) -> bool {
    let Some(Value::String(verified)) = OAuthState::GithubAppOAuth.verified(secrets, signed, now)
    else {
        return false;
    };
    let Some(Value::String(stored)) = stored else {
        return false;
    };
    verified.len() == stored.len() && bool::from(verified.as_bytes().ct_eq(stored.as_bytes()))
}

pub fn sign_state(secrets: &Secrets, raw: &str) -> String {
    OAuthState::GithubAppOAuth.generate(secrets, raw)
}
