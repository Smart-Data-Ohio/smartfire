
use serde_json::Value;
pub struct FormView {
    pub back_path: String,
    pub action: String,
    pub room_name: String,
    pub plain: String,
    pub creator: String,
    pub connected: bool,
    pub boards: Value,
    pub board_id: String,
    pub title: String,
    pub description: String,
    pub user_name: String,
    pub account_name: String,
}
impl FormView {
    pub fn excerpt(&self) -> String {
        campfire_richtext::ruby::truncate(&self.plain, 280, "...")
    }
}
