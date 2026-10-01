//! Full event pages consume preloaded facts, matching rooms/events/*.erb.
use super::CardView;
use crate::{
    ViewContext,
    helpers::{self as h, filters},
    layouts::Page,
};
use askama::Template;

#[derive(Clone, Debug, serde::Deserialize)]
pub struct VenueView {
    pub id: i64,
    pub name: String,
    pub stage: bool,
    pub member: bool,
    pub live_user: Option<String>,
}
impl VenueView {
    pub fn path(&self) -> String {
        format!("/rooms/{}", self.id)
    }
    pub fn kind(&self) -> &str {
        if self.stage {
            "stage-room"
        } else {
            "voice-room"
        }
    }
}
#[derive(Clone, Debug, serde::Deserialize)]
pub struct AttendeeView {
    pub name: String,
    pub response: String,
}
impl AttendeeView {
    pub fn label(&self) -> String {
        humanize(&self.response)
    }
}
#[derive(Clone, Debug, serde::Deserialize)]
pub struct PageEvent {
    pub card: CardView,
    pub venue: Option<VenueView>,
    pub going: i64,
    pub maybe: i64,
    pub declined: i64,
    pub recurrence_label: Option<String>,
    pub recurrence_phrase: Option<String>,
    pub recurrence_until: Option<String>,
    pub remaining: Option<i64>,
    pub manageable: bool,
    pub respondable: bool,
    pub current_response: Option<String>,
    pub previous: Option<i64>,
    pub next: Option<i64>,
    pub head: bool,
    pub description_html: Option<String>,
    pub calendar_copy: bool,
    pub attendances: Vec<AttendeeView>,
}
impl PageEvent {
    pub fn zone_label(&self) -> String {
        let z = crate::time::Zone::for_user(Some(&self.card.time_zone));
        let mut t = z.format(self.card.starts_at, "%-I:%M %p");
        if let Some(end) = self.card.ends_at {
            t.push('–');
            t.push_str(&z.format(end, "%-I:%M %p"));
        }
        format!("({t} {})", z.format(self.card.starts_at, "%Z"))
    }
    pub fn response_label(&self) -> String {
        self.current_response
            .as_deref()
            .map(humanize)
            .unwrap_or_else(|| "No response yet".into())
    }
    pub fn counts(&self, which: &str) -> String {
        match which {
            "going" => plural(self.going, "going"),
            "maybe" => plural(self.maybe, "maybe"),
            _ => plural(self.declined, "declined"),
        }
    }
    pub fn path_for(&self, id: impl std::fmt::Display) -> String {
        format!("/rooms/{}/events/{id}", self.card.room_id)
    }
    pub fn title_id(&self) -> String {
        format!("title_event_{}", self.card.id)
    }
    pub fn events_path(&self) -> String {
        format!("/rooms/{}/events", self.card.room_id)
    }
    pub fn remaining_text(&self) -> String {
        self.remaining
            .map(|n| plural(n, "occurrence"))
            .unwrap_or_default()
    }
    pub fn edit_path(&self) -> String {
        format!("{}/edit", self.card.path())
    }
    pub fn cancel_path(&self) -> String {
        format!("{}/cancel", self.card.path())
    }
    pub fn attendance_path(&self) -> String {
        format!("{}/attendance", self.card.path())
    }
    pub fn row(&self, ctx: &ViewContext) -> h::Html {
        h::raw(format!(
            "{}\n",
            Row { ctx, event: self }.render().expect("event row")
        ))
    }
    pub fn live_dot(&self, venue: &VenueView, row: bool) -> h::Html {
        let target = if row {
            format!("venue_live_dot_event_{}", self.card.id)
        } else {
            format!("event_stage_live_rooms_stage_{}", venue.id)
        };
        h::raw(format!(
            "{}\n",
            LiveDot {
                venue,
                target: &target
            }
            .render()
            .expect("event venue dot")
        ))
    }
    pub fn cancel_button(&self) -> h::Html {
        h::button_to_form(
            &self.cancel_path(),
            h::attrs()
                .attr("method", "patch")
                .class("btn btn--negative"),
            h::attrs().data(
                "turbo_confirm",
                "Cancel this event? Attendees will be notified.",
            ),
            "Cancel event",
        )
    }
    pub fn future_checkbox(&self) -> h::Html {
        h::legacy_tag(
            "input",
            h::attrs()
                .type_("checkbox")
                .name("apply_to_future")
                .id("apply_to_future")
                .value("1"),
        )
    }
    pub fn response_button(&self, name: &str, value: &str, class: &str) -> h::Html {
        h::button_tag(h::attrs().name("response").value(value).class(class), name)
    }
}
pub fn humanize(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .into_iter()
        .flat_map(char::to_uppercase)
        .chain(c)
        .collect()
}
pub fn plural(n: i64, s: &str) -> String {
    format!("{n} {s}{}", if n == 1 { "" } else { "s" })
}
#[derive(Clone, Debug, serde::Deserialize)]
pub struct IndexView {
    pub room_id: i64,
    pub room_name: String,
    pub upcoming: Vec<PageEvent>,
    pub past: Vec<PageEvent>,
    pub cancelled: Vec<PageEvent>,
}
impl IndexView {
    pub fn back_label(&self) -> String {
        format!("Back to {}", self.room_name)
    }
    pub fn path(&self) -> String {
        format!("/rooms/{}/events", self.room_id)
    }
    pub fn room_path(&self) -> String {
        format!("/rooms/{}", self.room_id)
    }
    pub fn new_path(&self) -> String {
        format!("{}/new", self.path())
    }
}
#[derive(Clone, Debug, serde::Deserialize)]
pub struct ShowView {
    pub room_name: String,
    pub event: PageEvent,
}
#[derive(Template)]
#[template(path = "rooms/events/_event.html")]
pub struct Row<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub event: &'a PageEvent,
}
#[derive(Template)]
#[template(path = "rooms/events/_venue_live_dot.html")]
pub struct LiveDot<'a> {
    pub venue: &'a VenueView,
    pub target: &'a str,
}
#[derive(Template)]
#[template(path="rooms/events/index.html",blocks=["head","content"])]
pub struct Index<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub view: &'a IndexView,
}
impl Page for Index<'_> {
    fn page_title(&self) -> Option<String> {
        Some(format!("Events in {}", self.view.room_name))
    }
    fn body_class(&self) -> Option<&str> {
        Some("sidebar work-workspace")
    }
    fn has_sidebar(&self) -> bool {
        true
    }
}
#[derive(Template)]
#[template(path="rooms/events/show.html",blocks=["head","content"])]
pub struct Show<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub view: &'a ShowView,
}
impl Page for Show<'_> {
    fn page_title(&self) -> Option<String> {
        Some(self.view.event.card.title.clone())
    }
    fn body_class(&self) -> Option<&str> {
        Some("sidebar work-workspace")
    }
    fn has_sidebar(&self) -> bool {
        true
    }
}
