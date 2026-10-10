//! The boot JSON: what the SPA needs before its first request, inlined into the shell (and served
//! by `GET /api/v1/boot` with the CSRF token).

use serde::Serialize;

/// Boot JSON v0. Field names are the TypeScript ones (camelCase).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Boot {
    pub user: BootUser,
    pub account: BootAccount,
    /// Workspace CSS, also rendered as the classic layout's custom style element.
    pub custom_styles: Option<String>,
    /// `<html data-theme>`: "light", "dark" or "system" ([`theme`]).
    pub theme: &'static str,
    /// `<html data-text-size>`: "smaller" to "larger" ([`text_size`]).
    pub text_size: &'static str,
    /// The Action Cable endpoint, as the layout's `action-cable-url` meta tag gives it.
    pub cable_url: String,
    /// The effective UI's worker. Classic users visiting the SPA register no worker.
    pub service_worker_url: Option<String>,
    /// `X-Version` (`APP_VERSION`, falling back to `GIT_REVISION`).
    pub version: String,
    /// `X-Rev` (`GIT_REVISION`), when set.
    pub revision: Option<String>,
    /// Pending classic feedback, consumed only by a shell navigation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flash: Option<BootFlash>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootFlash {
    pub kind: FlashKind,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FlashKind {
    Notice,
    Alert,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootUser {
    pub id: i64,
    pub name: String,
    /// `fresh_user_avatar_path`: changes whenever the user does, so it can be cached.
    pub avatar_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootAccount {
    /// `None` only before first run.
    pub name: Option<String>,
    pub logo_url: Option<String>,
    pub logo_still_url: Option<String>,
    pub banner_url: Option<String>,
    pub banner_still_url: Option<String>,
}

/// `GET /api/v1/boot`: the boot JSON with the masked CSRF token the shell's meta tag carries.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootResponse<'a> {
    #[serde(flatten)]
    pub boot: &'a Boot,
    pub csrf_token: &'a str,
}

/// `Users::PresenceHelper#user_theme`: the stored theme if it's one of `THEMES`, else "system".
pub fn theme(stored: Option<&str>) -> &'static str {
    match stored {
        Some("light") => "light",
        Some("dark") => "dark",
        _ => "system",
    }
}

/// `Users::PresenceHelper#user_text_size`: the stored size if it's one of `TEXT_SIZES`, else
/// "default".
pub fn text_size(stored: Option<&str>) -> &'static str {
    match stored {
        Some("smaller") => "smaller",
        Some("small") => "small",
        Some("large") => "large",
        Some("larger") => "larger",
        _ => "default",
    }
}

/// `value` as JSON that's safe inside a `<script>` element: no `<`, `>` or `&` (so no
/// `</script>` or `<!--`), and no U+2028/U+2029, which end a line in older JavaScript parsers.
/// Each only occurs inside JSON strings, where its `\u` escape means the same.
pub fn script_json(value: &impl Serialize) -> String {
    let json = serde_json::to_string(value).expect("boot JSON always serializes");
    let mut out = String::with_capacity(json.len());
    for c in json.chars() {
        match c {
            '<' => out.push_str("\\u003c"),
            '>' => out.push_str("\\u003e"),
            '&' => out.push_str("\\u0026"),
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
            c => out.push(c),
        }
    }
    out
}
