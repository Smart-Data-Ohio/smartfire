use crate::helpers as h;

#[derive(Clone, Debug, serde::Deserialize)]
pub struct Item {
    pub id: i64,
    pub state: String,
    pub event_label: String,
    pub created_at: Option<jiff::Timestamp>,
    pub approval: Option<crate::agents::history::Approval>,
    pub title: String,
    pub author: Option<String>,
    pub body: String,
}
pub const DESTINATIONS: [(&str, &str, &str); 5] = [
    ("/work", "check.svg", "Work threads"),
    ("/saved", "bookmark.svg", "Saved"),
    ("/scheduled_messages", "calendar.svg", "Scheduled"),
    ("/agents", "bot.svg", "Agents"),
    ("/users", "everyone.svg", "People"),
];
pub const TYPES: [(&str, &str); 9] = [
    ("all", "All"),
    ("mentions", "Mentions and replies"),
    ("threads", "Threads and work"),
    ("events", "Events"),
    ("agents", "Agents"),
    ("github", "GitHub"),
    ("huddles", "Huddles"),
    ("reminders", "Reminders"),
    ("security", "Security"),
];
pub fn path(filter: &str, kind: &str, before: Option<i64>) -> String {
    use h::url::{Param, with_query};
    let mut params = vec![
        ("status", Param::One(filter.into())),
        ("type", Param::One(kind.into())),
    ];
    if let Some(before) = before {
        params.push(("before", Param::One(before.to_string())));
    }
    with_query("/activity", params)
}
