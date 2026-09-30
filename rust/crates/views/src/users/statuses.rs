//! Session-free facts for the status badge and DM OOO line.
use crate::helpers as h;
use askama::Template;

#[derive(Template)]
#[template(path = "users/statuses/_badge.html")]
pub struct StatusBadge<'a> {
    pub presence: &'a str,
    pub status_text: Option<&'a str>,
}
impl StatusBadge<'_> {
    fn label(&self) -> &str {
        match self.presence {
            "online" => "Online",
            "idle" => "Idle",
            "dnd" => "Do not disturb",
            "agent" => "Agent",
            _ => "Offline",
        }
    }
    fn dot(&self) -> h::Html {
        h::builder_tag(
            "span",
            h::attrs()
                .role("img")
                .aria("label", self.label())
                .class("avatar__presence")
                .data("presence", self.presence),
        )
    }
}

#[derive(Template)]
#[template(path = "rooms/show/_ooo_notice_line.html")]
pub struct OooNotice<'a> {
    pub name: &'a str,
    pub visible: bool,
    pub until_date: Option<&'a str>,
    pub note: Option<&'a str>,
}
