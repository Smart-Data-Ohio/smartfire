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
