// `app/views/rooms/github_subscriptions/_section.html.erb`.
#[derive(Clone, Debug)]
pub struct Subscription {
    pub id: i64,
    pub full_name: String,
    pub events: Vec<String>,
}
#[derive(Clone, Debug, Default)]
pub struct Section {
    pub room_id: i64,
    pub can_administer: bool,
    pub administrator: bool,
    pub subscriptions: Vec<Subscription>,
}
pub const EVENTS: [(&str, &str, bool); 6] = [
    ("opened", "Opened", true),
    ("merged", "Merged", true),
    ("closed", "Closed", false),
    ("review_requested", "Review requested", true),
    ("review_submitted", "Review submitted", false),
    ("checks_failed", "Checks failed", true),
];
