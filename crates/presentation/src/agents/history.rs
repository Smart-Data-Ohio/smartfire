use crate::helpers as h;

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Approval {
    pub id: i64,
    pub bot_name: String,
    pub avatar_url: String,
    pub room_name: Option<String>,
    pub action: String,
    pub summary: String,
    pub status: String,
    pub expires_at: jiff::Timestamp,
    pub decided_by: Option<String>,
    pub decision_note: Option<String>,
    pub github_login: Option<String>,
    pub fizzy_user_name: Option<String>,
    pub approvable: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct LedgerEvent {
    pub id: i64,
    pub event_type: String,
    pub outcome: Option<String>,
    pub created_at: jiff::Timestamp,
    pub room_name: Option<String>,
    pub actor_name: Option<String>,
    pub message_id: Option<i64>,
    pub hop: i64,
    pub detail: Option<String>,
    pub webhook_status: String,
    pub webhook_attempts: i64,
    pub webhook_last_error: Option<String>,
    pub external_metadata: bool,
    pub external_action: Option<String>,
    pub external_status: Option<String>,
    pub external_message: Option<String>,
    pub handoff_summary: Option<String>,
    pub content: Option<String>,
}
pub fn choices(all: &str, values: &[&str]) -> Vec<(String, String)> {
    std::iter::once((all.into(), "".into()))
        .chain(values.iter().map(|s| (s.to_string(), s.to_string())))
        .collect()
}
pub fn page_path(path: &str, key: &str, filter: Option<&str>, page: i64) -> String {
    let mut pairs = vec![("page", page.to_string())];
    if let Some(filter) = filter {
        pairs.push((key, h::url::cgi_escape(filter)));
    }
    pairs.sort_by_key(|(key, _)| *key);
    format!(
        "{path}?{}",
        pairs
            .into_iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<_>>()
            .join("&")
    )
}
