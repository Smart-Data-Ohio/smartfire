
#[derive(Clone, Debug, serde::Deserialize, PartialEq)]
pub struct Card {
    pub author: String,
    pub room_label: String,
    pub excerpt: String,
    pub created_at: jiff::Timestamp,
    pub message_path: String,
}
/// WS8bm2 root integration: only same-room sources carry inline facts. Cross-room
/// references stay lazy and the existing frame endpoint performs viewer authorization.
#[derive(Clone, Debug, serde::Deserialize, PartialEq)]
pub struct Reference {
    pub id: i64,
    pub card: Option<Card>,
}

/// Origin placeholder used only by detached fixture renders.
pub const ORIGIN_SLOT: &str = "http://campfire.test";
