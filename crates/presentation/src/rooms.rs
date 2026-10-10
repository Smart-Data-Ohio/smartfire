pub mod header;
pub mod board_automations;
pub mod boards;
pub mod edit_sections;
pub mod navigation;
pub mod composition;
pub mod shell;
pub mod calls;

pub use header::HeaderIdentity;
use serde::Deserialize;
use crate::helpers as h;
use crate::messages::{RoomKind, UserView, room_dom_id};

/// A persisted room.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct RoomView {
    #[serde(default = "default_involvement")]
    pub involvement: String,
    pub id: i64,
    pub kind: RoomKind,
    pub name: Option<String>,
    /// `room_display_name(room)` for `Current.user`.
    pub display_name: String,
    #[serde(default)]
    pub header: Option<HeaderIdentity>,
}

/// `Membership#involvement`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct InvolvementView {
    pub room_id: i64,
    pub kind: RoomKind,
    /// "mentions", "everything", "nothing" or "invisible".
    pub involvement: String,
}

/// The room being created or edited by the open and closed room forms. `id` is `None` for a
/// new record.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct FormRoom {
    pub id: Option<i64>,
    pub name: Option<String>,
    #[serde(default)]
    pub icon_name: Option<String>,
    #[serde(default)]
    pub icon: Option<h::AvatarIcon>,
    #[serde(default)]
    pub errors: Vec<String>,
    #[serde(default)]
    pub error_attributes: Vec<String>,
    #[serde(default)]
    pub inbound_email: Option<InboundEmailView>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct InboundEmailView {
    pub id: i64,
    pub emailable: bool,
    pub enabled: bool,
    pub address: Option<String>,
}

/// `rooms/opens/{new,edit}`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct OpenFormView {
    pub room: FormRoom,
    /// `Current.user.can_administer?(room)`: administrators, the creator, or a new room.
    pub can_administer: bool,
    /// `User.active.ordered`.
    pub users: Vec<UserView>,
}

/// `rooms/closeds/{new,edit}`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ClosedFormView {
    pub room: FormRoom,
    pub can_administer: bool,
    pub current_user_id: i64,
    /// Active users with access (none for a new room).
    pub selected_users: Vec<UserView>,
    /// The other active users.
    pub unselected_users: Vec<UserView>,
}
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct DirectPickerUser {
    pub user: UserView,
    pub bot: bool,
    pub agent: bool,
    pub starred: bool,
}

/// `rooms/directs/edit`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct DirectEditView {
    pub room_id: i64,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub group_capable: bool,
    #[serde(default)]
    pub administrator: bool,
    #[serde(default)]
    pub candidates: Vec<UserView>,
    #[serde(default)]
    pub error_attributes: Vec<String>,
    /// `room_display_name(@room)` for `Current.user`.
    pub display_name: String,
    /// `@room.users.many? ? @room.users.without(Current.user) : @room.users`.
    pub users: Vec<UserView>,
}
/// Trusted output supplied by WS8b-m (and WS13/WS17 for configured header/OOO children).
/// These fragments are page inputs, never a shared fragment cache or broadcast payload.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct ShellComponents {
    #[serde(default)]
    pub pins_count: i64,
    #[serde(default)]
    pub thread_panel_name: Option<String>,
    pub pins_panel: String,
    pub thread_panel: String,
    pub huddle_header: String,
    pub ooo_notices: String,
    pub poll_builder: String,
    pub message_template: Option<String>,
    pub composer: Option<String>,
    pub message_list: Option<String>,
}

/// `Rooms::Direct#direct_display_name`: named rooms keep their name; unnamed
/// groups preview three first names. The caller supplies ordered other members.
pub fn room_display_name(name: Option<&str>, direct: bool, other_member_names: &[String], for_user_name: Option<&str>) -> String {
    if direct {
        if let Some(name) = h::presence(name) { return name.to_owned(); }
        match other_member_names {
            [] => for_user_name.unwrap_or_default().to_owned(),
            [name] => name.clone(),
            names => {
                let firsts = names.iter().take(3).map(|name| {
                    name.split([' ', '\t', '\n', '\r', '\x0b', '\x0c'])
                        .find(|s| !s.is_empty()).unwrap_or_default()
                }).collect::<Vec<_>>().join(", ");
                let remaining = names.len().saturating_sub(3);
                if remaining > 0 { format!("{firsts} +{remaining}") } else { firsts }
            }
        }
    } else {
        name.unwrap_or_default().to_string()
    }
}

/// `mention_prompt_tag(room)`'s `src`: `autocompletable_users_path(room_id: room.id)`.
pub fn mention_prompt_src(room_id: i64) -> String {
    format!(
        "{}?room_id={room_id}",
        campfire_routes::autocompletable_users()
    )
}

pub fn default_involvement() -> String {
    "mentions".into()
}
impl RoomView {
    pub fn is_board(&self) -> bool { self.kind == RoomKind::Board }
    pub fn is_stage(&self) -> bool {
        self.header
            .as_ref()
            .is_some_and(|h| h.param_key == "rooms_stage")
    }
    pub fn dom_id(&self, prefix: &str) -> String {
        self.header.as_ref().map_or_else(
            || room_dom_id(self.kind, self.id, prefix),
            |header| format!("{prefix}_{}_{}", header.param_key, self.id),
        )
    }

    pub fn is_direct(&self) -> bool {
        self.kind.is_direct()
    }

    /// `edit_polymorphic_path(room)`: `/rooms/opens/1/edit` and so on.
    pub fn edit_path(&self) -> String {
        if self.is_board() {
            return format!("/rooms/boards/{}/edit", self.id);
        }
        match self.kind {
            RoomKind::Open => campfire_routes::edit_rooms_open(self.id),
            RoomKind::Closed => campfire_routes::edit_rooms_closed(self.id),
            RoomKind::Direct => campfire_routes::edit_rooms_direct(self.id),
            RoomKind::Voice => campfire_routes::edit_rooms_voice(self.id),
            RoomKind::Stage => campfire_routes::edit_rooms_stage(self.id),
            RoomKind::Board => campfire_routes::edit_rooms_board(self.id),
        }
    }

    /// "Ping" for direct rooms, "room" otherwise.
    pub fn noun(&self) -> &'static str {
        if self.is_direct() { "Ping" } else { "room" }
    }
}

impl FormRoom {
    /// `form_with model: room`'s action for an open or closed room.
    pub fn action(&self, kind: RoomKind) -> String {
        match (self.id, kind) {
            (Some(id), RoomKind::Board) => campfire_routes::rooms_board(id),
            (None, RoomKind::Board) => campfire_routes::rooms_boards(),
            (Some(id), RoomKind::Open) => campfire_routes::rooms_open(id),
            (Some(id), _) => campfire_routes::rooms_closed(id),
            (None, RoomKind::Open) => campfire_routes::rooms_opens(),
            (None, _) => campfire_routes::rooms_closeds(),
        }
    }

    pub fn display_name(&self) -> &str {
        self.name.as_deref().unwrap_or_default()
    }
}
