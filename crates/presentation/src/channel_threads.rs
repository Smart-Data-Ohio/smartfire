pub mod board;

pub struct ListRow {
    pub id: i64,
    pub name: String,
    pub status: String,
    pub message_count: i64,
    pub work_label: Option<String>,
    pub owner_label: String,
    pub agent: bool,
}
pub struct Work {
    pub id: i64,
    pub status_label: String,
    pub owner_label: String,
    pub owner_agent: bool,
    pub can_manage: bool,
    pub history: Vec<board::History>,
    pub links: board::Links,
    pub steps: Vec<crate::messages::parts::AgentStep>,
}
