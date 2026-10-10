use crate::helpers as h;

pub struct Item {
    pub id: i64,
    pub room_name: String,
    pub thread_name: Option<String>,
    pub body: String,
    pub send_at: String,
    pub send_value: String,
    pub sent_at: Option<String>,
    pub message_path: Option<String>,
}
impl Item {
    pub fn excerpt(&self) -> String {
        h::truncate(&self.body, 500, "...")
    }
}
