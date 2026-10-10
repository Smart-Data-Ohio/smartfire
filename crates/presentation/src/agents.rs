use crate::users::UserSummary;
use crate::helpers as h;
pub mod history;

#[derive(Clone, Debug)]
pub struct DirectoryAgent {
    pub id: i64,
    pub user: UserSummary,
    pub icon: Option<h::AvatarIcon>,
    pub kind_description: String,
    pub status: String,
    pub status_note: Option<String>,
    pub suspended: bool,
    pub created_at: jiff::Timestamp,
    pub status_changed_at: Option<jiff::Timestamp>,
    pub last_seen_at: Option<jiff::Timestamp>,
}

/// Public bot profile facts. Management facts are present only for the owner/admin.
#[derive(Clone, Debug)]
pub struct Profile {
    pub agent: DirectoryAgent,
    pub provider_runtime: String,
    pub description: Option<String>,
    pub rooms: Vec<(i64, String)>,
    pub has_rooms: bool,
    pub hidden_room_count: usize,
    pub grants: String,
    pub management: Option<(String, String)>,
}
impl DirectoryAgent {
    pub fn status_label(&self) -> &str {
        match self.status.as_str() {
            "working" => "Working",
            "waiting" => "Waiting",
            "failed" => "Failed",
            _ => "Idle",
        }
    }
}
