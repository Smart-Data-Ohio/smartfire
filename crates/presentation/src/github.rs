pub mod write_actions;
pub mod subscriptions;
pub mod connections;

#[derive(Clone, Debug, Default)]
pub struct Card {
    pub id: i64,
    pub owner: String,
    pub repo: String,
    pub number: i64,
    pub display_full_name: String,
    pub state: Option<String>,
    pub private: Option<bool>,
    pub title: Option<String>,
    pub fetch_error: Option<String>,
    pub author_login: Option<String>,
    pub author_avatar_url: Option<String>,
    pub base_branch: Option<String>,
    pub head_branch: Option<String>,
    pub review_decision: Option<String>,
    pub check_status: Option<String>,
    pub html_url: Option<String>,
    pub github_updated_at: Option<jiff::Timestamp>,
    pub files_loaded: bool,
    pub files: Vec<File>,
    pub files_total: i64,
    pub discussion_thread: Option<i64>,
}
#[derive(Clone, Debug, Default)]
pub struct File {
    pub filename: String,
    pub status: Option<String>,
    pub additions: i64,
    pub deletions: i64,
}
#[derive(Clone, Debug)]
pub struct CardMessage {
    pub id: i64,
    pub room_id: i64,
    pub thread_id: Option<i64>,
}
pub fn present(value: &Option<String>) -> Option<&str> {
    value
        .as_deref()
        .filter(|s| !s.chars().all(char::is_whitespace))
}
pub fn state_label(state: Option<&str>) -> &'static str {
    match state {
        Some("merged") => "Merged",
        Some("closed") => "Closed",
        Some("draft") => "Draft",
        _ => "Open",
    }
}
pub fn review_label(state: Option<&str>) -> Option<&'static str> {
    match state {
        Some("approved") => Some("Approved"),
        Some("changes_requested") => Some("Changes requested"),
        Some("review_required") => Some("Review required"),
        _ => None,
    }
}
pub fn checks_label(state: Option<&str>) -> &'static str {
    match state {
        Some("passing") => "Checks passing",
        Some("pending") => "Checks pending",
        Some("failing") => "Checks failing",
        _ => "No checks",
    }
}
pub fn frame_id(pr_id: i64, message_id: Option<i64>, thread_id: Option<i64>) -> String {
    match message_id {
        Some(id) => format!("card_for_message_{id}_github_pull_request_{pr_id}"),
        None => format!(
            "card_for_thread_{}_github_pull_request_{pr_id}",
            thread_id.map(|id| id.to_string()).unwrap_or_default()
        ),
    }
}
