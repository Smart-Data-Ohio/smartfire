// Session-free facts for the status badge and DM OOO line.
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct OooNoticeMember {
    pub id: i64,
    pub name: String,
    pub visible: bool,
    pub until_date: Option<String>,
    pub note: Option<String>,
    pub stream_name: String,
}

#[derive(Debug, Clone)]
pub struct ProfileStatus {
    pub user_id: i64,
    pub stream_name: String,
    pub presence: String,
    pub status_text: Option<String>,
    pub dnd_allowed: bool,
}
