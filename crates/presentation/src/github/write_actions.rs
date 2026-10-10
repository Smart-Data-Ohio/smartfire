// Per-viewer write frame; forms receive request-local CSRF tokens through FormWith.
#[derive(Clone, Debug, Default, serde::Deserialize)]
pub struct WriteActions {
    pub thread_id: i64,
    pub room_id: i64,
    pub pull_request_id: i64,
    pub linked: bool,
    pub usable: bool,
    pub login: String,
    pub notice: Option<String>,
    pub alert: Option<String>,
    pub comment_body: Option<String>,
    pub review_body: Option<String>,
    pub reviewers_body: Option<String>,
}
