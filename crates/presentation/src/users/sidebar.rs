use crate::helpers as h;
// Sidebar row data from Users::SidebarHelper. Permission facts belong to the row's viewer.
use super::UserSummary;

#[derive(Clone, Debug, Default, serde::Deserialize)]
pub struct RoomMenu {
    pub menu_categorizable: bool,
    pub menu_favorited: bool,
    pub menu_favorite_position: Option<i64>,
    pub menu_muted: bool,
    pub menu_default_involvement: String,
    pub menu_category_id: Option<i64>,
    pub menu_can_delete: bool,
    pub menu_can_leave: bool,
    pub menu_leave_url: String,
    pub menu_open_room: bool,
    pub menu_direct_room: bool,
    pub menu_room_label: Option<String>,
}

#[derive(Clone, Debug)]
pub struct SidebarDirect {
    pub room_id: i64,
    pub unread: bool,
    pub updated_at_epoch: String,
    /// Other members in preloaded association order; self if there are none.
    pub members: Vec<UserSummary>,
    pub label: String,
    pub menu: RoomMenu,
    pub viewer_administrator: bool,
    /// WS13 supplies its already rendered trusted partial and the collection-key participant IDs.
    pub huddle_participants: Option<String>,
    pub participant_ids: Option<Vec<i64>>,
    pub membership_id: i64,
    pub membership_updated_at: jiff::Timestamp,
    pub avatar_zone: crate::time::Zone,
}
#[derive(Clone, Debug)]
pub struct SidebarRoom {
    pub id: i64,
    pub param_key: String,
    pub name: String,
    pub unread: bool,
    pub menu: RoomMenu,
    pub icon: Option<h::AvatarIcon>,
    /// Rendered by WS13; None means huddles are unconfigured.
    pub huddle_participants: Option<String>,
}

/// The outer sidebar owns section layout; WS12/WS13 supply configured feature children.
#[derive(Clone, Debug)]
pub enum SidebarItem {
    Direct(Box<SidebarDirect>),
    Room(Box<SidebarRoom>),
}
#[derive(Clone, Debug)]
pub struct SidebarCategory {
    pub id: i64,
    pub name: String,
    pub collapsed: bool,
    pub rooms: Vec<SidebarRoom>,
}
impl SidebarDirect {
    pub fn class_names(&self) -> String {
        format!(
            "sidebar-item direct{}{}",
            if self.unread { " unread" } else { "" },
            if self.menu.menu_muted { " muted" } else { "" }
        )
    }
}

impl SidebarRoom {
    pub fn class_names(&self) -> String {
        format!(
            "sidebar-item room btn{}{}{}",
            match self.param_key.as_str() {
                "rooms_board" => " board-room",
                "rooms_voice" => " voice-room",
                "rooms_stage" => " voice-room stage-room",
                _ => "",
            },
            if self.unread { " unread" } else { "" },
            if self.menu.menu_muted { " muted" } else { "" }
        )
    }
}

impl SidebarCategory {
    pub fn path(&self) -> String {
        format!("/room_categories/{}", self.id)
    }
    pub fn toggle_label(&self) -> &'static str {
        if self.collapsed { "Expand" } else { "Collapse" }
    }
}
