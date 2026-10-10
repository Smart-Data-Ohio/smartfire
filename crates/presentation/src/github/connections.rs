// Owned GitHub sections embedded in the WS8b-r2 profile and WS11-ui bot page.
// Public display data only; request-local tokens come from the shared form helpers.
use crate::helpers as h;
#[derive(Clone, Debug, Default, serde::Deserialize)]
#[serde(default)]
pub struct Connection {
    pub linked: bool,
    pub usable: bool,
    pub login: String,
    pub reason: Option<String>,
    pub app_token: bool,
    pub app_configured: bool,
}
pub fn reason(data: &Connection) -> String {
    data.reason
        .as_deref()
        .filter(|s| !s.chars().all(char::is_whitespace))
        .map(|s| format!(" ({})", h::escape(s)))
        .unwrap_or_default()
}
