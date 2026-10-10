
use std::collections::BTreeMap;
#[derive(Clone, Debug)]
pub struct TagRule {
    pub id: i64,
    pub tag: String,
    pub name: String,
    pub agent: bool,
}
#[derive(Clone, Debug)]
pub struct Settings {
    pub room_id: i64,
    pub room_name: String,
    pub tags: Vec<TagRule>,
    pub candidates: Vec<(String, String)>,
    pub rules: BTreeMap<String, (Option<String>, Option<String>)>,
    pub tag_error: Option<String>,
    pub sla_error: Option<String>,
}
