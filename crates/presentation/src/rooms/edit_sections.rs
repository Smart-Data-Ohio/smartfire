
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
impl Repository {
    pub fn full_name(&self) -> String {
        format!("{}/{}", self.owner, self.repo)
    }
}
