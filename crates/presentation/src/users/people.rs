// Plain display models for the people directory and profile-card popover.
use super::UserSummary;

#[derive(Clone, Debug)]
pub struct Person {
    pub user: UserSummary,
    pub online: bool,
    pub starred: bool,
    pub agent: bool,
    pub agent_owner: Option<String>,
    pub presence: String,
    pub custom_status: Option<String>,
    pub mention: Option<String>,
}
impl Person {
    pub fn label(&self) -> &str {
        match self.presence.as_str() {
            "online" => "Online",
            "idle" => "Idle",
            "dnd" => "Do not disturb",
            "agent" => "Agent",
            _ => "Offline",
        }
    }
}
