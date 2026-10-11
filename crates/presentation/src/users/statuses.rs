// Session-free facts for the status badge and DM OOO line.

#[derive(Debug, Clone)]
pub struct ProfileStatus {
    pub user_id: i64,
    pub presence: String,
    pub status_text: Option<String>,
    pub dnd_allowed: bool,
}
