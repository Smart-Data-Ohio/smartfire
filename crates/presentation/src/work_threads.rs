// Work index and human handoff form. Domain policies are gathered before rendering.
use crate::channel_threads::board::Links;

pub struct Row {
    pub id: i64,
    pub name: String,
    pub path: String,
    pub status: String,
    pub status_label: String,
    pub room_name: String,
    pub owner_label: String,
    pub agent: bool,
    pub count: i64,
    pub updated_at: jiff::Timestamp,
    pub links: Links,
}
pub struct Handoff {
    pub id: i64,
    pub room_id: i64,
    pub name: String,
    pub receivers: Vec<(String, i64)>,
    pub error: Option<String>,
}
