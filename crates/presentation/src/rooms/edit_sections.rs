use crate::helpers as h;

use serde::Deserialize;

pub const EVENTS: [&str; 6] = [
    "opened",
    "merged",
    "closed",
    "review_requested",
    "review_submitted",
    "checks_failed",
];
pub const DEFAULT_EVENTS: [&str; 4] = ["opened", "merged", "review_requested", "checks_failed"];
#[derive(Clone, Debug, Deserialize)]
pub struct Repository {
    pub id: i64,
    pub owner: String,
    pub repo: String,
    pub events: Vec<String>,
}
#[derive(Clone, Debug, Deserialize)]
pub struct EditSections {
    pub room_id: i64,
    pub can_administer: bool,
    pub administrator: bool,
    pub repositories: Vec<Repository>,
    pub inbound_domain: Option<String>,
    pub inbound_token: Option<String>,
}
impl Repository {
    pub fn full_name(&self) -> String {
        format!("{}/{}", self.owner, self.repo)
    }
}

impl EditSections {
    pub fn events(&self) -> &'static [&'static str] {
        &EVENTS
    }
    pub fn event_label(&self, event: &str) -> String {
        let mut label = event.replace('_', " ");
        label[..1].make_ascii_uppercase();
        label
    }
    pub fn email_address(&self) -> Option<String> {
        self.inbound_domain
            .as_ref()
            .zip(self.inbound_token.as_ref().filter(|s| !h::is_blank(s)))
            .map(|(d, t)| format!("room-{t}@{d}"))
    }
}
