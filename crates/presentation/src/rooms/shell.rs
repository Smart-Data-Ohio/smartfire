// Message-list shell and uncached, per-viewer notices from our Rails room view.

#[derive(Clone, Debug, Default, serde::Deserialize, PartialEq)]
#[serde(default)]
pub struct State {
    pub unread_message_id: Option<i64>,
    pub unread_count: i64,
    pub unread_index: Option<usize>,
    pub scroll_to_divider: bool,
    pub jump_url: Option<String>,
    pub notices: Vec<Notice>,
}
#[derive(Clone, Debug, serde::Deserialize, PartialEq)]
pub struct Notice {
    pub id: i64,
    pub name: String,
    pub until_date: Option<String>,
    pub note: Option<String>,
}
