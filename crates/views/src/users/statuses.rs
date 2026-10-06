//! Session-free facts for the status badge and DM OOO line.
use crate::helpers::{self as h, filters};
use askama::Template;
use serde::Deserialize;

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

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct OooNoticeMember {
    pub id: i64,
    pub name: String,
    pub visible: bool,
    pub until_date: Option<String>,
    pub note: Option<String>,
    pub stream_name: String,
}
impl OooNoticeMember {
    fn line(&self) -> h::Html {
        h::raw(
            OooNotice {
                name: &self.name,
                visible: self.visible,
                until_date: self.until_date.as_deref(),
                note: self.note.as_deref(),
            }
            .render()
            .expect("OOO line"),
        )
    }
}

#[derive(Template)]
#[template(path = "rooms/show/_ooo_notices.html")]
pub struct OooNotices<'a> {
    pub direct: bool,
    pub members: &'a [OooNoticeMember],
}

#[derive(Debug, Clone)]
pub struct ProfileStatus {
    pub user_id: i64,
    pub stream_name: String,
    pub presence: String,
    pub status_text: Option<String>,
    pub dnd_allowed: bool,
}
impl ProfileStatus {
    pub fn badge(&self) -> h::Html {
        h::raw(
            StatusBadge {
                presence: &self.presence,
                status_text: self.status_text.as_deref(),
            }
            .render()
            .expect("status badge"),
        )
    }
}

#[derive(Template)]
#[template(path = "users/statuses/_profile_status.html")]
pub struct ProfileStatusSection<'a> {
    pub status: &'a ProfileStatus,
}

#[derive(Template)]
#[template(path = "users/statuses/_allowance.html")]
pub struct DndAllowance<'a> {
    pub ctx: &'a crate::ViewContext<'a>,
    pub status: &'a ProfileStatus,
}
