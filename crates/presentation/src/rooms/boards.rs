// Byte-for-byte Rails board rows and listings. All inputs are gathered by the app presenter.
use super::RoomView;

#[derive(Clone, Debug)]
pub struct Row {
    pub id: i64,
    pub room_id: i64,
    pub name: String,
    pub work_status: String,
    pub work_label: String,
    pub lifecycle: String,
    pub owner_id: Option<i64>,
    pub owner_label: String,
    pub agent: bool,
    pub tags: Vec<String>,
    pub replies: i64,
    pub links: i64,
    pub updated_at: jiff::Timestamp,
}

#[derive(Clone, Debug)]
pub struct Listing {
    pub room: RoomView,
    pub board_view: bool,
    pub status: String,
    pub owner: String,
    pub tag: String,
    pub current_user_id: i64,
    pub can_administer: bool,
    pub posts: Vec<Row>,
    pub owner_options: Vec<(String, String)>,
    pub tag_counts: Vec<(String, i64)>,
    pub any_posts: bool,
    pub has_more: bool,
    pub page: i64,
    pub digest: Option<(String, String)>,
    pub stream_name: String,
}
