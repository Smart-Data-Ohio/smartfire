

pub struct Item {
    pub id: i64,
    pub status: String,
    pub created_at: String,
    pub remind_at: Option<String>,
    pub reminded_at: Option<String>,
    pub room_name: String,
    pub author_name: String,
    pub body: String,
    pub message_path: String,
}
impl Item {
    pub fn done(&self) -> bool {
        self.status == "done"
    }
    pub fn status_class(&self) -> String {
        self.status.replace('_', "-")
    }
    pub fn path(&self, filter: &str) -> String {
        format!("{}?status={filter}", campfire_routes::saved_item(self.id))
    }
}
