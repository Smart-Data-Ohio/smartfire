// Full event pages consume preloaded facts, matching rooms/events/*.erb.
use super::CardView;

#[derive(Clone, Debug, serde::Deserialize)]
pub struct VenueView {
    pub id: i64,
    pub name: String,
    pub stage: bool,
    pub member: bool,
    pub live_user: Option<String>,
}
#[derive(Clone, Debug, serde::Deserialize)]
pub struct AttendeeView {
    pub name: String,
    pub response: String,
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
#[derive(Clone, Debug, serde::Deserialize)]
pub struct IndexView {
    pub room_id: i64,
    pub room_name: String,
    pub upcoming: Vec<PageEvent>,
    pub past: Vec<PageEvent>,
    pub cancelled: Vec<PageEvent>,
}
#[derive(Clone, Debug, serde::Deserialize)]
pub struct ShowView {
    pub room_name: String,
    pub event: PageEvent,
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

impl AttendeeView {
    pub fn label(&self) -> String {
        humanize(&self.response)
    }
}

impl PageEvent {
    pub fn zone_label(&self) -> String {
        let z = crate::time::Zone::for_user(Some(&self.card.time_zone));
        let mut t = self.card.starts_at.format(&z, "%-I:%M %p");
        if let Some(end) = self.card.ends_at {
            t.push('–');
            t.push_str(&end.format(&z, "%-I:%M %p"));
        }
        format!("({t} {})", self.card.starts_at.format(&z, "%Z"))
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
