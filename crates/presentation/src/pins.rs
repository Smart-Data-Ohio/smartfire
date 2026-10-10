
use jiff::Timestamp;

#[derive(Clone, Debug, serde::Deserialize, PartialEq)]
pub struct Pin {
    pub message_id: i64,
    pub pinner_name: String,
    pub author_name: String,
    pub excerpt: String,
    pub created_at: Timestamp,
    pub message_path: String,
}
#[derive(Clone, Debug, serde::Deserialize, PartialEq)]
pub struct List {
    pub room_id: i64,
    pub room_param_key: String,
    pub pins: Vec<Pin>,
}
pub struct Badge {
    pub client_message_id: String,
    pub details: BadgeState,
}
pub struct BadgeState {
    pub pinned: bool,
}
impl List {
    pub fn dom_id(&self, prefix: &str) -> String {
        format!("{prefix}_{}_{}", self.room_param_key, self.room_id)
    }
}

impl Badge {
    pub fn new(client_message_id: String, pinned: bool) -> Self {
        Self {
            client_message_id,
            details: BadgeState { pinned },
        }
    }
    pub fn dom_id(&self, prefix: &str) -> String {
        format!("{prefix}_message_{}", self.client_message_id)
    }
}
