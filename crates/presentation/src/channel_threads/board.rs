

#[derive(Clone, Debug, Default)]
pub struct NewPost {
    pub room_id: i64,
    pub room_name: String,
    pub name: Option<String>,
    pub first_message: Option<String>,
    pub status: String,
    pub owner_id: Option<i64>,
    pub tags: Vec<String>,
    pub tag_suggestions: Vec<String>,
    pub humans: Vec<(String, i64)>,
    pub agents: Vec<(String, i64)>,
    pub error: Option<String>,
    pub field_errors: Vec<String>,
}

pub struct History {
    pub kind: String,
    pub actor: String,
    pub from_status: String,
    pub to_status: String,
    pub status_changed: bool,
    pub owner_changed: bool,
    pub from_owner: String,
    pub to_owner: String,
    pub note: Option<String>,
    pub summary: Option<String>,
    pub links: i64,
    pub questions: i64,
}

pub struct Link {
    pub id: i64,
    pub kind: String,
    pub label: String,
    pub url: String,
    pub remove_label: String,
    pub state: Option<String>,
    pub title: Option<String>,
    pub event_time: Option<jiff::Timestamp>,
    pub event_zone: Option<String>,
    pub cancelled: bool,
}
pub struct Links {
    pub items: Vec<Link>,
    pub events: Vec<(String, i64)>,
}
