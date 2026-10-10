use crate::helpers as h;

#[derive(Clone, Debug)]
pub struct Chip {
    pub label: String,
    pub remove_query: String,
}
#[derive(Clone, Debug)]
pub struct SectionRow {
    pub title: String,
    pub path: String,
    pub room_label: String,
    pub time: jiff::Timestamp,
    pub status: Option<String>,
}
#[derive(Clone, Debug)]
pub struct Section {
    pub id: String,
    pub heading: String,
    pub records: Vec<SectionRow>,
}
pub fn search_path(query: &str) -> String {
    format!(
        "{}?q={}",
        campfire_routes::searches(),
        h::url::cgi_escape(query)
    )
}
